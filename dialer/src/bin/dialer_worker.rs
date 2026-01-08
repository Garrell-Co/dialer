use dialer::app;
use dialer::app::WorkerConfig;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let cfg = WorkerConfig::from_env_and_args()?;

    init_tracing(&cfg)?;

    tracing::info!(
        "Starting worker '{}' at log_level={}",
        cfg.worker_name,
        cfg.log_level
    );

    app::run_worker(cfg).await
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
