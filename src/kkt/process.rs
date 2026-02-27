use super::Kkt;
use crate::{ProcessSuccess, ProcessError};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use tokio::process::Command as TokioCommand;

impl Kkt {
    pub async fn get_open_processes(&self) -> std::result::Result<ProcessSuccess<Vec<u32>>, ProcessError> {
        if cfg!(target_os = "macos") {
            let output = TokioCommand::new("pgrep")
                .arg("-f")
                .arg("kkt")
                .output()
                .await
                .map_err(|e| ProcessError::new("PROCESS_ERROR", "Не удалось получить список процессов", e.to_string()))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let pids: Vec<u32> = stdout
                .lines()
                .filter_map(|line| line.trim().parse().ok())
                .collect();
            Ok(ProcessSuccess::new("GET_PROCESSES_SUCCESSFUL", pids))
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
                .await
                .map_err(|e| ProcessError::new("PROCESS_ERROR", "Не удалось получить список процессов", e.to_string()))?;
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

            Ok(ProcessSuccess::new("GET_PROCESSES_SUCCESSFUL", pids))
        } else {
            Ok(ProcessSuccess::new("GET_PROCESSES_SUCCESSFUL", vec![]))
        }
    }

    pub async fn is_server_open(&self) -> std::result::Result<ProcessSuccess<bool>, ProcessError> {
        let processes = self.get_open_processes().await?.data;
        Ok(ProcessSuccess::new("IS_SERVER_OPEN_SUCCESSFUL", !processes.is_empty()))
    }

    pub async fn stop_server(&mut self) -> std::result::Result<ProcessSuccess<()>, ProcessError> {
        let processes = match self.get_open_processes().await {
            Ok(v) => v.data,
            Err(_) => {
                return Ok(ProcessSuccess::new("STOP_SERVER_SUCCESSFUL", ()));
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

        Ok(ProcessSuccess::new("STOP_SERVER_SUCCESSFUL", ()))
    }

    pub async fn run_server(&mut self) -> std::result::Result<ProcessSuccess<()>, ProcessError> {
        if self.is_server_open().await?.data {
            return Ok(ProcessSuccess::new("RUN_SERVER_SUCCESSFUL", ()));
        }

        #[allow(unused_mut)]
        let mut bin_path = PathBuf::from_str("./libs/kkt")
            .map_err(|e| ProcessError::new("RUN_SERVER_FAIL", "Не удалось найти путь к серверу", e.to_string()))?;
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

        self.server_process = Some(cmd.spawn().map_err(|e| ProcessError::new("RUN_SERVER_FAIL", "Ошибка запуска процесса", e.to_string()))?);
        Ok(ProcessSuccess::new("RUN_SERVER_SUCCESSFUL", ()))
    }
}
