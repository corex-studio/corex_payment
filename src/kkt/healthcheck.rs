use std::{path::PathBuf, str::FromStr};

use crate::{
    KktConfig,
    healthcheck::{CheckUnit, HealthcheckResult},
    utils::check_port::is_device_connected,
};
use anyhow::anyhow;

pub async fn healthcheck(config: KktConfig) -> HealthcheckResult {
    let port_connected = check_port(config).await;
    let drivers_ready = check_drivers();

    let success = port_connected.is_success() && drivers_ready.is_success();
    if success {
        return HealthcheckResult::success();
    }

    let mut message: String = String::new();
    let mut details: Vec<String> = Vec::new();

    if let CheckUnit::Error(m, e) = port_connected {
        message = format!("{}\n", m);
        if let Some(err) = e {
            details.push(format!("{}", err));
        }
    }
    if let CheckUnit::Error(m, e) = drivers_ready {
        message = format!("{}\n", m);
        if let Some(err) = e {
            details.push(format!("{}", err));
        }
    }

    HealthcheckResult::error(message, details)
}

async fn check_port(config: KktConfig) -> CheckUnit {
    let port = match config.address {
        Some(v) => v,
        None => {
            return CheckUnit::Error(
                "Адрес ККТ не сконфигурирован".to_string(),
                Some(anyhow!("KKT address not configured")),
            );
        }
    };

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

fn check_drivers() -> CheckUnit {
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
