use anyhow::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthcheckResult {
    pub success: bool,
    pub message: Option<String>,
    pub details: Option<Vec<String>>,
}

impl HealthcheckResult {
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
            details: None,
        }
    }

    pub fn error(message: String, details: Vec<String>) -> Self {
        Self {
            success: false,
            message: Some(message),
            details: Some(details),
        }
    }
}

#[derive(Debug)]
pub enum CheckUnit {
    Success,
    Error(String, Option<Error>),
}

impl CheckUnit {
    pub fn is_success(&self) -> bool {
        matches!(self, CheckUnit::Success)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, CheckUnit::Error(_, _))
    }
}
