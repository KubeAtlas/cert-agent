use crate::certificate::{CertificateManager, CertificateRequest};
use crate::redis_client::RedisClient;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

pub mod cert_agent {
    tonic::include_proto!("cert_agent");
}

use cert_agent::{
    cert_agent_server::{CertAgent, CertAgentServer},
    *,
};

#[derive(Debug)]
pub struct CertAgentService {
    cert_manager: CertificateManager,
    redis: RedisClient,
}

impl CertAgentService {
    pub fn new(cert_manager: CertificateManager, redis: RedisClient) -> Self {
        Self {
            cert_manager,
            redis,
        }
    }

    pub async fn start(&self, bind_address: String) -> crate::error::Result<()> {
        let addr = bind_address.parse().map_err(|e| {
            crate::error::CertAgentError::InvalidRequest(format!("Invalid bind address: {}", e))
        })?;

        let service = CertAgentServer::new(self.clone());

        info!("Starting gRPC server on {}", addr);

        tonic::transport::Server::builder()
            .add_service(service)
            .serve(addr)
            .await
            .map_err(|e| {
                crate::error::CertAgentError::Internal(format!("gRPC server error: {}", e))
            })?;

        Ok(())
    }
}

#[tonic::async_trait]
impl CertAgent for CertAgentService {
    async fn issue_certificate(
        &self,
        request: Request<IssueCertificateRequest>,
    ) -> std::result::Result<Response<IssueCertificateResponse>, Status> {
        let req = request.into_inner();

        info!("Issuing certificate for CN: {}", req.common_name);

        let cert_request = CertificateRequest {
            common_name: req.common_name,
            dns_names: req.dns_names,
            ip_addresses: req.ip_addresses,
            validity_days: req.validity_days as u32,
            organization: Some(req.organization),
            organizational_unit: Some(req.organizational_unit),
            country: Some(req.country),
            state: Some(req.state),
            locality: Some(req.locality),
            metadata: req.metadata,
        };

        match self.cert_manager.issue_certificate(cert_request).await {
            Ok(cert) => {
                let response = IssueCertificateResponse {
                    certificate_id: cert.certificate_id,
                    certificate_pem: cert.certificate_pem,
                    private_key_pem: cert.private_key_pem,
                    ca_certificate_pem: cert.ca_certificate_pem,
                    expires_at: cert.expires_at.timestamp(),
                    status: cert_status_to_proto(&cert.status),
                };

                info!(
                    "Successfully issued certificate: {}",
                    response.certificate_id
                );
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to issue certificate: {}", e);
                Err(Status::internal(format!(
                    "Failed to issue certificate: {}",
                    e
                )))
            }
        }
    }

    async fn renew_certificate(
        &self,
        request: Request<RenewCertificateRequest>,
    ) -> std::result::Result<Response<RenewCertificateResponse>, Status> {
        let req = request.into_inner();

        info!("Renewing certificate: {}", req.certificate_id);

        let validity_days = if req.validity_days > 0 {
            Some(req.validity_days as u32)
        } else {
            None
        };

        match self
            .cert_manager
            .renew_certificate(&req.certificate_id, validity_days)
            .await
        {
            Ok(cert) => {
                let response = RenewCertificateResponse {
                    certificate_id: cert.certificate_id,
                    certificate_pem: cert.certificate_pem,
                    private_key_pem: cert.private_key_pem,
                    expires_at: cert.expires_at.timestamp(),
                    status: cert_status_to_proto(&cert.status),
                };

                info!(
                    "Successfully renewed certificate: {}",
                    response.certificate_id
                );
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to renew certificate {}: {}", req.certificate_id, e);
                Err(Status::internal(format!(
                    "Failed to renew certificate: {}",
                    e
                )))
            }
        }
    }

    async fn revoke_certificate(
        &self,
        request: Request<RevokeCertificateRequest>,
    ) -> std::result::Result<Response<RevokeCertificateResponse>, Status> {
        let req = request.into_inner();

        info!("Revoking certificate: {}", req.certificate_id);

        match self
            .cert_manager
            .revoke_certificate(&req.certificate_id, Some(&req.reason))
            .await
        {
            Ok(()) => {
                let response = RevokeCertificateResponse {
                    certificate_id: req.certificate_id,
                    success: true,
                    message: "Certificate revoked successfully".to_string(),
                };

                info!(
                    "Successfully revoked certificate: {}",
                    response.certificate_id
                );
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to revoke certificate {}: {}", req.certificate_id, e);
                let response = RevokeCertificateResponse {
                    certificate_id: req.certificate_id,
                    success: false,
                    message: format!("Failed to revoke certificate: {}", e),
                };
                Ok(Response::new(response))
            }
        }
    }

