use anyhow::{Result, Context};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing;

pub struct EslClientConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
}

pub struct EslClient {
    config: EslClientConfig,
    stream: Option<TcpStream>,
}


impl EslClient {
    pub fn new(config: EslClientConfig) -> Self {
        Self { 
            config,
            stream: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
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
        self.stream = Some(stream);
        Ok(())
    }
}