use crate::acquiring::connection::BaseConnection;
use crate::acquiring::types::ConnectionConfig;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

pub struct TcpConnection {
    config: ConnectionConfig,
    stream: Option<Arc<Mutex<TcpStream>>>,
    connected: bool,
}

impl TcpConnection {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            stream: None,
            connected: false,
        }
    }
}

#[async_trait::async_trait]
impl BaseConnection for TcpConnection {
    fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn connect(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let port = self.config.port.ok_or("Port is required for TCP connection")?;
        let address = self
            .config
            .address
            .as_ref()
            .ok_or("Address is required for TCP connection")?;

        let addr = format!("{}:{}", address, port);
        let stream = TcpStream::connect(&addr).await?;
        self.stream = Some(Arc::new(Mutex::new(stream)));
        self.connected = true;
        Ok(true)
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(stream) = &self.stream {
            let mut stream = stream.lock().await;
            use tokio::io::AsyncWriteExt;
            stream.shutdown().await?;
        }
        self.stream = None;
        self.connected = false;
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("Not connected".into());
        }

        if let Some(stream) = &self.stream {
            let mut stream = stream.lock().await;
            use tokio::io::AsyncWriteExt;
            stream.write_all(data).await?;
            stream.flush().await?;
            Ok(())
        } else {
            Err("Not connected".into())
        }
    }

    async fn read(&mut self, timeout_ms: Option<u32>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("Not connected".into());
        }

        if let Some(stream) = &self.stream {
            let mut stream = stream.lock().await;
            use tokio::io::AsyncReadExt;

            let mut buffer = vec![0u8; 4096];
            let read_future = stream.read(&mut buffer);

            let result = if let Some(timeout_ms) = timeout_ms {
                timeout(Duration::from_millis(timeout_ms as u64), read_future).await?
            } else {
                read_future.await
            };

            let n = result?;
            buffer.truncate(n);
            Ok(buffer)
        } else {
            Err("Not connected".into())
        }
    }
}

