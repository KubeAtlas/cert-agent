use crate::config::CertificateConfig;
use crate::error::{CertAgentError, Result};
use crate::redis_client::{CertificateRecord, RedisClient};
use chrono::{DateTime, Utc};
use openssl::{
    asn1::Asn1Time,
    bn::BigNum,
    hash::MessageDigest,
    pkey::{PKey, Private},
    rsa::Rsa,
    x509::{X509Name, X509},
};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CertificateManager {
    config: CertificateConfig,
    redis: RedisClient,
    ca_cert: Option<X509>,
    ca_key: Option<PKey<Private>>,
}

#[derive(Debug, Clone)]
pub struct CertificateRequest {
    pub common_name: String,
    pub dns_names: Vec<String>,
    pub ip_addresses: Vec<String>,
    pub validity_days: u32,
    pub organization: Option<String>,
    pub organizational_unit: Option<String>,
    pub country: Option<String>,
    pub state: Option<String>,
    pub locality: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct IssuedCertificate {
    pub certificate_id: String,
    pub certificate_pem: String,
    pub private_key_pem: String,
    pub ca_certificate_pem: String,
    pub expires_at: DateTime<Utc>,
    pub status: String,
}

impl CertificateManager {
    pub async fn new(config: &CertificateConfig, redis: RedisClient) -> Result<Self> {
        let mut manager = Self {
            config: config.clone(),
            redis,
            ca_cert: None,
            ca_key: None,
        };

        // Load CA certificate and key
        manager.load_ca_credentials().await?;

        // Ensure storage directory exists
        fs::create_dir_all(&config.storage_path).await?;

        Ok(manager)
    }

    async fn load_ca_credentials(&mut self) -> Result<()> {
        // Try to load existing CA certificate and key
        if Path::new(&self.config.ca_cert_path).exists()
            && Path::new(&self.config.ca_key_path).exists()
        {
            let cert_pem = fs::read_to_string(&self.config.ca_cert_path).await?;
            self.ca_cert = Some(X509::from_pem(cert_pem.as_bytes())?);

            let key_pem = fs::read_to_string(&self.config.ca_key_path).await?;
            self.ca_key = Some(PKey::private_key_from_pem(key_pem.as_bytes())?);
        } else {
            // Generate new CA certificate and key
            self.generate_ca_certificate().await?;
        }

        Ok(())
    }

    async fn generate_ca_certificate(&mut self) -> Result<()> {
        // Generate CA private key
        let rsa = Rsa::generate(self.config.key_size)?;
        let ca_key = PKey::from_rsa(rsa)?;

        // Create CA certificate
        let mut name = X509Name::builder()?;
        name.append_entry_by_text("CN", "Cert Agent CA")?;
        name.append_entry_by_text("O", "Cert Agent")?;
        name.append_entry_by_text("C", "US")?;
        let name = name.build();

        let mut cert_builder = X509::builder()?;
        cert_builder.set_version(2)?;
        cert_builder.set_subject_name(&name)?;
        cert_builder.set_issuer_name(&name)?;

        // Set serial number
        let serial = BigNum::from_u32(1)?;
        let serial_int = serial.to_asn1_integer()?;
        cert_builder.set_serial_number(&serial_int)?;

        // Set validity period (10 years for CA)
        let not_before = Asn1Time::days_from_now(0)?;
        let not_after = Asn1Time::days_from_now(3650)?; // 10 years
        cert_builder.set_not_before(&not_before)?;
        cert_builder.set_not_after(&not_after)?;

        // Add CA extensions
        cert_builder.append_extension(
            openssl::x509::extension::BasicConstraints::new()
                .ca()
                .pathlen(0)
                .build()?,
        )?;

        cert_builder.append_extension(
            openssl::x509::extension::KeyUsage::new()
                .key_cert_sign()
                .crl_sign()
                .build()?,
        )?;

        // Set public key and sign
        cert_builder.set_pubkey(&ca_key)?;
        cert_builder.sign(&ca_key, MessageDigest::sha256())?;

        let ca_cert = cert_builder.build();

        // Save CA certificate and key
        fs::create_dir_all(Path::new(&self.config.ca_cert_path).parent().unwrap()).await?;
        fs::write(&self.config.ca_cert_path, ca_cert.to_pem()?).await?;
        fs::write(&self.config.ca_key_path, ca_key.private_key_to_pem_pkcs8()?).await?;

        self.ca_cert = Some(ca_cert);
        self.ca_key = Some(ca_key);

        Ok(())
    }

