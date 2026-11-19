use crate::{
    ConnectionType,
    acquiring::protocol::{InpasAdapter, SBAdapter},
    healthcheck::{CheckUnit, Healthchecker},
    utils::check_port::is_device_connected,
};
use anyhow::anyhow;
use async_trait::async_trait;
use std::time::Duration;
use tokio::{net::TcpStream, time::timeout};

const TCP_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

#[async_trait]
impl Healthchecker for SBAdapter {
    async fn check_port(&self) -> CheckUnit {
        if self.config.connection_type == ConnectionType::Bluetooth {
            return CheckUnit::Error(
                "Bluetooth не поддерживается для терминала Сбера".into(),
                None,
            );
        }

        let address = match self.config.address.as_deref() {
            Some(v) if !v.trim().is_empty() => v,
            _ => {
                return CheckUnit::Error(
                    "Адрес терминала не сконфигурирован".to_string(),
                    Some(anyhow!("Terminal address not configured")),
                );
            }
        };

        if self.config.connection_type == ConnectionType::Tcp {
            let Ok(port @ 1..) = u16::try_from(self.tcp_port()) else {
                return CheckUnit::Error(
                    "TCP-порт терминала должен быть от 1 до 65535".into(),
                    None,
                );
            };
            return match timeout(TCP_CHECK_TIMEOUT, TcpStream::connect((address, port))).await {
                Ok(Ok(_)) => CheckUnit::Success,
                Ok(Err(e)) if e.kind() != std::io::ErrorKind::TimedOut => CheckUnit::Error(
                    format!("Не удалось установить TCP-соединение с {address}:{port}"),
                    Some(e.into()),
                ),
                _ => CheckUnit::Error(
                    format!("Истекло время ожидания TCP-соединения с {address}:{port}"),
                    None,
                ),
            };
        }
        let port_connected = is_device_connected(address).await;
        match port_connected {
            Ok(true) => CheckUnit::Success,
            Ok(false) => {
                CheckUnit::Error(format!("Терминал не подключен к порту {}", address), None)
            }
            Err(e) => CheckUnit::Error(
                "Ошибка при проверке порта терминала".to_string(),
                Some(anyhow!("Terminal port check error: {}", e)),
            ),
        }
    }

    fn check_drivers(&self) -> CheckUnit {
        if !self.dir.join("sb_pilot.exe").is_file() {
            return CheckUnit::Error(
                "Отсутствует исполняемый файл Сбербанка".to_string(),
                Some(anyhow!("Missing sb_pilot.exe")),
            );
        }

        let Ok(path) = self.get_pinpad_ini() else {
            return CheckUnit::Error(
                "Отсутствует файл конфигурации Сбербанка".to_string(),
                Some(anyhow!("Missing pinpad.ini")),
            );
        };
        match self.validate_pinpad_ini(&path) {
            Ok(()) => CheckUnit::Success,
            Err(e) => CheckUnit::Error(format!("Ошибка проверки pinpad.ini: {e}"), Some(e)),
        }
    }
}

#[async_trait]
impl Healthchecker for InpasAdapter {
    async fn check_port(&self) -> CheckUnit {
        if self.config.dc_host.is_none() {
            return CheckUnit::Error("Missing DualConnector host address".to_string(), None);
        }
        if self.config.serial_number.is_empty() {
            return CheckUnit::Error("Missing terminal serial number".to_string(), None);
        }

        match self.config.connection_type {
            ConnectionType::Tcp | ConnectionType::Usb => {
                let has_address = self.config.address.is_some();
                let has_port = self.config.port.is_some();
                if has_address && has_port {
                    CheckUnit::Success
                } else {
                    CheckUnit::Error("Terminal address or port are missing".to_string(), None)
                }
            }
            _ => CheckUnit::Error("Unimplemented connection type".to_string(), None),
        }
    }

    fn check_drivers(&self) -> CheckUnit {
        CheckUnit::Success
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectionConfig, ProtocolType, acquiring::protocol::base::Acquiring};
    use std::{fs, path::PathBuf};
    use tokio::net::TcpSocket;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let dir =
                std::env::temp_dir().join(format!("corex-health-{:032x}", rand::random::<u128>()));
            fs::create_dir(&dir).unwrap();
            fs::write(dir.join("sb_pilot.exe"), "").unwrap();
            fs::write(dir.join("pinpad.ini"), "").unwrap();
            Self(dir)
        }

        fn adapter(
            &self,
            connection_type: ConnectionType,
            address: &str,
            port: Option<u32>,
        ) -> SBAdapter {
            SBAdapter::new(
                ConnectionConfig {
                    connection_type,
                    address: Some(address.into()),
                    port,
                    protocol: ProtocolType::Sb,
                    serial_number: String::new(),
                    sc552_path: None,
                    dc_host: None,
                    timeout: None,
                },
                self.0.clone(),
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn tcp_healthcheck_requires_listener_and_matching_ini() {
        let socket = TcpSocket::new_v4().unwrap();
        socket.bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let port = socket.local_addr().unwrap().port();
        let fixture = Fixture::new();
        let mut adapter = fixture.adapter(ConnectionType::Tcp, "127.0.0.1", Some(port.into()));
        adapter.connect().await.unwrap();
        let refused = adapter.healthcheck().await.unwrap().data;
        assert!(!refused.success);
        assert!(refused.message.unwrap().contains("TCP"));
        let _listener = socket.listen(8).unwrap();
        assert!(adapter.healthcheck().await.unwrap().data.success);
        let ini = fixture.0.join("pinpad.ini");
        let settings = fs::read_to_string(&ini).unwrap();
        fs::write(&ini, settings.replace("127.0.0.1", "127.0.0.2")).unwrap();
        let mismatch = adapter.healthcheck().await.unwrap().data;
        assert!(!mismatch.success);
        assert!(mismatch.message.unwrap().contains("PinpadIPAddr"));
    }

    #[tokio::test]
    async fn usb_settings_follow_connect_and_ignore_commented_keys() {
        let fixture = Fixture::new();
        let mut adapter = fixture.adapter(ConnectionType::Usb, "COM12", None);
        adapter.connect().await.unwrap();
        assert!(adapter.check_drivers().is_success());
        let ini = fixture.0.join("pinpad.ini");
        let settings = fs::read_to_string(&ini).unwrap();
        for (old, new) in [
            ("ComPort=12", ";ComPort=12"),
            ("Speed=9600", "Speed=19200"),
            ("EnableUSB=1", "EnableUSB=0"),
            ("ComPort=12", "ComPort=12\nPinpadIPPort=1234"),
        ] {
            fs::write(&ini, settings.replace(old, new)).unwrap();
            assert!(adapter.check_drivers().is_error(), "{new}");
        }
        fs::write(&ini, [0xff]).unwrap();
        assert!(adapter.check_drivers().is_error());
    }

    #[tokio::test]
    async fn rejects_invalid_tcp_configuration_and_bluetooth() {
        let fixture = Fixture::new();
        for (address, port) in [
            ("", None),
            ("127.0.0.1", Some(0)),
            ("127.0.0.1", Some(65536)),
        ] {
            assert!(
                fixture
                    .adapter(ConnectionType::Tcp, address, port)
                    .check_port()
                    .await
                    .is_error()
            );
        }
        let result = fixture
            .adapter(ConnectionType::Bluetooth, "COM12", None)
            .healthcheck()
            .await
            .unwrap();
        assert!(!result.data.success);
        assert!(result.data.message.unwrap().contains("Bluetooth"));
    }
}
