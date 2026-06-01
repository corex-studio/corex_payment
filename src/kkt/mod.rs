mod client;
mod healthcheck;
mod process;
pub mod types;

pub use types::*;

use std::process::Child;

use crate::{
    ProcessError, ProcessSuccess,
    healthcheck::{HealthcheckResult, Healthchecker},
};

pub struct Kkt {
    pub(crate) config: KktConfig,
    pub(crate) server_process: Option<Child>,
}

impl Kkt {
    pub fn new(config: types::KktConfig) -> Self {
        Self {
            config,
            server_process: None,
        }
    }

    pub async fn healthcheck(&self) -> Result<ProcessSuccess<HealthcheckResult>, ProcessError> {
        let result = self.run_healthcheck().await;
        let raw_result = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
        Ok(ProcessSuccess::new(
            "Healthcheck completed",
            result,
            raw_result,
        ))
    }
}
