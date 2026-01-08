use anyhow::{anyhow, Context, Result};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::reader::EslReader;
use crate::freeswitch::types::FsEventKind;
use crate::telephony::TelephonyEvent;

pub type Headers = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct EslEvent {
    pub frame_headers: Headers,
    pub event_headers: Headers,
    pub event_body: Option<Vec<u8>>,
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum EslEventFormat {
    Json,
    Plain,
    Xml,
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

#[allow(dead_code)]
pub enum EslCommand {
    /// "api <cmd>\n\n" with oneshot reply fulfilled by reader loop
    Api {
        cmd: String,
        reply: oneshot::Sender<Result<EslEvent>>,
    },
    /// raw lines that must end with \n\n
    SendRaw {
        lines: String,
    },
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

    #[allow(dead_code)]
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
    pub event_list: Vec<FsEventKind>,
}

pub struct EslSupervisor;

impl EslSupervisor {
    pub fn spawn(
        config: EslSupervisorConfig,
    ) -> (
        EslHandle,
        mpsc::Receiver<EslEvent>,
        mpsc::Receiver<TelephonyEvent>,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<EslCommand>(100);
        let (event_tx, event_rx) = mpsc::channel::<EslEvent>(100);
        let (connection_state_tx, connection_state_rx) = mpsc::channel::<TelephonyEvent>(10);

        let handle = EslHandle::new(cmd_tx);

        tokio::spawn(supervisor_task(
            config,
            cmd_rx,
            event_tx,
            connection_state_tx,
        ));

        (handle, event_rx, connection_state_rx)
    }
}

async fn supervisor_task(
    config: EslSupervisorConfig,
    mut cmd_rx: mpsc::Receiver<EslCommand>,
    event_tx: mpsc::Sender<EslEvent>,
    connection_state_tx: mpsc::Sender<TelephonyEvent>,
) {
    loop {
        match establish_connection(&config).await {
            Ok((fs_reader, fs_writer)) => {
                // Emit TransportUp when connection is established
                let _ = connection_state_tx.send(TelephonyEvent::TransportUp).await;
                tracing::info!("FreeSWITCH connection established, transport is up");

                if let Err(e) = run_connection(fs_reader, fs_writer, &mut cmd_rx, &event_tx).await {
                    tracing::error!("Connection error: {}, reconnecting...", e);
                    // Emit TransportDown when connection is lost
                    let _ = connection_state_tx
                        .send(TelephonyEvent::TransportDown)
                        .await;
                    tracing::warn!("FreeSWITCH connection lost, transport is down");
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect: {}, retrying...", e);
                // Emit TransportDown on connection failure
                let _ = connection_state_tx
                    .send(TelephonyEvent::TransportDown)
                    .await;
            }
        }

        // Wait before reconnecting
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

/// Establish a TCP connection and authenticate
async fn establish_connection(
    config: &EslSupervisorConfig,
) -> Result<(EslReader<OwnedReadHalf>, OwnedWriteHalf)> {
    tracing::debug!(
        host = %config.host,
        port = config.port,
        "Connecting to FreeSWITCH ESL"
    );

    let fs_conn_str = format!("{}:{}", config.host, config.port);
    let fs_stream = TcpStream::connect(fs_conn_str)
        .await
        .context("Failed to connect to FreeSWITCH")?;

    tracing::debug!("Connected to FreeSWITCH");

    let (fs_reader, fs_writer) = fs_stream.into_split();

    // Create an EslReader to parse responses
    let mut reader = EslReader::new(fs_reader, config.event_format.clone());

    // Expect auth request
    match reader.read_next_event().await {
        Ok(_) => {
            tracing::debug!("Received auth req");
        }
        Err(_) => return Err(anyhow!("No auth request received")),
    };

    // Authenticate
    let auth_cmd = format!("auth {}\n\n", config.password);
    let mut writer = fs_writer;
    writer
        .write_all(auth_cmd.as_bytes())
        .await
        .context("Failed to send auth command")?;

    // Read and parse auth response
    let auth_response = reader
        .read_next_event()
        .await
        .context("Failed to read auth response")?
        .ok_or_else(|| anyhow!("Unexpected EOF while reading auth response"))?;

    // Check if auth was successful
    let reply_text = auth_response
        .frame_headers
        .get("Reply-Text")
        .or_else(|| auth_response.event_headers.get("Reply-Text"))
        .cloned()
        .unwrap_or_default();

    if !reply_text.starts_with("+OK") {
        return Err(anyhow!("Authentication failed: {}", reply_text));
    }

    tracing::debug!("Authenticated to FreeSWITCH: {}", reply_text);

    // Subscribe to events
    let event_names: Vec<String> = config.event_list.iter().map(|e| e.to_string()).collect();
    let event_list_str = event_names.join(" ");
    let event_cmd = format!("event {} {}\n\n", config.event_format, event_list_str);
    writer
        .write_all(event_cmd.as_bytes())
        .await
        .context("Failed to send event subscription")?;

    // Read and parse event subscription response
    let event_response = reader
        .read_next_event()
        .await
        .context("Failed to read event subscription response")?
        .ok_or_else(|| anyhow!("Unexpected EOF while reading event subscription response"))?;

    // Check if event subscription was successful
    let event_reply_text = event_response
        .frame_headers
        .get("Reply-Text")
        .or_else(|| event_response.event_headers.get("Reply-Text"))
        .cloned()
        .unwrap_or_default();

    if !event_reply_text.starts_with("+OK") {
        return Err(anyhow!("Event subscription failed: {}", event_reply_text));
    }

    tracing::debug!("Subscribed to FreeSWITCH events: {}", event_reply_text);

    Ok((reader, writer))
}

async fn run_connection(
    fs_reader: EslReader<OwnedReadHalf>,
    fs_writer: OwnedWriteHalf,
    cmd_rx: &mut mpsc::Receiver<EslCommand>,
    event_tx: &mpsc::Sender<EslEvent>,
) -> Result<()> {
    let pending_api = Arc::new(Mutex::new(
        VecDeque::<oneshot::Sender<Result<EslEvent>>>::new(),
    ));

    // Spawn reader task
    let pending_api_reader = pending_api.clone();
    let event_tx_clone = event_tx.clone();
    let mut reader_handle = tokio::spawn(read_events_and_forward(
        fs_reader,
        event_tx_clone,
        pending_api_reader,
    ));

    // Run writer in the current task (so we can use the mutable reference to cmd_rx)
    let pending_api_writer = pending_api.clone();

    // Use tokio::select! to wait for either reader or writer to fail
    // Pin the writer future so we can use it in select
    let writer_future = send_commands_to_fs(fs_writer, cmd_rx, pending_api_writer);
    tokio::pin!(writer_future);

    // Wait for either reader or writer to fail
    // If reader fails, connection is lost (EOF or parse error)
    // If writer fails, connection is lost (write error)
    tokio::select! {
        reader_result = &mut reader_handle => {
            // Reader task completed (either successfully or with error)
            match reader_result {
                Ok(Ok(())) => {
                    // Reader completed successfully (shouldn't happen in normal operation)
                    // Continue with writer
                    writer_future.await
                }
                Ok(Err(e)) => {
                    // Reader detected disconnection - this is the key fix!
                    // Now the supervisor will know the connection is lost
                    Err(e)
                }
                Err(e) => {
                    // Reader task panicked
                    Err(anyhow!("Reader task panicked: {:?}", e))
                }
            }
        }
        writer_result = writer_future.as_mut() => {
            // Writer task completed (connection lost on write)
            reader_handle.abort();
            writer_result
        }
    }
}

async fn read_events_and_forward(
    mut fs_reader: EslReader<OwnedReadHalf>,
    fs_event_tx: mpsc::Sender<EslEvent>,
    pending_api: Arc<Mutex<VecDeque<oneshot::Sender<Result<EslEvent>>>>>,
) -> Result<()> {
    loop {
        let event = match fs_reader.read_next_event().await {
            Ok(Some(e)) => e,
            Ok(None) => {
                // EOF detected - connection closed by FreeSWITCH
                tracing::warn!("ESL connection closed (EOF detected)");
                let mut q = pending_api.lock().await;
                while let Some(tx) = q.pop_front() {
                    let _ = tx.send(Err(anyhow!("ESL connection closed")));
                }
                return Err(anyhow!("ESL connection closed (EOF)"));
            }
            Err(e) => {
                // Parse error usually indicates connection issue
                tracing::error!("Failed to parse event (connection likely lost): {}", e);
                let mut q = pending_api.lock().await;
                while let Some(tx) = q.pop_front() {
                    let _ = tx.send(Err(anyhow!("ESL connection closed")));
                }
                return Err(e.context("ESL connection lost"));
            }
        };

        let content_type = event
            .frame_headers
            .get("Content-Type")
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
                    tracing::debug!("[ESL WRITE] Sending API command: {}", wire);
                    tracing::debug!(
                        "[ESL WRITE] Raw bytes ({}): {:?}",
                        wire.len(),
                        wire.as_bytes()
                    );
                    fs_writer.write_all(wire.as_bytes()).await?;
                }
                EslCommand::SendRaw { lines } => {
                    tracing::debug!(
                        "[ESL WRITE] Sending raw command ({} bytes): {:?}",
                        lines.len(),
                        lines.as_bytes()
                    );
                    fs_writer.write_all(lines.as_bytes()).await?;
                }
                EslCommand::Close => {
                    tracing::debug!("[ESL WRITE] Received Close command, breaking loop");
                    break;
                }
            }
        } else {
            tracing::debug!("[ESL WRITE] Command channel closed, breaking loop");
            break;
        }
    }

    Ok(())
}
