use std::{path::PathBuf, str::FromStr, time::Duration};

use super::ConnectionType;
use crate::{
    Kkt,
    healthcheck::{CheckUnit, Healthchecker},
    utils::check_port::is_device_connected,
};
use anyhow::anyhow;
use async_trait::async_trait;

#[async_trait]
impl Healthchecker for Kkt {
    async fn check_port(&self) -> CheckUnit {
        let port = match &self.config.address {
            Some(v) => v.clone(),
            None => {
                return CheckUnit::Error(
                    "Адрес ККТ не сконфигурирован".to_string(),
                    Some(anyhow!("KKT address not configured")),
                );
            }
        };

        if self.config.connection_type == ConnectionType::Tcp {
            let tcp_port = match self.config.port {
                Some(v) => v,
                None => {
                    return CheckUnit::Error(
                        "Порт ККТ не сконфигурирован".to_string(),
                        Some(anyhow!("KKT port not configured")),
                    );
                }
            };

            // kkt.exe owns the device connection; do not open a second TCP session.
            return match tokio::time::timeout(Duration::from_secs(2), self.check_connection()).await
            {
                Ok(Ok(result)) if result.data.is_connected => CheckUnit::Success,
                Ok(Ok(_)) => CheckUnit::Error(
                    format!("ККТ не подключен по TCP к {}:{}", port, tcp_port),
                    None,
                ),
                Ok(Err(e)) => CheckUnit::Error(
                    "Ошибка при проверке TCP-подключения ККТ через KKT-сервер".to_string(),
                    Some(anyhow!(e)),
                ),
                Err(e) => CheckUnit::Error(
                    "Превышено время ожидания проверки TCP-подключения ККТ".to_string(),
                    Some(anyhow!(e)),
                ),
            };
        }

        let port_connected = is_device_connected(&port).await;
        match port_connected {
            Ok(true) => CheckUnit::Success,
            Ok(false) => CheckUnit::Error(format!("ККТ не подключен к порту {}", port), None),
            Err(e) => CheckUnit::Error(
                "Ошибка при проверке порта".to_string(),
                Some(anyhow!("Port check error: {}", e)),
            ),
        }
    }

    fn check_drivers(&self) -> CheckUnit {
        let libs_dir = match PathBuf::from_str("./libs") {
            Ok(v) => v,
            Err(e) => {
                return CheckUnit::Error(
                    "Не удалось проверить папку с драйверами".to_string(),
                    Some(anyhow!(e)),
                );
            }
        };

        let kkt_exe = {
            let mut filename = libs_dir.join("kkt");
            if cfg!(target_os = "windows") {
                filename.set_extension("exe");
            }
            filename
        };

        if !kkt_exe.exists() {
            return CheckUnit::Error("Отсутствует исполняемый файл KKT".to_string(), None);
        }

        let os_name = {
            if cfg!(target_os = "windows") {
                "windows"
            } else if cfg!(target_os = "macos") {
                "darwin"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                "unknown"
            }
        };

        if !libs_dir.join(os_name).exists() {
            return CheckUnit::Error(format!("Отсутствует драйвер KKT для {}", os_name), None);
        }

        CheckUnit::Success
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KktConfig;

    #[tokio::test]
    async fn tcp_requires_configured_address_and_port() {
        for (address, port, expected) in [
            (None, Some(5555), "Адрес ККТ не сконфигурирован"),
            (
                Some("192.0.2.1".to_string()),
                None,
                "Порт ККТ не сконфигурирован",
            ),
        ] {
            let kkt = Kkt::new(KktConfig {
                connection_type: ConnectionType::Tcp,
                address,
                port,
            });
            match kkt.check_port().await {
                CheckUnit::Error(message, _) => assert_eq!(message, expected),
                CheckUnit::Success => panic!("TCP configuration must be checked"),
            }
        }
    }
}
