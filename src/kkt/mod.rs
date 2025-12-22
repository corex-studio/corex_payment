pub mod types;

pub use types::*;

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use tokio::process::Command as TokioCommand;

pub struct Kkt {
    config: KktConfig,
    server_process: Option<Child>,
}

impl Kkt {
    pub fn new(config: types::KktConfig) -> Self {
        Self {
            config: config,
            server_process: None,
        }
    }

    pub async fn get_open_processes(&self) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        let output = TokioCommand::new("pgrep")
            .arg("-f")
            .arg("kkt")
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<u32> = stdout
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect();
        Ok(pids)
    }

    pub async fn is_server_open(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let processes = self.get_open_processes().await?;
        Ok(!processes.is_empty())
    }

    pub async fn stop_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let processes = match self.get_open_processes().await {
            Ok(v) => v,
            Err(_) => {
                return Ok(());
            }
        };
        for pid in processes {
            let _ = TokioCommand::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .output()
                .await;
        }
        Ok(())
    }

    pub async fn run_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stop_server().await?;

        let bin_path = "./libs/kkt";
        let mut cmd = Command::new(bin_path);
        cmd.args(["--type", self.config.connection_type.raw()]);
        if let Some(v) = &self.config.address {
            cmd.args(["--address", &v]);
        };
        if let Some(v) = &self.config.port {
            cmd.args(["--port", &v.to_string()]);
        }
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        self.server_process = Some(cmd.spawn()?);
        Ok(())
    }

    fn make_url(&self, action: &str) -> String {
        format!("http://localhost:3000/{}", action)
    }

    async fn send(
        &self,
        action: &str,
        method: &str,
        data: Option<&serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        let url = self.make_url(action);
        let client = reqwest::Client::new();

        let response = if method == "GET" {
            let mut req = client.get(&url);
            if let Some(data) = data {
                if let Some(obj) = data.as_object() {
                    for (key, value) in obj {
                        req = req.query(&[(key, value.as_str().unwrap_or(""))]);
                    }
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
            let error_data = response
                .json::<HashMap<String, serde_json::Value>>()
                .await?;
            return Ok(error_data);
        }

        let data = response
            .json::<HashMap<String, serde_json::Value>>()
            .await?;
        Ok(data)
    }

    pub async fn connect(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        self.send("connect", "POST", None).await
    }

    pub async fn disconnect(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        self.send("disconnect", "POST", None).await
    }

    pub async fn check_connection(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let response = self.send("check", "GET", None).await;
        match response {
            Ok(v) => Ok(match &v["connected"] {
                serde_json::Value::Bool(v) => *v,
                _ => false,
            }),
            Err(e) => Err(e),
        }
    }

    pub async fn open_shift(
        &self,
        operator: &types::Operator,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        let data = serde_json::json!({ "operator": operator });
        self.send("open_shift", "POST", Some(&data)).await
    }

    pub async fn close_shift(
        &self,
        operator: &types::Operator,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        let data = serde_json::json!({ "operator": operator });
        self.send("close_shift", "POST", Some(&data)).await
    }

    pub async fn payment(
        &self,
        sell_task: &types::SellTask,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        let data = serde_json::to_value(sell_task)?;
        self.send("payment", "POST", Some(&data)).await
    }

    pub async fn refund(
        &self,
        sell_task: &types::SellTask,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        let data = serde_json::to_value(sell_task)?;
        self.send("refund", "POST", Some(&data)).await
    }

    pub async fn document(
        &self,
        id: u32,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        let data = serde_json::json!({ "number": id });
        self.send("document", "GET", Some(&data)).await
    }

    pub async fn info(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        self.send("info", "GET", None).await
    }
}
