use std::{path::PathBuf, str::FromStr};

use corex_payment::{
    ConnectionConfig, ProtocolType,
    acquiring::{protocol::SBAdapter, response::normalize_terminal_response},
};

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
        sc552_path: None,
    };
    let sc552 = PathBuf::from_str("libs/sc552/").unwrap_or_else(|_| {
        let mut p = PathBuf::new();
        p.push("C:/");
        p.push("sc552/");
        p
    });
    let adapter = SBAdapter::new(config.clone(), sc552);

    let e_data = adapter.read_e().unwrap();
    let norm_data = normalize_terminal_response(ProtocolType::Ttk, &e_data.as_hash_map());
    dbg!(norm_data);
}
