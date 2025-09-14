mod certificate;
mod config;
mod error;
mod grpc;
mod redis_client;
mod watcher;

use anyhow::Result;
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use grpc::CertAgentService;
use watcher::CertificateWatcher;

#[derive(Parser)]
#[command(name = "cert-agent")]
#[command(about = "mTLS Certificate Management Service")]
struct Args {
    #[arg(short, long, default_value = "config/default.toml")]
    config: String,

    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| args.log_level.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting cert-agent service...");

    // Load configuration
    let config = Config::load(&args.config)?;
    info!("Configuration loaded from: {}", args.config);

    // Initialize Redis client
    let redis_client = redis_client::RedisClient::new(&config.redis.url).await?;
    info!("Connected to Redis at: {}", config.redis.url);

    // Initialize certificate manager
    let cert_manager =
        certificate::CertificateManager::new(&config.certificate, redis_client.clone()).await?;

    // Start certificate watcher
    let watcher = CertificateWatcher::new(
        cert_manager.clone(),
        redis_client.clone(),
        config.watcher.clone(),
    );
    let watcher_handle = tokio::spawn(async move {
        if let Err(e) = watcher.start().await {
            error!("Certificate watcher error: {}", e);
        }
    });

    // Initialize gRPC service
    let grpc_service = CertAgentService::new(cert_manager, redis_client);

    // Start gRPC server
    let bind_address = config.grpc.bind_address.clone();
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = grpc_service.start(bind_address.clone()).await {
            error!("gRPC server error: {}", e);
        }
    });

    info!("Cert-agent service started successfully");
    info!("gRPC server listening on: {}", config.grpc.bind_address);
    info!(
        "Certificate watcher running with {} second intervals",
        config.watcher.check_interval_seconds
    );

    // Wait for either service to exit
    tokio::select! {
        result = watcher_handle => {
            error!("Certificate watcher exited: {:?}", result);
        }
        result = grpc_handle => {
            error!("gRPC server exited: {:?}", result);
        }
    }

    Ok(())
}
