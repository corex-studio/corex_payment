pub mod types;

pub use types::*;

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use tokio::process::Command as TokioCommand;

pub struct Kkt {
    config: KktConfig,
    server_process: Option<Child>,
}

impl Kkt {
    pub fn new(config: types::KktConfig) -> Self {
        Self {
            config,
            server_process: None,
        }
    }

    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "windows")]
    pub async fn get_open_processes(&self) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        let output = TokioCommand::new("wmic")
            .args([
                "process",
                "where",
                "name='kkt.exe'",
                "get",
                "ProcessId",
                "/format:csv",
            ])
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<u32> = stdout
            .lines()
            .skip(1)
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 2 {
                    parts[1].parse().ok()
                } else {
                    None
                }
            })
            .collect();

        Ok(pids)
    }

    pub async fn is_server_open(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let processes = self.get_open_processes().await?;
        Ok(!processes.is_empty())
    }

    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "windows")]
    pub async fn stop_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let processes = match self.get_open_processes().await {
            Ok(v) => v,
            Err(_) => {
                return Ok(());
            }
        };

        for pid in processes {
            let _ = TokioCommand::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output()
                .await;
        }

        Ok(())
    }

    pub async fn run_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stop_server().await?;

        #[allow(unused_mut)]
        let mut bin_path = PathBuf::from_str("./libs/kkt")?;
        #[cfg(target_os = "windows")]
        bin_path.set_extension("exe");

        let mut cmd = Command::new(bin_path);
        cmd.args(["--type", self.config.connection_type.raw()]);
        if let Some(v) = &self.config.address {
            cmd.args(["--address", v]);
        };
        if let Some(v) = &self.config.port {
            cmd.args(["--port", &v.to_string()]);
        }
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

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
