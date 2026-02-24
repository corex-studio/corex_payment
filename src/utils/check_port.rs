use thiserror::Error;

#[derive(Debug, Error)]
pub enum PortCheckError {
    #[error("Failed to enumerate serial ports: {0}")]
    Enumeration(#[from] serialport::Error),

    #[error("Blocking task panicked")]
    Join(#[from] tokio::task::JoinError),
}

pub async fn is_device_connected(port_name: &str) -> Result<bool, PortCheckError> {
    let port_name = port_name.to_string();

    tokio::task::spawn_blocking(move || {
        // available_ports() — блокирующий syscall, поэтому внутри spawn_blocking
        let ports = serialport::available_ports()?;

        let connected = ports.iter().any(|info| {
            #[cfg(windows)]
            {
                // На Windows порт может быть "COM5" или "\\.\COM10" — нормализуем оба
                let available = info
                    .port_name
                    .strip_prefix(r"\\.\")
                    .unwrap_or(&info.port_name);
                let requested = port_name.strip_prefix(r"\\.\").unwrap_or(&port_name);

                // Регистронезависимо: "com5" == "COM5"
                available.eq_ignore_ascii_case(requested)
            }
            #[cfg(not(windows))]
            {
                // Linux/macOS: точное совпадение пути "/dev/ttyUSB1"
                info.port_name == port_name
            }
        });

        Ok(connected)
    })
    // .await -> Result<Result<bool, PortCheckError>, JoinError>
    // первый `?`  -> если поток запаниковал, возвращаем PortCheckError::Join
    // результат  -> Result<bool, PortCheckError>
    .await?
}
