use std::sync::Arc;

use dialer::api::{self, AppState};
use dialer::app::WorkerConfig;
use dialer::controller;
use dialer::freeswitch::types::FsEventKind;
use dialer::freeswitch::{EslEventFormat, EslSupervisorConfig, FreeswitchTelephonyAdapter};
use dialer::policy::manual_phone::ManualPhonePolicy;
use dialer::store::memory::MemoryCallStore;
use dialer::telephony::TelephonyPort;

use tokio::sync::{broadcast, mpsc};
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

    // Build telephony adapter
    let supervisor_config = EslSupervisorConfig {
        host: cfg.freeswitch_host.clone(),
        port: cfg.freeswitch_port,
        password: cfg.freeswitch_password.clone(),
        event_format: EslEventFormat::Json,
        event_list: vec![
            FsEventKind::CHANNEL_CREATE,
            FsEventKind::CHANNEL_STATE,
            FsEventKind::CHANNEL_DESTROY,
            FsEventKind::CHANNEL_ANSWER,
            FsEventKind::CHANNEL_HANGUP,
            FsEventKind::CHANNEL_HANGUP_COMPLETE,
            FsEventKind::CHANNEL_PROGRESS,
            FsEventKind::CHANNEL_PROGRESS_MEDIA,
            FsEventKind::CHANNEL_PARK,
            FsEventKind::CHANNEL_UNPARK,
            FsEventKind::CHANNEL_ORIGINATE,
            FsEventKind::CHANNEL_OUTGOING,
            FsEventKind::CHANNEL_BRIDGE,
            FsEventKind::CHANNEL_UNBRIDGE,
            FsEventKind::CHANNEL_HOLD,
            FsEventKind::CHANNEL_UNHOLD,
            FsEventKind::CHANNEL_EXECUTE,
            FsEventKind::CHANNEL_EXECUTE_COMPLETE,
            FsEventKind::CHANNEL_APPLICATION,
            FsEventKind::CHANNEL_DATA,
            FsEventKind::CHANNEL_UUID,
            FsEventKind::CHANNEL_CALLSTATE,
        ],
    };

    let mut telephony = FreeswitchTelephonyAdapter::connect(supervisor_config).await?;
    let event_rx = telephony.take_event_rx();

    // Build components
    let store: Arc<dyn dialer::store::CallStore> = Arc::new(MemoryCallStore::new());
    let policy = ManualPhonePolicy;
    let (command_tx, command_rx) = mpsc::channel(100);
    let (state_tx, _) = broadcast::channel(100);

    // Build API
    let app_state = AppState {
        command_tx,
        state_tx: state_tx.clone(),
        store: store.clone(),
    };
    let router = api::build_router(app_state);

    let api_port = std::env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001u16);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", api_port)).await?;
    tracing::info!("API server listening on 0.0.0.0:{}", api_port);

    // Spawn API server
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("API server error: {}", e);
        }
    });

    // Run controller loop (blocks until shutdown)
    controller::run_controller(&telephony, store.as_ref(), &policy, command_rx, event_rx, state_tx)
        .await
}

fn init_tracing(cfg: &WorkerConfig) -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&cfg.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}
