use thiserror::Error;

#[derive(Error, Debug)]
pub enum CertAgentError {
    #[error("Certificate error: {0}")]
    Certificate(String),
    
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("OpenSSL error: {0}")]
    OpenSsl(#[from] openssl::error::ErrorStack),
    
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    
    #[error("Invalid certificate ID: {0}")]
    #[allow(dead_code)]
    InvalidCertificateId(String),
    
    #[error("Certificate not found: {0}")]
    CertificateNotFound(String),
    
    #[error("Certificate expired: {0}")]
    #[allow(dead_code)]
    CertificateExpired(String),
    
    #[error("Certificate already exists: {0}")]
    #[allow(dead_code)]
    CertificateAlreadyExists(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, CertAgentError>;
