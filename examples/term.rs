use corex_payment::{ConnectionConfig, Terminal};

#[tokio::main]
async fn main() {
    let mut term = Terminal::new(ConnectionConfig {
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
    });

    term.connect().await.unwrap();
    term.payment(100, None).await.unwrap();
    let res = term.payment(100, None).await;
    match res {
        Ok(_) => println!("Good"),
        Err(e) => eprintln!("Err: {}", e),
    };
}
