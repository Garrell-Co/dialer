use anyhow::{Context, Result, anyhow};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, mpsc, oneshot};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;
use tracing;

use crate::freeswitch::parser::{EslParser, Frame};

pub type Headers = HashMap<String, String>;

pub struct EslEvent {
    pub event_name: String,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

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

pub struct EslClientConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub event_format: EslEventFormat,
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


#[async_trait::async_trait]
pub trait EslConnector {
    async fn connect(&self) -> Result<Box<dyn EslPort>>;
}


#[async_trait::async_trait]
impl EslConnector for EslClientConfig {
    async fn connect(&self) -> Result<Box<dyn EslPort>> {
        Ok(Box::new(EslClient::connect(self.host.clone(), self.port).await?))
    }
}

pub struct EslConnection {
    cmd_tx: mpsc::Sender<EslCommand>,
    event_rx: Option<mpsc::Receiver<EslEvent>>,
}


impl EslConnection {
    pub fn take_event_rx(&mut self) -> mpsc::Receiver<EslEvent> {
        self.event_rx.take().expect("ESL event_rx already taken")
    }
}


#[async_trait::async_trait]
pub trait EslPort: Send + Sync {
    async fn api(&self, cmd: String) -> Result<EslEvent>;
    async fn send_raw(&self, lines: String) -> Result<()>;
    fn take_event_rx(&mut self) -> mpsc::Receiver<EslEvent>;
}


#[async_trait::async_trait]
impl EslPort for EslConnection {
    async fn api(&self, cmd: String) -> Result<EslEvent> {
        let (tx, rx) = oneshot::channel();

        self.cmd_tx
            .send(EslCommand::Api { cmd, reply: tx })
            .await
            .map_err(|_| anyhow!("ESL command channel closed"))?;

        rx.await.map_err(|_| anyhow!("ESL reply channel dropped"))?
    }

    async fn send_raw(&self, lines: String) -> Result<()> {
        self.cmd_tx
            .send(EslCommand::SendRaw { lines })
            .await
            .map_err(|_| anyhow!("ESL command channel closed"))
    }

    fn take_event_rx(&mut self) -> mpsc::Receiver<EslEvent> {
        EslConnection::take_event_rx(self)
    }
}

pub struct EslClient; 


impl EslClient {
    async fn connect(host: String, port: u16) -> Result<EslConnection> {
        tracing::debug!(
            host = host,
            port = port,
            "Connecting to FreeSWITCH ESL"
        );

        // Connect to FreeSWITCH
        let fs_conn_str = format!("{}:{}", host, port);
        let fs_stream = TcpStream::connect(fs_conn_str).await
            .context("Failed to connect to FreeSWITCH")?;
        let (mut fs_reader, mut fs_writer) = fs_stream.into_split();
        
        // Setup reader and writer tasks
        tracing::debug!("Connected to FreeSWITCH");

        let pending_api= Arc::new(Mutex::new(VecDeque::<oneshot::Sender<Result<EslEvent>>>::new()));

        tracing::debug!("Spawning command task");
        let pending_api_writer = pending_api.clone();
        let (command_tx, command_rx) = mpsc::channel::<EslCommand>(100);
        tokio::spawn(send_commands_to_fs(fs_writer, command_rx, pending_api_writer));

        let pending_api_reader = pending_api.clone();
        tracing::debug!("Spawning event task");
        let (event_tx, event_rx) = mpsc::channel::<EslEvent>(100);
        tokio::spawn(read_frames_and_forward(fs_reader, event_tx, pending_api_reader));

        Ok(EslConnection { cmd_tx: command_tx, event_rx: Some(event_rx) })
    }
}


async fn read_frames_and_forward(
    fs_reader: OwnedReadHalf,
    fs_event_tx: mpsc::Sender<EslEvent>,
    pending_api: Arc<Mutex<VecDeque<oneshot::Sender<Result<EslEvent>>>>>,
) -> Result<()> {
    let mut parser = EslParser::new(fs_reader);

    loop {
        let frame = match parser.parse_event().await {
            Ok(Some(e)) => e,
            Ok(None) => break,
            Err(e) => {
                tracing::error!("Failed to parse event: {}", e);
                break
            }
        };

        let event = esl_frame_to_event(&frame);

        match frame.content_type.as_str() {
            "text/event-plain" => {
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
                // ignore other message types in this minimal skeleton
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
    mut command_rx: mpsc::Receiver<EslCommand>,
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
        };
    }

    Ok(())
}


fn esl_frame_to_event(msg: &Frame) -> EslEvent {
    // For text/event-plain, event name is usually in "Event-Name"
    let event_name = msg
        .headers
        .get("Event-Name")
        .cloned()
        .unwrap_or_else(|| "UNKNOWN".into());

    EslEvent {
        event_name,
        headers: msg.headers.clone(),
        body: msg.body.clone(),
    }
}
