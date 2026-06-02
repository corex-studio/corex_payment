use super::Kkt;
use super::types;
use crate::kkt::ConnectionStatus;
use crate::{ProcessError, ProcessSuccess};
use anyhow::Result;
use serde_json::Value;

impl Kkt {
    fn make_url(&self, action: &str) -> String {
        format!("http://localhost:3000/{}", action)
    }

    async fn send(
        &self,
        action: &str,
        method: &str,
        data: Option<&serde_json::Value>,
    ) -> Result<ProcessSuccess<serde_json::Value>, ProcessError> {
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
            return Err(ProcessError::from_kkt_value(error_data));
        }

        let raw_bytes = response.bytes().await;
        let raw_input = data.map(|v| v.to_string());

        match raw_bytes {
            Ok(b) => match serde_json::from_slice(&b) {
                Ok(d) => Ok(ProcessSuccess::new(
                    format!("Action {action} completed successfully"),
                    d,
                    Value::String(String::from_utf8(b.to_vec()).unwrap_or("".to_string())),
                )),
                Err(e) => Err(ProcessError::from_error_with_input(
                    "Could not parse KKT response",
                    e,
                    raw_input,
                )),
            },
            Err(e) => Err(ProcessError::from_error_with_input(
                "Could not read KKT response",
                e,
                raw_input,
            )),
        }
    }

    pub async fn connect(
        &self,
    ) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        self.send("connect", "POST", None).await
    }

    pub async fn disconnect(
        &self,
    ) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        self.send("disconnect", "POST", None).await
    }

    pub async fn check_connection(
        &self,
    ) -> std::result::Result<ProcessSuccess<ConnectionStatus>, ProcessError> {
        let response = self.send("check", "GET", None).await;
        let connected = response.is_ok();
        Ok(ProcessSuccess::new(
            "Connection status",
            ConnectionStatus::new(connected),
            Value::Bool(connected),
        ))
    }

    pub async fn open_shift(
        &self,
        operator: &types::Operator,
    ) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::json!({ "operator": operator });
        self.send("open_shift", "POST", Some(&data)).await
    }

    pub async fn close_shift(
        &self,
        operator: &types::Operator,
    ) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::json!({ "operator": operator });
        self.send("close_shift", "POST", Some(&data)).await
    }

    pub async fn shift_status(
        &self,
    ) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        self.send("shift_status", "GET", None).await
    }

    pub async fn payment(
        &self,
        sell_task: &types::SellTask,
    ) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::to_value(sell_task).map_err(|e| {
            ProcessError::from_error_with_input(
                "Failed to parse input data into JSON value",
                e,
                Some(format!("{:?}", sell_task)),
            )
        })?;
        self.send("payment", "POST", Some(&data)).await
    }

    pub async fn refund(
        &self,
        sell_task: &types::SellTask,
    ) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::to_value(sell_task).map_err(|e| {
            ProcessError::from_error_with_input(
                "Failed to parse input data into JSON value",
                e,
                Some(format!("{:?}", sell_task)),
            )
        })?;
        self.send("refund", "POST", Some(&data)).await
    }

    pub async fn document(
        &self,
        id: u32,
    ) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        let data = serde_json::json!({ "number": id });
        self.send("document", "GET", Some(&data)).await
    }

    pub async fn info(
        &self,
    ) -> std::result::Result<ProcessSuccess<serde_json::Value>, ProcessError> {
        self.send("info", "GET", None).await
    }
}
