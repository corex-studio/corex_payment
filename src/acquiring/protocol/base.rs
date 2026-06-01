use async_trait::async_trait;
use std::result::Result;

use crate::{
    ProcessError, ProcessSuccess,
    acquiring::types::{ConnectionStatus, NormalizedTransactionData},
    healthcheck::HealthcheckResult,
};

#[async_trait]
pub trait Acquiring: Send + Sync {
    async fn connected(&self) -> Result<ProcessSuccess<ConnectionStatus>, ProcessError>;

    async fn connect(&mut self) -> Result<ProcessSuccess<ConnectionStatus>, ProcessError>;

    async fn disconnect(&mut self) -> Result<ProcessSuccess<ConnectionStatus>, ProcessError>;

    async fn payment(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> Result<ProcessSuccess<NormalizedTransactionData>, ProcessError>;

    async fn totals(&mut self) -> Result<ProcessSuccess<NormalizedTransactionData>, ProcessError>;

    async fn refund(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> Result<ProcessSuccess<NormalizedTransactionData>, ProcessError>;

    async fn healthcheck(&self) -> Result<ProcessSuccess<HealthcheckResult>, ProcessError>;
}
