use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSuccess<T> {
    pub message: String,
    pub data: T,
    pub raw_data: Value,
}

impl<T> ProcessSuccess<T> {
    pub fn new(message: impl Into<String>, data: T, raw_data: Value) -> Self {
        Self {
            message: message.into(),
            data,
            raw_data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessError {
    pub message: String,
    pub raw_data: Value,
    pub input_data: Option<String>,
}

impl ProcessError {
    pub fn new(message: impl Into<String>, raw_data: Value) -> Self {
        Self {
            message: message.into(),
            raw_data: json!({
                "error": raw_data,
            }),
            input_data: None,
        }
    }

    pub fn new_with_input(
        message: impl Into<String>,
        raw_data: Value,
        input_data: Option<String>,
    ) -> Self {
        Self {
            message: message.into(),
            raw_data: json!({
                "error": raw_data,
            }),
            input_data,
        }
    }

    pub fn from_error(message: impl Into<String>, error: impl std::error::Error) -> Self {
        Self {
            message: message.into(),
            raw_data: json!({
                "error": error.to_string(),
            }),
            input_data: None,
        }
    }

    pub fn from_error_with_input(
        message: impl Into<String>,
        error: impl std::error::Error,
        input_data: Option<String>,
    ) -> Self {
        Self {
            message: message.into(),
            raw_data: json!({
                "error": error.to_string(),
            }),
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
            message: "Error while processing a KKT call".to_string(),
            raw_data: Value::String(details),
            input_data,
        }
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {:?}", self.message, self.raw_data)
    }
}

impl std::error::Error for ProcessError {}

impl From<anyhow::Error> for ProcessError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            message: "Internal error".to_string(),
            raw_data: Value::String(err.to_string()),
            input_data: None,
        }
    }
}

impl From<reqwest::Error> for ProcessError {
    fn from(err: reqwest::Error) -> Self {
        Self {
            message: "Error while HTTP call".to_string(),
            raw_data: Value::String(err.to_string()),
            input_data: None,
        }
    }
}

impl From<std::io::Error> for ProcessError {
    fn from(err: std::io::Error) -> Self {
        Self {
            message: "IO error".to_string(),
            raw_data: Value::String(err.to_string()),
            input_data: None,
        }
    }
}

impl From<Box<dyn std::error::Error>> for ProcessError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Self {
            message: "Unexpected error".to_string(),
            raw_data: Value::String(err.to_string()),
            input_data: None,
        }
    }
}
