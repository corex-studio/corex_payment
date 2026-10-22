use anyhow::Error;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait Healthchecker {
    async fn run_healthcheck(&self) -> HealthcheckResult {
        let port_connected = self.check_port().await;
        let drivers_ready = self.check_drivers();

        let success = port_connected.is_success() && drivers_ready.is_success();
        if success {
            return HealthcheckResult::success();
        }

        let mut message: String = String::new();
        let mut details: Vec<String> = Vec::new();

        if let CheckUnit::Error(m, e) = port_connected {
            message.push_str(&format!("{}\n", m));
            if let Some(err) = e {
                details.push(format!("{}", err));
            }
        }
        if let CheckUnit::Error(m, e) = drivers_ready {
            message.push_str(&format!("{}\n", m));
            if let Some(err) = e {
                details.push(format!("{}", err));
            }
        }

        HealthcheckResult::error(message, details)
    }

    async fn check_port(&self) -> CheckUnit;

    fn check_drivers(&self) -> CheckUnit;
}

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

#[cfg(test)]
mod tests {
    use super::*;

    struct FailedChecks;

    #[async_trait]
    impl Healthchecker for FailedChecks {
        async fn check_port(&self) -> CheckUnit {
            CheckUnit::Error("Ошибка порта".into(), Some(anyhow::anyhow!("port details")))
        }

        fn check_drivers(&self) -> CheckUnit {
            CheckUnit::Error(
                "Ошибка драйвера".into(),
                Some(anyhow::anyhow!("driver details")),
            )
        }
    }

    #[tokio::test]
    async fn preserves_both_check_errors() {
        let result = FailedChecks.run_healthcheck().await;
        assert!(!result.success);
        assert_eq!(
            result.message.as_deref(),
            Some("Ошибка порта\nОшибка драйвера\n")
        );
        assert_eq!(result.details.unwrap(), ["port details", "driver details"]);
    }
}
