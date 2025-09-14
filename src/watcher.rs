use crate::certificate::CertificateManager;
use crate::config::WatcherConfig;
use crate::error::Result;
use crate::redis_client::RedisClient;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct CertificateWatcher {
    cert_manager: CertificateManager,
    redis: RedisClient,
    config: WatcherConfig,
}

impl CertificateWatcher {
    pub fn new(
        cert_manager: CertificateManager,
        redis: RedisClient,
        config: WatcherConfig,
    ) -> Self {
        Self {
            cert_manager,
            redis,
            config,
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting certificate watcher with {} second intervals",
            self.config.check_interval_seconds
        );

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
            self.config.check_interval_seconds,
        ));

        // Semaphore to limit concurrent renewals
        let renewal_semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_renewals));

        loop {
            interval.tick().await;

            if let Err(e) = self
                .check_and_renew_certificates(renewal_semaphore.clone())
                .await
            {
                error!("Error in certificate watcher: {}", e);
            }
        }
    }

    async fn check_and_renew_certificates(&self, renewal_semaphore: Arc<Semaphore>) -> Result<()> {
        // Get certificates that are expiring soon
        let expiring_certs = self.cert_manager.get_expiring_certificates().await?;

        if expiring_certs.is_empty() {
            info!("No certificates need renewal");
            return Ok(());
        }

        info!(
            "Found {} certificates that need renewal",
            expiring_certs.len()
        );

        // Create tasks for concurrent renewal processing
        let mut renewal_tasks = Vec::new();

        for cert_record in expiring_certs {
            let cert_manager = self.cert_manager.clone();
            let redis = self.redis.clone();
            let renewal_semaphore = renewal_semaphore.clone();
            let cert_id = cert_record.certificate_id.clone();

            let task = tokio::spawn(async move {
                let _permit = renewal_semaphore.acquire().await.unwrap();

                info!("Renewing certificate: {}", cert_id);

                match cert_manager.renew_certificate(&cert_id, None).await {
                    Ok(new_cert) => {
                        info!(
                            "Successfully renewed certificate: {} -> {}",
                            cert_id, new_cert.certificate_id
                        );

                        // Publish renewal event
                        if let Err(e) = redis
                            .publish_event("auto_renewed", &new_cert.certificate_id)
                            .await
                        {
                            warn!("Failed to publish renewal event: {}", e);
                        }

                        Ok(new_cert.certificate_id)
                    }
                    Err(e) => {
                        error!("Failed to renew certificate {}: {}", cert_id, e);

                        // Publish error event
                        let error_msg = format!("{}:{}", cert_id, e);
                        if let Err(e) = redis.publish_event("renewal_failed", &error_msg).await {
                            warn!("Failed to publish renewal error event: {}", e);
                        }

                        Err(e)
                    }
                }
            });

            renewal_tasks.push(task);
        }

        // Wait for all renewal tasks to complete
        let mut successful_renewals = 0;
        let mut failed_renewals = 0;

        for task in renewal_tasks {
            match task.await {
                Ok(Ok(_)) => successful_renewals += 1,
                Ok(Err(e)) => {
                    error!("Certificate renewal failed: {}", e);
                    failed_renewals += 1;
                }
                Err(e) => {
                    error!("Renewal task panicked: {}", e);
                    failed_renewals += 1;
                }
            }
        }

        info!(
            "Certificate renewal batch completed: {} successful, {} failed",
            successful_renewals, failed_renewals
        );

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn check_certificate_health(&self) -> Result<()> {
        let all_certs = self.cert_manager.list_certificates(None).await?;

        let mut active_count = 0;
        let mut expired_count = 0;
        let mut revoked_count = 0;

        for cert in all_certs {
            match cert.status.as_str() {
                "active" => active_count += 1,
                "expired" => expired_count += 1,
                "revoked" => revoked_count += 1,
                _ => {}
            }
        }

        info!(
            "Certificate health check: {} active, {} expired, {} revoked",
            active_count, expired_count, revoked_count
        );

        // Publish health metrics
        let health_data = format!(
            "active:{},expired:{},revoked:{}",
            active_count, expired_count, revoked_count
        );
        self.redis
            .publish_event("health_check", &health_data)
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn cleanup_expired_certificates(&self, days_old: u32) -> Result<()> {
        let cutoff_time = chrono::Utc::now().timestamp() - (days_old as i64 * 24 * 60 * 60);
        let all_certs = self.cert_manager.list_certificates(Some("expired")).await?;

        let mut cleaned_count = 0;

        for cert in all_certs {
            if cert.expires_at < cutoff_time {
                // Delete certificate files and Redis record
                if let Err(e) = self.redis.delete_certificate(&cert.certificate_id).await {
                    warn!(
                        "Failed to delete expired certificate {}: {}",
                        cert.certificate_id, e
                    );
                } else {
                    cleaned_count += 1;
                    info!("Cleaned up expired certificate: {}", cert.certificate_id);
                }
            }
        }

        if cleaned_count > 0 {
            info!(
                "Cleaned up {} expired certificates older than {} days",
                cleaned_count, days_old
            );
            self.redis
                .publish_event("cleanup", &format!("removed:{}", cleaned_count))
                .await?;
        }

        Ok(())
    }
}
