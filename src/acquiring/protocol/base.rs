use async_trait::async_trait;
use std::result::Result;

use crate::{
    ProcessError, ProcessSuccess,
    acquiring::types::{ConnectionStatus, NormalizedTransactionData, RefundMode},
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

    /// Refund using the original payment's RRN. Unsupported adapters must not
    /// silently fall back to an unreferenced refund.
    async fn refund_with_reference(
        &mut self,
        amount: u64,
        currency: Option<String>,
        rrn: String,
        mode: RefundMode,
    ) -> Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        Err(ProcessError::new_with_input(
            "Возврат по RRN не поддерживается этим адаптером",
            serde_json::json!({ "original_rrn": rrn, "refund_mode": mode, "currency": currency }),
            Some(amount.to_string()),
        ))
    }

    async fn healthcheck(&self) -> Result<ProcessSuccess<HealthcheckResult>, ProcessError>;
}
