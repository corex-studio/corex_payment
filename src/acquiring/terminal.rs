use crate::acquiring::commands::{PaymentCommand, RefundCommand, TotalsCommand};
use crate::acquiring::connection::{BaseConnection, InpasConnection, TcpConnection, UsbConnection};
use crate::acquiring::types::{ConnectionConfig, ConnectionType, TerminalResponse};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Terminal {
    connection: Option<Arc<Mutex<Box<dyn BaseConnection>>>>,
    config: ConnectionConfig,
}

impl Terminal {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            connection: None,
            config,
        }
    }

    pub async fn connect(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if matches!(self.config.protocol, crate::acquiring::types::ProtocolType::Inpas) {
            let mut conn: Box<dyn BaseConnection> = Box::new(InpasConnection::new(self.config.clone()));
            let result = conn.connect().await?;
            self.connection = Some(Arc::new(Mutex::new(conn)));
            return Ok(result);
        }

        let mut conn: Box<dyn BaseConnection> = match self.config.connection_type {
            ConnectionType::Tcp => Box::new(TcpConnection::new(self.config.clone())),
            ConnectionType::Usb => Box::new(UsbConnection::new(self.config.clone())),
            ConnectionType::Bluetooth => {
                return Err("Bluetooth connection not yet implemented".into())
            }
        };

        let result = conn.connect().await?;
        self.connection = Some(Arc::new(Mutex::new(conn)));
        Ok(result)
    }

    pub async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(conn) = &self.connection {
            let mut conn = conn.lock().await;
            conn.disconnect().await?;
        }
        self.connection = None;
        Ok(())
    }

    pub async fn payment(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> Result<TerminalResponse, Box<dyn std::error::Error>> {
        let conn = self
            .connection
            .as_ref()
            .ok_or("Not connected to terminal")?;

        let conn_guard = conn.lock().await;
        if !conn_guard.is_connected() {
            return Err("Not connected to terminal".into());
        }
        drop(conn_guard);

        let conn = Arc::clone(conn);
        let context = crate::acquiring::commands::base::CommandContext::new(conn);
        let command = PaymentCommand::new(amount, currency);
        command.execute(context).await
    }

    pub async fn totals(&mut self) -> Result<TerminalResponse, Box<dyn std::error::Error>> {
        let conn = self
            .connection
            .as_ref()
            .ok_or("Not connected to terminal")?;

        let conn_guard = conn.lock().await;
        if !conn_guard.is_connected() {
            return Err("Not connected to terminal".into());
        }
        drop(conn_guard);

        let conn = Arc::clone(conn);
        let context = crate::acquiring::commands::base::CommandContext::new(conn);
        let command = TotalsCommand::new();
        command.execute(context).await
    }

    pub async fn refund(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> Result<TerminalResponse, Box<dyn std::error::Error>> {
        let conn = self
            .connection
            .as_ref()
            .ok_or("Not connected to terminal")?;

        let conn_guard = conn.lock().await;
        if !conn_guard.is_connected() {
            return Err("Not connected to terminal".into());
        }
        drop(conn_guard);

        let conn = Arc::clone(conn);
        let context = crate::acquiring::commands::base::CommandContext::new(conn);
        let command = RefundCommand::new(amount, currency);
        command.execute(context).await
    }

    pub fn connected(&self) -> bool {
        if let Some(conn) = &self.connection {
            // We can't easily check without blocking, so we'll use a try_lock
            if let Ok(conn_guard) = conn.try_lock() {
                return conn_guard.is_connected();
            }
        }
        false
    }
}

