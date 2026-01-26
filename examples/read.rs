use std::{path::PathBuf, str::FromStr};

use corex_payment::{ConnectionConfig, acquiring::protocol::SBAdapter};

fn main() {
    let config = ConnectionConfig {
        connection_type: corex_payment::ConnectionType::Usb,
        protocol: corex_payment::ProtocolType::Ttk,
        serial_number: "30413601".to_string(),
        // address: Some("192.168.39.43".to_string()),
        address: None,
        // port: Some(8888),
        port: None,
        timeout: Some(30000),
        dc_host: None,
        ncom: Some("COM6".to_string()),
        baudrate: Some(115_600),
    };
    let sc552 = PathBuf::from_str("libs/sc552/").unwrap_or_else(|_| {
        let mut p = PathBuf::new();
        p.push("C:/");
        p.push("sc552/");
        p
    });
    let adapter = SBAdapter::new(config.clone(), sc552);

    dbg!(adapter.read_e().unwrap());
}
