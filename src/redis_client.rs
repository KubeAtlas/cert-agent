use crate::error::{CertAgentError, Result};
use redis::{Client, AsyncCommands};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
// use std::time::Duration; // Not used currently

#[derive(Debug, Clone)]
pub struct RedisClient {
    client: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateRecord {
    pub certificate_id: String,
    pub common_name: String,
    pub dns_names: Vec<String>,
    pub ip_addresses: Vec<String>,
    pub status: String,
    pub expires_at: i64,
    pub issued_at: i64,
    pub metadata: std::collections::HashMap<String, String>,
}

impl RedisClient {
    pub async fn new(url: &str) -> Result<Self> {
        let client = Client::open(url)
            .map_err(|e| CertAgentError::Redis(e))?;
        
        // Test connection
        let mut conn = client.get_connection_manager().await
            .map_err(|e| CertAgentError::Redis(e))?;
        
        redis::cmd("PING").exec_async(&mut conn).await
            .map_err(|e| CertAgentError::Redis(e))?;
        
        Ok(Self { client })
    }
    
    pub async fn get_connection(&self) -> Result<ConnectionManager> {
        self.client.get_connection_manager().await
            .map_err(|e| CertAgentError::Redis(e))
    }
    
    // Certificate operations
    pub async fn store_certificate(&self, cert_record: &CertificateRecord) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("cert:{}", cert_record.certificate_id);
        let value = serde_json::to_string(cert_record)?;
        
        conn.set_ex(&key, value, 365 * 24 * 60 * 60).await
            .map_err(|e| CertAgentError::Redis(e))?;
        
        // Add to index for listing
        let _: () = conn.sadd("certs:all", &cert_record.certificate_id).await
            .map_err(|e| CertAgentError::Redis(e))?;
        
        Ok(())
    }
    
    pub async fn get_certificate(&self, certificate_id: &str) -> Result<Option<CertificateRecord>> {
        let mut conn = self.get_connection().await?;
        let key = format!("cert:{}", certificate_id);
        
        let value: Option<String> = conn.get(&key).await
            .map_err(|e| CertAgentError::Redis(e))?;
        
        match value {
            Some(v) => {
                let cert_record: CertificateRecord = serde_json::from_str(&v)?;
                Ok(Some(cert_record))
            }
            None => Ok(None),
        }
    }
    
    pub async fn update_certificate_status(&self, certificate_id: &str, status: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("cert:{}", certificate_id);
        
        // Get current record
        let value: Option<String> = conn.get(&key).await
            .map_err(|e| CertAgentError::Redis(e))?;
        
        if let Some(v) = value {
            let mut cert_record: CertificateRecord = serde_json::from_str(&v)?;
            cert_record.status = status.to_string();
            let updated_value = serde_json::to_string(&cert_record)?;
            
            conn.set(&key, updated_value).await
                .map_err(|e| CertAgentError::Redis(e))?;
        }
        
        Ok(())
    }
    
    pub async fn list_certificates(&self, status_filter: Option<&str>) -> Result<Vec<CertificateRecord>> {
        let mut conn = self.get_connection().await?;
        let certificate_ids: Vec<String> = conn.smembers("certs:all").await
            .map_err(|e| CertAgentError::Redis(e))?;
        
        let mut certificates = Vec::new();
        
        for cert_id in certificate_ids {
            let key = format!("cert:{}", cert_id);
            let value: Option<String> = conn.get(&key).await
                .map_err(|e| CertAgentError::Redis(e))?;
            
            if let Some(v) = value {
                let cert_record: CertificateRecord = serde_json::from_str(&v)?;
                
                if let Some(status) = status_filter {
                    if cert_record.status == status {
                        certificates.push(cert_record);
                    }
                } else {
                    certificates.push(cert_record);
                }
            }
        }
        
        Ok(certificates)
    }
    
    pub async fn get_expiring_certificates(&self, threshold_days: u32) -> Result<Vec<CertificateRecord>> {
        let all_certs = self.list_certificates(Some("active")).await?;
        let threshold_seconds = (threshold_days as i64) * 24 * 60 * 60;
        let current_time = chrono::Utc::now().timestamp();
        
        let expiring_certs = all_certs
            .into_iter()
            .filter(|cert| {
                let time_until_expiry = cert.expires_at - current_time;
                time_until_expiry > 0 && time_until_expiry <= threshold_seconds
            })
            .collect();
        
        Ok(expiring_certs)
    }
    
    #[allow(dead_code)]
    pub async fn delete_certificate(&self, certificate_id: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("cert:{}", certificate_id);
        
        // Remove from main storage
        let _: () = conn.del(&key).await
            .map_err(|e| CertAgentError::Redis(e))?;
        
        // Remove from index
        let _: () = conn.srem("certs:all", certificate_id).await
            .map_err(|e| CertAgentError::Redis(e))?;
        
        Ok(())
    }
    
    // Pub/Sub for real-time notifications
    pub async fn publish_event(&self, event: &str, data: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.publish("cert_events", format!("{}:{}", event, data)).await
            .map_err(|e| CertAgentError::Redis(e))?;
        Ok(())
    }
}
