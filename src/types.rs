use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub input_data: Option<String>,
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
            input_data: None,
        }
    }

    pub fn new_with_input(
        code: impl Into<String>,
        message: impl Into<String>,
        details: impl Into<String>,
        input_data: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: details.into(),
            input_data,
        }
    }

    pub fn from_kkt_value(value: Value) -> Self {
        let details = match value.get("error") {
            Some(v) => {
                if let Value::String(m) = v {
                    m.clone()
                } else {
                    "No error message".to_string()
                }
            }
            None => "No error message".to_string(),
        };

        let input_data = match value.get("input") {
            Some(v) => {
                if let Value::String(m) = v {
                    Some(m.clone())
                } else {
                    None
                }
            }
            None => None,
        };

        Self {
            code: "KKT_REQUEST_FAILURE".to_string(),
            message: "Ошибка при выполнении запроса к ККТ".to_string(),
            details,
            input_data,
        }
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} ({})", self.code, self.message, self.details)
    }
}

impl std::error::Error for ProcessError {}

impl From<anyhow::Error> for ProcessError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: "Произошла внутренняя ошибка".to_string(),
            details: err.to_string(),
            input_data: None,
        }
    }
}

impl From<reqwest::Error> for ProcessError {
    fn from(err: reqwest::Error) -> Self {
        Self {
            code: "NETWORK_ERROR".to_string(),
            message: "Ошибка сети при выполнении запроса".to_string(),
            details: err.to_string(),
            input_data: None,
        }
    }
}

impl From<std::io::Error> for ProcessError {
    fn from(err: std::io::Error) -> Self {
        Self {
            code: "IO_ERROR".to_string(),
            message: "Ошибка ввода-вывода".to_string(),
            details: err.to_string(),
            input_data: None,
        }
    }
}

impl From<Box<dyn std::error::Error>> for ProcessError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Self {
            code: "UNKNOWN_ERROR".to_string(),
            message: "Неизвестная ошибка".to_string(),
            details: err.to_string(),
            input_data: None,
        }
    }
}
