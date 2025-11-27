use corex_payment::{ConnectionConfig, ConnectionType, ProtocolType, Terminal};

#[tokio::main]
async fn main() {
    let mut term = Terminal::new(ConnectionConfig {
        protocol: ProtocolType::Inpas,
        serial_number: String::from("10285694"),
        connection_type: ConnectionType::Usb,
        dc_host: Some(String::from("192.168.39.176:9015")),
        ncom: Some(String::from("COM15")),
        baudrate: Some(9600),
        address: None,
        port: None,
        timeout: Some(10000),
    });

    let con = term.connect().await;
    match con {
        Ok(_) => println!(""),
        Err(e) => {
            println!("Error while connecting: {e:?}");
            return;
        }
    }

    let result = term.payment(1000, None).await;
    match result {
        Ok(v) => println!("Success: {v:?}"),
        Err(e) => println!("Error: ${e:?}"),
    }

    let _ = term.disconnect();
}
