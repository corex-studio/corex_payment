use super::Kkt;
use super::types;
use anyhow::{anyhow, Result};
use crate::{ProcessSuccess, ProcessError};

impl Kkt {
    fn make_url(&self, action: &str) -> String {
        format!("http://localhost:3000/{}", action)
    }

    async fn send(
        &self,
        action: &str,
        method: &str,
        data: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let url = self.make_url(action);
        let client = reqwest::Client::new();

        let response = if method == "GET" {
            let mut req = client.get(&url);
            if let Some(data) = data
                && let Some(obj) = data.as_object()
            {
                for (key, value) in obj {
                    req = req.query(&[(key, value)]);
                }
            }
            req.send().await?
        } else {
            let mut req = client.post(&url);
            if let Some(data) = data {
                req = req.json(data);
            }
            req.send().await?
        };

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error_data = response.json::<serde_json::Value>().await?;
            return Ok(error_data);
        }

        let raw_bytes = response.bytes().await;

        match raw_bytes {
            Ok(b) => match serde_json::from_slice(&b) {
                Ok(d) => Ok(d),
                Err(e) => {
                    let s = String::from_utf8(b.to_vec())?;
                    Err(anyhow!(
                        "Could not parse response. Error: {}. Raw data: {}",
                        e,
                        s
                    ))
                }
            },
            Err(e) => Err(anyhow!("Cannot read response. {:?}", e)),
        }
    }

    pub async fn connect(&self) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        self.send("connect", "POST", None).await
            .map(|v| ProcessSuccess::new("CONNECT_SUCCESSFUL", v))
            .map_err(|e| ProcessError::new("CONNECT_FAIL", "Ошибка подключения к ККТ", e.to_string()))
    }

    pub async fn disconnect(&self) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        self.send("disconnect", "POST", None).await
            .map(|v| ProcessSuccess::new("DISCONNECT_SUCCESSFUL", v))
            .map_err(|e| ProcessError::new("DISCONNECT_FAIL", "Ошибка отключения от ККТ", e.to_string()))
    }

    pub async fn check_connection(&self) -> std::result::Result<ProcessSuccess<bool>, ProcessError> {
        let response = self.send("check", "GET", None).await
            .map_err(|e| ProcessError::new("CHECK_CONNECTION_FAIL", "Ошибка проверки статуса", e.to_string()))?;
        let connected = match &response["connected"] {
            serde_json::Value::Bool(v) => *v,
            _ => false,
        };
        Ok(ProcessSuccess::new("CHECK_CONNECTION_SUCCESSFUL", connected))
    }

    pub async fn open_shift(&self, operator: &types::Operator) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::json!({ "operator": operator });
        self.send("open_shift", "POST", Some(&data)).await
            .map(|v| ProcessSuccess::new("OPEN_SHIFT_SUCCESSFUL", v))
            .map_err(|e| ProcessError::new("OPEN_SHIFT_FAIL", "Ошибка открытия смены", e.to_string()))
    }

    pub async fn close_shift(&self, operator: &types::Operator) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::json!({ "operator": operator });
        self.send("close_shift", "POST", Some(&data)).await
            .map(|v| ProcessSuccess::new("CLOSE_SHIFT_SUCCESSFUL", v))
            .map_err(|e| ProcessError::new("CLOSE_SHIFT_FAIL", "Ошибка закрытия смены", e.to_string()))
    }

    pub async fn shift_status(&self) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        self.send("shift_status", "GET", None).await
            .map(|v| ProcessSuccess::new("SHIFT_STATUS_SUCCESSFUL", v))
            .map_err(|e| ProcessError::new("SHIFT_STATUS_FAIL", "Ошибка получения статуса смены", e.to_string()))
    }

    pub async fn payment(&self, sell_task: &types::SellTask) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::to_value(sell_task)
            .map_err(|e| ProcessError::new("PAYMENT_FAIL", "Ошибка формирования данных для оплаты", e.to_string()))?;
        self.send("payment", "POST", Some(&data)).await
            .map(|v| ProcessSuccess::new("PAYMENT_SUCCESSFUL", v))
            .map_err(|e| ProcessError::new("PAYMENT_FAIL", "Ошибка при проведении оплаты", e.to_string()))
    }

    pub async fn refund(&self, sell_task: &types::SellTask) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::to_value(sell_task)
            .map_err(|e| ProcessError::new("REFUND_FAIL", "Ошибка формирования данных для возврата", e.to_string()))?;
        self.send("refund", "POST", Some(&data)).await
            .map(|v| ProcessSuccess::new("REFUND_SUCCESSFUL", v))
            .map_err(|e| ProcessError::new("REFUND_FAIL", "Ошибка при выполнении возврата", e.to_string()))
    }

    pub async fn document(&self, id: u32) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::json!({ "number": id });
        self.send("document", "GET", Some(&data)).await
            .map(|v| ProcessSuccess::new("DOCUMENT_SUCCESSFUL", v))
            .map_err(|e| ProcessError::new("DOCUMENT_FAIL", "Ошибка получения документа", e.to_string()))
    }

    pub async fn info(&self) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        self.send("info", "GET", None).await
            .map(|v| ProcessSuccess::new("INFO_SUCCESSFUL", v))
            .map_err(|e| ProcessError::new("INFO_FAIL", "Ошибка получения информации о ККТ", e.to_string()))
    }
}
