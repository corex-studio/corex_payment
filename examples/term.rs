use corex_payment::{ConnectionConfig, Terminal};

#[tokio::main]
async fn main() {
    let mut term = Terminal::new(ConnectionConfig {
        connection_type: corex_payment::ConnectionType::Tcp,
        protocol: corex_payment::ProtocolType::Ttk,
        serial_number: "30413601".to_string(),
        // address: Some("100.85.201.101".to_string()),
        address: Some("192.168.39.43".to_string()),
        port: Some(8888),
        timeout: Some(30000),
        dc_host: None,
        ncom: None,
        baudrate: None,
    });

    let connected = term.connect().await.unwrap();
    println!("Connected: {}, {}", connected, term.connected());

    // term.totals().await.unwrap();
    term.payment(100, None).await.unwrap();
    let res = term.payment(100, None).await;
    match res {
        Ok(_) => println!("Good"),
        Err(e) => eprintln!("Err: {}", e),
    };
}