    async fn get_certificate_status(
        &self,
        request: Request<GetCertificateStatusRequest>,
    ) -> std::result::Result<Response<GetCertificateStatusResponse>, Status> {
        let req = request.into_inner();

        match self
            .cert_manager
            .get_certificate_status(&req.certificate_id)
            .await
        {
            Ok(Some(cert_record)) => {
                let response = GetCertificateStatusResponse {
                    certificate_id: cert_record.certificate_id,
                    status: cert_status_to_proto(&cert_record.status),
                    expires_at: cert_record.expires_at,
                    issued_at: cert_record.issued_at,
                    common_name: cert_record.common_name,
                    dns_names: cert_record.dns_names,
                    metadata: cert_record.metadata,
                };
                Ok(Response::new(response))
            }
            Ok(None) => {
                warn!("Certificate not found: {}", req.certificate_id);
                Err(Status::not_found(format!(
                    "Certificate not found: {}",
                    req.certificate_id
                )))
            }
            Err(e) => {
                error!(
                    "Failed to get certificate status {}: {}",
                    req.certificate_id, e
                );
                Err(Status::internal(format!(
                    "Failed to get certificate status: {}",
                    e
                )))
            }
        }
    }

    async fn list_certificates(
        &self,
        request: Request<ListCertificatesRequest>,
    ) -> std::result::Result<Response<ListCertificatesResponse>, Status> {
        let req = request.into_inner();

        let status_filter = if req.status == cert_status_to_proto("") {
            None
        } else {
            Some(proto_to_cert_status(&req.status))
        };

        match self
            .cert_manager
            .list_certificates(status_filter.as_deref())
            .await
        {
            Ok(certificates) => {
                let cert_infos: Vec<CertificateInfo> = certificates
                    .into_iter()
                    .map(|cert| CertificateInfo {
                        certificate_id: cert.certificate_id,
                        common_name: cert.common_name,
                        dns_names: cert.dns_names,
                        status: cert_status_to_proto(&cert.status),
                        expires_at: cert.expires_at,
                        issued_at: cert.issued_at,
                        metadata: cert.metadata,
                    })
                    .collect();

                let response = ListCertificatesResponse {
                    certificates: cert_infos,
                    next_page_token: String::new(), // TODO: Implement pagination
                };

                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to list certificates: {}", e);
                Err(Status::internal(format!(
                    "Failed to list certificates: {}",
                    e
                )))
            }
        }
    }

    type WatchCertificatesStream = ReceiverStream<std::result::Result<CertificateEvent, Status>>;

    async fn watch_certificates(
        &self,
        request: Request<WatchCertificatesRequest>,
    ) -> std::result::Result<Response<Self::WatchCertificatesStream>, Status> {
        let req = request.into_inner();

        info!(
            "Starting certificate watch for {} certificates",
            req.certificate_ids.len()
        );

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let cert_manager = self.cert_manager.clone();
        let _redis = self.redis.clone();
        let certificate_ids = req.certificate_ids;
        let check_interval = req.check_interval_seconds;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(check_interval as u64));

            loop {
                interval.tick().await;

                // Get expiring certificates
                match cert_manager.get_expiring_certificates().await {
                    Ok(expiring_certs) => {
                        for cert in expiring_certs {
                            // Check if we should watch this certificate
                            if certificate_ids.is_empty()
                                || certificate_ids.contains(&cert.certificate_id)
                            {
                                let event = CertificateEvent {
                                    certificate_id: cert.certificate_id.clone(),
                                    event_type: CertificateEventType::Expiring as i32,
                                    message: format!(
                                        "Certificate expires in {} days",
                                        (cert.expires_at - chrono::Utc::now().timestamp())
                                            / (24 * 60 * 60)
                                    ),
                                    timestamp: chrono::Utc::now().timestamp(),
                                };

                                if tx.send(Ok(event)).await.is_err() {
                                    return; // Client disconnected
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to check expiring certificates: {}", e);
                        let error_event = CertificateEvent {
                            certificate_id: String::new(),
                            event_type: CertificateEventType::Unspecified as i32,
                            message: format!("Error checking certificates: {}", e),
                            timestamp: chrono::Utc::now().timestamp(),
                        };

                        if tx.send(Ok(error_event)).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

// Helper functions for status conversion
fn cert_status_to_proto(status: &str) -> i32 {
    match status {
        "active" => CertificateStatus::Active as i32,
        "expired" => CertificateStatus::Expired as i32,
        "revoked" => CertificateStatus::Revoked as i32,
        "pending" => CertificateStatus::Pending as i32,
        _ => CertificateStatus::Unspecified as i32,
    }
}

fn proto_to_cert_status(status: &i32) -> String {
    match *status {
        x if x == CertificateStatus::Active as i32 => "active".to_string(),
        x if x == CertificateStatus::Expired as i32 => "expired".to_string(),
        x if x == CertificateStatus::Revoked as i32 => "revoked".to_string(),
        x if x == CertificateStatus::Pending as i32 => "pending".to_string(),
        _ => "unspecified".to_string(),
    }
}

// Implement Clone for the service
impl Clone for CertAgentService {
    fn clone(&self) -> Self {
        Self {
            cert_manager: self.cert_manager.clone(),
            redis: self.redis.clone(),
        }
    }
}
