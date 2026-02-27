mod client;
mod healthcheck;
mod process;
pub mod types;

pub use types::*;

use std::process::Child;

use crate::healthcheck::{HealthcheckResult, Healthchecker};

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

    pub async fn healthcheck(&self) -> HealthcheckResult {
        self.run_healthcheck().await
    }
}
