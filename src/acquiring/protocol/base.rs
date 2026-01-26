use anyhow::Result;
use async_trait::async_trait;

use crate::TerminalResponse;

#[async_trait]
pub trait Acquiring: Send + Sync {
    async fn connected(&self) -> bool;

    async fn connect(&mut self) -> Result<bool>;

    async fn disconnect(&mut self) -> Result<()>;

    async fn payment(&mut self, amount: u64, currency: Option<String>) -> Result<TerminalResponse>;

    async fn totals(&mut self) -> Result<TerminalResponse>;

    async fn refund(&mut self, amount: u64, currency: Option<String>) -> Result<TerminalResponse>;
}
