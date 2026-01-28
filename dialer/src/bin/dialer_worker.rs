use dialer::app;
use dialer::app::WorkerConfig;
use dialer::telephony::{OriginateRequest, DestinationType};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let cfg = WorkerConfig::from_env_and_args("dialer_worker")?;

    init_tracing(&cfg)?;

    tracing::info!(
        "Starting dialer worker '{}' at log_level={}",
        cfg.worker_name,
        cfg.log_level
    );

    // Get local IP address dynamically
    let local_ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to get local IP: {}, using localhost", e);
            "127.0.0.1".to_string()
        });
    
    tracing::info!("Using local IP: {}", local_ip);

    // Configure call to registered linphone extension
    let originate_req = OriginateRequest {
        id: "8d47cccc-1320-445e-b9ab-23db31c8b35f".to_string(),
        from: "1000".to_string(),
        caller_id_name: Some("Extension 1000".to_string()),
        destination: DestinationType::RegisteredUser {
            user: "1001".to_string(),
            domain: Some(local_ip),
        },
        application: Some("playback(local_stream://moh)".to_string()),
    };

    app::run_worker(cfg, originate_req).await
}

fn init_tracing(cfg: &WorkerConfig) -> anyhow::Result<()> {
    // Parse log level from config, defaulting to INFO if invalid
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&cfg.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(true) // Show module paths (e.g., dialer::app, dialer::telephony)
        .with_file(true) // Show file names
        .with_line_number(true) // Show line numbers
        .init();

    Ok(())
}
