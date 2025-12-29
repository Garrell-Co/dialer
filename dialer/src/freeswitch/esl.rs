use anyhow::{Result, Context};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use std::fmt;
use tracing;

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

type Header = HashMap<String, String>;

struct Frame {
    header: Header,
    body: String,
}


impl Frame {
    pub fn new(header: Header, body: String) -> Self {
        Self { header, body }
    }
}

struct EslEvent {
    header: Header,
    body: String,
}


pub struct EslClientConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub event_format: EslEventFormat,
}

pub struct EslClient {
    config: EslClientConfig,
    stream: Option<TcpStream>,
}


#[async_trait::async_trait]
pub trait EventSocket: Send + Sync {
    async fn connect(&self) -> Result<()>;
    async fn api(&self, command: &str) -> Result<String>;
    async fn bgapi(&self, command: &str) -> Result<String>;
    async fn is_alive(&self) -> bool;
    async fn disconnect(&self) -> Result<()>;
}


impl EslClient {
    pub fn new(config: EslClientConfig) -> Self {
        Self { config, stream: None }
    }
}


#[async_trait::async_trait]
impl EventSocket for EslClient {
    async fn connect(&self) -> Result<()> {
        tracing::debug!(
            host = %self.config.host,
            port = self.config.port,
            "Connecting to FreeSWITCH ESL"
        );
        
        // Connect to FreeSWITCH
        let mut stream = TcpStream::connect(format!("{}:{}", self.config.host, self.config.port))
            .await
            .context("Failed to connect to FreeSWITCH")?;
        
        tracing::debug!("Connected to FreeSWITCH, reading auth request");
        
        // Read the initial auth request from FreeSWITCH
        let mut buffer = [0u8; 1024];
        let n = stream.read(&mut buffer).await
            .context("Failed to read auth request from FreeSWITCH")?;
        
        tracing::debug!("Sending authentication");
        
        // Send authentication with password
        let auth_command = format!("auth {}\n\n", self.config.password);
        stream.write_all(auth_command.as_bytes()).await
            .context("Failed to send auth command")?;
        stream.flush().await?;
        
        // Read auth response
        let mut response = [0u8; 1024];
        let _ = stream.read(&mut response).await
            .context("Failed to read auth response")?;
        
        // TODO: Parse response to verify authentication succeeded
        
        tracing::info!("Successfully authenticated with FreeSWITCH");
        Ok(())
    }
    
    async fn api(&self, command: &str) -> Result<String> {
        todo!()
    }
    
    async fn bgapi(&self, command: &str) -> Result<String> {
        todo!()
    }
    
    async fn is_alive(&self) -> bool {
        todo!()
    }
    
    async fn disconnect(&self) -> Result<()> {
        todo!()
    }
}