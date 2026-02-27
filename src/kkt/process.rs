use super::Kkt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use tokio::process::Command as TokioCommand;

impl Kkt {
    pub async fn get_open_processes(&self) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        if cfg!(target_os = "macos") {
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
        } else if cfg!(target_os = "windows") {
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
        } else {
            Ok(vec![])
        }
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

        if cfg!(target_os = "macos") {
            for pid in processes {
                let _ = TokioCommand::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .output()
                    .await;
            }
        } else if cfg!(target_os = "windows") {
            for pid in processes {
                let _ = TokioCommand::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .output()
                    .await;
            }
        }

        Ok(())
    }

    pub async fn run_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_server_open().await? {
            return Ok(());
        }

        #[allow(unused_mut)]
        let mut bin_path = PathBuf::from_str("./libs/kkt")?;
        if cfg!(target_os = "windows") {
            bin_path.set_extension("exe");
        }

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
}
