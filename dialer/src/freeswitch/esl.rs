use anyhow::{Context, Result, anyhow};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, mpsc, oneshot};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;
use tracing;

use super::parser::EslParser;

pub type Headers = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct EslEvent {
    pub frame_headers: Headers,
    pub event_headers: Headers,
    pub event_body: Option<Vec<u8>>,
}

#[derive(Clone)]
pub enum EslEventFormat {
    Json,
    Plain,
    Xml
}

impl fmt::Display for EslEventFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EslEventFormat::Json => "json",
            EslEventFormat::Plain => "plain",
            EslEventFormat::Xml => "xml",
        };
        write!(f, "{}", s)
    }
}

pub enum EslCommand {
    /// "api <cmd>\n\n" with oneshot reply fulfilled by reader loop
    Api {
        cmd: String,
        reply: oneshot::Sender<Result<EslEvent>>,
    },
    /// raw lines that must end with \n\n
    SendRaw { lines: String },
    Close,
}

/// Cheap cloneable handle to the ESL supervisor
/// Just contains a channel sender - no sockets or heavy state
#[derive(Clone)]
pub struct EslHandle {
    cmd_tx: mpsc::Sender<EslCommand>,
}

impl EslHandle {
    /// Create a new handle from a command channel sender
    /// This is typically called by EslSupervisor
    pub(crate) fn new(cmd_tx: mpsc::Sender<EslCommand>) -> Self {
        Self { cmd_tx }
    }

    /// Send an API command and wait for the response
    pub async fn api(&self, cmd: String) -> Result<EslEvent> {
        let (tx, rx) = oneshot::channel();

        self.cmd_tx
            .send(EslCommand::Api { cmd, reply: tx })
            .await
            .map_err(|_| anyhow!("ESL command channel closed"))?;

        rx.await.map_err(|_| anyhow!("ESL reply channel dropped"))?
    }

    /// Send raw lines to the ESL connection
    pub async fn send_raw(&self, lines: String) -> Result<()> {
        self.cmd_tx
            .send(EslCommand::SendRaw { lines })
            .await
            .map_err(|_| anyhow!("ESL command channel closed"))
    }
}

/// Supervisor configuration for establishing ESL connections
pub struct EslSupervisorConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub event_format: EslEventFormat,
}

pub struct EslSupervisor;

impl EslSupervisor {
    pub fn spawn(
        config: EslSupervisorConfig,
    ) -> (EslHandle, mpsc::Receiver<EslEvent>) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<EslCommand>(100);
        let (event_tx, event_rx) = mpsc::channel::<EslEvent>(100);
        
        let handle = EslHandle::new(cmd_tx);
        
        tokio::spawn(supervisor_task(config, cmd_rx, event_tx));
        
        (handle, event_rx)
    }
}

async fn supervisor_task(
    config: EslSupervisorConfig,
    mut cmd_rx: mpsc::Receiver<EslCommand>,
    event_tx: mpsc::Sender<EslEvent>,
) {
    loop {
        match establish_connection(&config).await {
            Ok((fs_reader, fs_writer)) => {
                if let Err(e) = run_connection(fs_reader, fs_writer, &mut cmd_rx, &event_tx).await {
                    tracing::error!("Connection error: {}, reconnecting...", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect: {}, retrying...", e);
            }
        }
        
        // Wait before reconnecting
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

/// Establish a TCP connection and authenticate
async fn establish_connection(
    config: &EslSupervisorConfig,
) -> Result<(OwnedReadHalf, OwnedWriteHalf)> {
    tracing::debug!(
        host = %config.host,
        port = config.port,
        "Connecting to FreeSWITCH ESL"
    );

    let fs_conn_str = format!("{}:{}", config.host, config.port);
    let fs_stream = TcpStream::connect(fs_conn_str).await
        .context("Failed to connect to FreeSWITCH")?;
    
    tracing::debug!("Connected to FreeSWITCH");
    
    let (fs_reader, fs_writer) = fs_stream.into_split();
    
    // Authenticate
    let auth_cmd = format!("auth {}\n\n", config.password);
    let mut writer = fs_writer;
    writer.write_all(auth_cmd.as_bytes()).await
        .context("Failed to send auth command")?;

    tracing::debug!("Authenticated to FreeSWITCH");
    
    // Subscribe to events
    let event_cmd = format!("event {} ALL\n\n", config.event_format);
    writer.write_all(event_cmd.as_bytes()).await
        .context("Failed to send event subscription")?;

    tracing::debug!("Subscribed to FreeSWITCH events");
    
    Ok((fs_reader, writer))
}


async fn run_connection(
    fs_reader: OwnedReadHalf,
    fs_writer: OwnedWriteHalf,
    cmd_rx: &mut mpsc::Receiver<EslCommand>,
    event_tx: &mpsc::Sender<EslEvent>,
) -> Result<()> {
    let pending_api = Arc::new(Mutex::new(VecDeque::<oneshot::Sender<Result<EslEvent>>>::new()));
    
    // Spawn reader task
    let pending_api_reader = pending_api.clone();
    let event_tx_clone = event_tx.clone();
    let reader_handle = tokio::spawn(read_frames_and_forward(
        fs_reader,
        event_tx_clone,
        pending_api_reader,
    ));
    
    // Handle commands in the main task
    let pending_api_writer = pending_api.clone();
    let writer_result = send_commands_to_fs(
        fs_writer,
        cmd_rx,
        pending_api_writer,
    ).await;
    
    reader_handle.abort();
    
    writer_result
}


async fn read_frames_and_forward(
    fs_reader: OwnedReadHalf,
    fs_event_tx: mpsc::Sender<EslEvent>,
    pending_api: Arc<Mutex<VecDeque<oneshot::Sender<Result<EslEvent>>>>>,
) -> Result<()> {
    let mut parser = EslParser::new(fs_reader);

    loop {
        let event = match parser.parse_frame().await {
            Ok(Some(e)) => e,
            Ok(None) => break,
            Err(e) => {
                tracing::error!("Failed to parse event: {}", e);
                break
            }
        };

        let content_type = event.frame_headers.get("Content-Type")
            .cloned()
            .unwrap_or_default();

        match content_type.as_str() {
            "text/event-plain" | "text/event-json" | "text/event-xml" => {
                if fs_event_tx.send(event).await.is_err() {
                    tracing::error!("event dropped during notification");
                }
            }
            "command/reply" | "api/response" => {
                let mut q = pending_api.lock().await;
                if let Some(reply_tx) = q.pop_front() {
                    let _ = reply_tx.send(Ok(event));
                }
            }
            _ => {
                // ignore other message types
            }
        }
    }

    let mut q = pending_api.lock().await;
    while let Some(tx) = q.pop_front() {
        let _ = tx.send(Err(anyhow!("ESL connection closed")));
    }

    Ok(())
}


async fn send_commands_to_fs(
    mut fs_writer: OwnedWriteHalf,
    command_rx: &mut mpsc::Receiver<EslCommand>,
    pending_api: Arc<Mutex<VecDeque<oneshot::Sender<Result<EslEvent>>>>>,
) -> Result<()> {
    loop {
        if let Some(cmd) = command_rx.recv().await {
            match cmd {
                EslCommand::Api { cmd, reply } => {
                    pending_api.lock().await.push_back(reply);
                    let wire = format!("api {}\n\n", cmd);
                    fs_writer.write_all(wire.as_bytes()).await?;
                }
                EslCommand::SendRaw { lines } => {
                    fs_writer.write_all(lines.as_bytes()).await?;
                }
                EslCommand::Close => break,
            }
        } else {
            break;
        }
    }

    Ok(())
}


