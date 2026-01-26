use crate::ProtocolType;
use crate::acquiring::protocol::base::Acquiring;
use crate::acquiring::protocol::{InpasAdapter, SBAdapter};
use crate::acquiring::types::{ConnectionConfig, TerminalResponse};
use anyhow::Result;
use std::path::PathBuf;
use std::str::FromStr;

pub struct Terminal {
    adapter: Box<dyn Acquiring>,
}

impl Terminal {
    pub fn new(config: ConnectionConfig) -> Self {
        let default_sc552 = PathBuf::from_str("./libs/sc552/").unwrap_or_else(|_| {
            let mut p = PathBuf::new();
            p.push("C:/");
            p.push("sc552/");
            p
        });

        let sc552 = match &config.sc552_path {
            Some(v) => PathBuf::from_str(v).unwrap_or(default_sc552.clone()),
            None => default_sc552.clone(),
        };

        let adapter: Box<dyn Acquiring> = match config.protocol {
            ProtocolType::Ttk => Box::new(SBAdapter::new(config.clone(), sc552)),
            ProtocolType::Inpas => Box::new(InpasAdapter::new(config.clone())),
        };

        Self { adapter }
    }

    pub async fn connect(&mut self) -> Result<bool> {
        self.adapter.connect().await
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.adapter.disconnect().await
    }

    pub async fn payment(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> Result<TerminalResponse> {
        self.adapter.payment(amount, currency).await
    }

    pub async fn totals(&mut self) -> Result<TerminalResponse> {
        self.adapter.totals().await
    }

    pub async fn refund(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> Result<TerminalResponse> {
        self.adapter.refund(amount, currency).await
    }

    pub async fn connected(&self) -> bool {
        self.adapter.connected().await
    }
}
