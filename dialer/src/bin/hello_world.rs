use dialer::app;
use dialer::app::WorkerConfig;
use dialer::telephony::{OriginateRequest, DestinationType};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let cfg = WorkerConfig::from_env_and_args("loopback_test")?;

    init_tracing(&cfg)?;

    tracing::info!(
        "Starting loopback test worker '{}' at log_level={}",
        cfg.worker_name,
        cfg.log_level
    );

    // Configure a simple loopback call for testing
    let originate_req = OriginateRequest {
        id: "loopback-test-uuid".to_string(),
        from: "loopback_test".to_string(),
        caller_id_name: Some("Loopback Test".to_string()),
        destination: DestinationType::Loopback {
            extension: "9196".to_string(),
            context: "default".to_string(),
        },
        application: Some("echo()".to_string()),
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

