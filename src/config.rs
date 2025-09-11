use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub grpc: GrpcConfig,
    pub redis: RedisConfig,
    pub certificate: CertificateConfig,
    pub watcher: WatcherConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    pub bind_address: String,
    pub max_message_size: usize,
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_file: String,
    pub key_file: String,
    pub ca_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout_secs: u64,
    pub command_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateConfig {
    pub ca_cert_path: String,
    pub ca_key_path: String,
    pub storage_path: String,
    pub default_validity_days: u32,
    pub renewal_threshold_days: u32,
    pub key_size: u32,
    pub signature_algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    pub check_interval_seconds: u64,
    pub renewal_threshold_days: u32,
    pub max_concurrent_renewals: usize,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let settings = if path.as_ref().exists() {
            config::Config::builder()
                .add_source(config::File::from(path.as_ref()))
                .add_source(config::Environment::with_prefix("CERT_AGENT").separator("_"))
                .build()?
        } else {
            config::Config::builder()
                .add_source(config::Environment::with_prefix("CERT_AGENT").separator("_"))
                .build()?
        };
        
        let config: Config = settings.try_deserialize()?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grpc: GrpcConfig {
                bind_address: "0.0.0.0:50051".to_string(),
                max_message_size: 4 * 1024 * 1024, // 4MB
                tls: None,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                max_connections: 10,
                connection_timeout_secs: 5,
                command_timeout_secs: 3,
            },
            certificate: CertificateConfig {
                ca_cert_path: "./certs/ca.crt".to_string(),
                ca_key_path: "./certs/ca.key".to_string(),
                storage_path: "./certs/storage".to_string(),
                default_validity_days: 365,
                renewal_threshold_days: 30,
                key_size: 2048,
                signature_algorithm: "sha256".to_string(),
            },
            watcher: WatcherConfig {
                check_interval_seconds: 3600, // 1 hour
                renewal_threshold_days: 30,
                max_concurrent_renewals: 10,
            },
        }
    }
}
