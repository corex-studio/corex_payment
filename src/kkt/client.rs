use super::Kkt;
use super::types;
use anyhow::{anyhow, Result};

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

    pub async fn connect(&self) -> Result<serde_json::Value> {
        self.send("connect", "POST", None).await
    }

    pub async fn disconnect(&self) -> Result<serde_json::Value> {
        self.send("disconnect", "POST", None).await
    }

    pub async fn check_connection(&self) -> Result<bool> {
        let response = self.send("check", "GET", None).await;
        match response {
            Ok(v) => Ok(match &v["connected"] {
                serde_json::Value::Bool(v) => *v,
                _ => false,
            }),
            Err(e) => Err(e),
        }
    }

    pub async fn open_shift(&self, operator: &types::Operator) -> Result<serde_json::Value> {
        let data = serde_json::json!({ "operator": operator });
        self.send("open_shift", "POST", Some(&data)).await
    }

    pub async fn close_shift(&self, operator: &types::Operator) -> Result<serde_json::Value> {
        let data = serde_json::json!({ "operator": operator });
        self.send("close_shift", "POST", Some(&data)).await
    }

    pub async fn shift_status(&self) -> Result<serde_json::Value> {
        self.send("shift_status", "GET", None).await
    }

    pub async fn payment(&self, sell_task: &types::SellTask) -> Result<serde_json::Value> {
        let data = serde_json::to_value(sell_task)?;
        self.send("payment", "POST", Some(&data)).await
    }

    pub async fn refund(&self, sell_task: &types::SellTask) -> Result<serde_json::Value> {
        let data = serde_json::to_value(sell_task)?;
        self.send("refund", "POST", Some(&data)).await
    }

    pub async fn document(&self, id: u32) -> Result<serde_json::Value> {
        let data = serde_json::json!({ "number": id });
        self.send("document", "GET", Some(&data)).await
    }

    pub async fn info(&self) -> Result<serde_json::Value> {
        self.send("info", "GET", None).await
    }
}