    pub async fn issue_certificate(
        &self,
        request: CertificateRequest,
    ) -> Result<IssuedCertificate> {
        let certificate_id = Uuid::new_v4().to_string();

        // Generate private key for the certificate
        let rsa = Rsa::generate(self.config.key_size)?;
        let private_key = PKey::from_rsa(rsa)?;

        // Create certificate request
        let mut name = X509Name::builder()?;
        name.append_entry_by_text("CN", &request.common_name)?;

        if let Some(ref org) = request.organization {
            name.append_entry_by_text("O", org)?;
        }
        if let Some(ref ou) = request.organizational_unit {
            name.append_entry_by_text("OU", ou)?;
        }
        if let Some(ref country) = request.country {
            name.append_entry_by_text("C", country)?;
        }
        if let Some(ref state) = request.state {
            name.append_entry_by_text("ST", state)?;
        }
        if let Some(ref locality) = request.locality {
            name.append_entry_by_text("L", locality)?;
        }
        let name = name.build();

        // Create certificate
        let mut cert_builder = X509::builder()?;
        cert_builder.set_version(2)?;
        cert_builder.set_subject_name(&name)?;
        cert_builder.set_issuer_name(self.ca_cert.as_ref().unwrap().subject_name())?;

        // Set serial number
        let serial = BigNum::from_u32(uuid::Uuid::new_v4().as_fields().0)?;
        let serial_int = serial.to_asn1_integer()?;
        cert_builder.set_serial_number(&serial_int)?;

        // Set validity period
        let not_before = Asn1Time::days_from_now(0)?;
        let not_after = Asn1Time::days_from_now(request.validity_days)?;
        cert_builder.set_not_before(&not_before)?;
        cert_builder.set_not_after(&not_after)?;

        // Add SAN extensions
        {
            let mut san = openssl::x509::extension::SubjectAlternativeName::new();
            for dns_name in &request.dns_names {
                san.dns(dns_name);
            }
            for ip_addr in &request.ip_addresses {
                san.ip(ip_addr);
            }

            // Create X509v3 context for SAN extension
            let ctx = cert_builder.x509v3_context(None, None);
            cert_builder.append_extension(san.build(&ctx)?)?;
        }

        // Add key usage and extended key usage
        cert_builder.append_extension(
            openssl::x509::extension::KeyUsage::new()
                .digital_signature()
                .key_encipherment()
                .build()?,
        )?;

        cert_builder.append_extension(
            openssl::x509::extension::ExtendedKeyUsage::new()
                .server_auth()
                .client_auth()
                .build()?,
        )?;

        // Set public key and sign
        cert_builder.set_pubkey(&private_key)?;
        cert_builder.sign(self.ca_key.as_ref().unwrap(), MessageDigest::sha256())?;

        let certificate = cert_builder.build();

        // Store certificate files
        let cert_path = format!("{}/{}.crt", self.config.storage_path, certificate_id);
        let key_path = format!("{}/{}.key", self.config.storage_path, certificate_id);

        fs::write(&cert_path, certificate.to_pem()?).await?;
        fs::write(&key_path, private_key.private_key_to_pem_pkcs8()?).await?;

        // Create certificate record for Redis
        let expires_at = Utc::now() + chrono::Duration::days(request.validity_days as i64);
        let cert_record = CertificateRecord {
            certificate_id: certificate_id.clone(),
            common_name: request.common_name,
            dns_names: request.dns_names,
            ip_addresses: request.ip_addresses,
            status: "active".to_string(),
            expires_at: expires_at.timestamp(),
            issued_at: Utc::now().timestamp(),
            metadata: request.metadata,
        };

        // Store in Redis
        self.redis.store_certificate(&cert_record).await?;

        // Publish event
        self.redis.publish_event("issued", &certificate_id).await?;

        Ok(IssuedCertificate {
            certificate_id,
            certificate_pem: String::from_utf8(certificate.to_pem()?)?,
            private_key_pem: String::from_utf8(private_key.private_key_to_pem_pkcs8()?)?,
            ca_certificate_pem: String::from_utf8(self.ca_cert.as_ref().unwrap().to_pem()?)?,
            expires_at,
            status: "active".to_string(),
        })
    }

    pub async fn renew_certificate(
        &self,
        certificate_id: &str,
        validity_days: Option<u32>,
    ) -> Result<IssuedCertificate> {
        // Get existing certificate record
        let cert_record = self
            .redis
            .get_certificate(certificate_id)
            .await?
            .ok_or_else(|| CertAgentError::CertificateNotFound(certificate_id.to_string()))?;

        if cert_record.status != "active" {
            return Err(CertAgentError::Certificate(format!(
                "Cannot renew certificate with status: {}",
                cert_record.status
            )));
        }

        // Create renewal request
        let renewal_request = CertificateRequest {
            common_name: cert_record.common_name,
            dns_names: cert_record.dns_names,
            ip_addresses: cert_record.ip_addresses,
            validity_days: validity_days.unwrap_or(self.config.default_validity_days),
            organization: None,
            organizational_unit: None,
            country: None,
            state: None,
            locality: None,
            metadata: cert_record.metadata,
        };

        // Issue new certificate
        let new_cert = self.issue_certificate(renewal_request).await?;

        // Mark old certificate as revoked
        self.redis
            .update_certificate_status(certificate_id, "revoked")
            .await?;
        self.redis.publish_event("revoked", certificate_id).await?;

        // Publish renewal event
        self.redis
            .publish_event("renewed", &new_cert.certificate_id)
            .await?;

        Ok(new_cert)
    }

    pub async fn revoke_certificate(
        &self,
        certificate_id: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        // Update status in Redis
        self.redis
            .update_certificate_status(certificate_id, "revoked")
            .await?;

        // Publish event
        let event_data = if let Some(reason) = reason {
            format!("{}:{}", certificate_id, reason)
        } else {
            certificate_id.to_string()
        };
        self.redis.publish_event("revoked", &event_data).await?;

        Ok(())
    }

    pub async fn get_certificate_status(
        &self,
        certificate_id: &str,
    ) -> Result<Option<CertificateRecord>> {
        self.redis.get_certificate(certificate_id).await
    }

    pub async fn list_certificates(
        &self,
        status_filter: Option<&str>,
    ) -> Result<Vec<CertificateRecord>> {
        self.redis.list_certificates(status_filter).await
    }

    pub async fn get_expiring_certificates(&self) -> Result<Vec<CertificateRecord>> {
        self.redis
            .get_expiring_certificates(self.config.renewal_threshold_days)
            .await
    }
}
