use crate::{
    ConnectionType,
    acquiring::protocol::{InpasAdapter, SBAdapter},
    healthcheck::{CheckUnit, Healthchecker},
    utils::check_port::is_device_connected,
};
use anyhow::anyhow;
use async_trait::async_trait;

#[async_trait]
impl Healthchecker for SBAdapter {
    async fn check_port(&self) -> CheckUnit {
        if matches!(self.config.connection_type, ConnectionType::Tcp) {
            return CheckUnit::Success;
        }

        let port = match &self.config.address {
            Some(v) => v.clone(),
            None => {
                return CheckUnit::Error(
                    "Адрес терминала не сконфигурирован".to_string(),
                    Some(anyhow!("Terminal address not configured")),
                );
            }
        };

        let port_connected = is_device_connected(&port).await;
        match port_connected {
            Ok(true) => CheckUnit::Success,
            Ok(false) => CheckUnit::Error(format!("Терминал не подключен к порту {}", port), None),
            Err(e) => CheckUnit::Error(
                "Ошибка при проверке порта терминала".to_string(),
                Some(anyhow!("Terminal port check error: {}", e)),
            ),
        }
    }

    fn check_drivers(&self) -> CheckUnit {
        if self.get_pilot().is_err() {
            return CheckUnit::Error(
                "Отсутствует исполняемый файл Сбербанка".to_string(),
                Some(anyhow!("Missing sb_pilot.exe")),
            );
        }

        if self.get_pinpad_ini().is_err() {
            return CheckUnit::Error(
                "Отсутствует файл конфигурации Сбербанка".to_string(),
                Some(anyhow!("Missing pinpad.ini")),
            );
        }

        CheckUnit::Success
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
