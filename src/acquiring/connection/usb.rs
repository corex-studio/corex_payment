use crate::acquiring::connection::BaseConnection;
use crate::acquiring::types::ConnectionConfig;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

pub struct UsbConnection {
    config: ConnectionConfig,
    port: Option<Arc<Mutex<SerialStream>>>,
    connected: bool,
}

impl UsbConnection {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            port: None,
            connected: false,
        }
    }
}

#[async_trait::async_trait]
impl BaseConnection for UsbConnection {
    fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn connect(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let path = self
            .config
            .ncom
            .as_ref()
            .or_else(|| self.config.address.as_ref())
            .ok_or("USB path or serialNumber must be provided in config")?;

        let baud_rate = self.config.baudrate.unwrap_or(9600);
        let port = tokio_serial::new(path, baud_rate).open_native_async()?;
        self.port = Some(Arc::new(Mutex::new(port)));
        self.connected = true;
        Ok(true)
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.port = None;
        self.connected = false;
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("Not connected".into());
        }

        if let Some(port) = &self.port {
            let mut port = port.lock().await;
            use tokio::io::AsyncWriteExt;
            port.write_all(data).await?;
            port.flush().await?;
            Ok(())
        } else {
            Err("Not connected".into())
        }
    }

    async fn read(&mut self, timeout_ms: Option<u32>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("Not connected".into());
        }

        if let Some(port) = &self.port {
            let mut port = port.lock().await;
            use tokio::io::AsyncReadExt;

            let mut buffer = vec![0u8; 4096];
            let read_future = port.read(&mut buffer);

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

