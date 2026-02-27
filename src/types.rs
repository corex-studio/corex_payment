use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSuccess<T> {
    pub code: String,
    pub data: T,
}

impl<T> ProcessSuccess<T> {
    pub fn new(code: impl Into<String>, data: T) -> Self {
        Self {
            code: code.into(),
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessError {
    pub code: String,
    pub message: String,
    pub details: String,
}

impl ProcessError {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: details.into(),
        }
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} ({})",
            self.code, self.message, self.details
        )
    }
}

impl std::error::Error for ProcessError {}

impl From<anyhow::Error> for ProcessError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: "Произошла внутренняя ошибка".to_string(),
            details: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for ProcessError {
    fn from(err: reqwest::Error) -> Self {
        Self {
            code: "NETWORK_ERROR".to_string(),
            message: "Ошибка сети при выполнении запроса".to_string(),
            details: err.to_string(),
        }
    }
}

impl From<std::io::Error> for ProcessError {
    fn from(err: std::io::Error) -> Self {
        Self {
            code: "IO_ERROR".to_string(),
            message: "Ошибка ввода-вывода".to_string(),
            details: err.to_string(),
        }
    }
}

impl From<Box<dyn std::error::Error>> for ProcessError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Self {
            code: "UNKNOWN_ERROR".to_string(),
            message: "Неизвестная ошибка".to_string(),
            details: err.to_string(),
        }
    }
}
