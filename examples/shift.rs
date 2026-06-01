use corex_payment::{Kkt, KktConfig, KktConnectionType, Operator, SellTask};
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let config = KktConfig {
        connection_type: KktConnectionType::Com,
        address: None,
        port: None,
    };

    let mut fiscal = Kkt::new(config);
    // let result = fiscal.run_server().await;
    // match result {
    //     Ok(_) => println!("YES OK OK HUYOK"),
    //     Err(e) => println!("NO NO NO {e:?}"),
    // }
    // println!("Run!!!");
    // thread::sleep(Duration::from_secs(3));
    // println!("Slept");

    let operator = Operator {
        name: "Степан".to_string(),
        vatin: None,
    };

    let sell_task = SellTask {
        taxation_type: None,
        electronically: false,
        operator: Some(operator),
        client_info: None,
        items: vec![],
        payments: vec![],
        taxes: None,
        total: -10000.0,
    };

    let response = fiscal.payment(&sell_task).await;
    match response {
        Ok(v) => println!("{v:?}"),
        Err(e) => println!("Error: {e:?}"),
    }

    // let _ = fiscal.stop_server().await;
}
//
// fn sync_fun() {
//     println!("ZOPAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa")
// }
//
// async fn start() {
//     println!("Start");
//     let config = KktConfig {
//         connection_type: KktConnectionType::Usb,
//         address: Some(String::from("asdf")),
//         port: Some(123),
//     };
//     println!("conf asdfasdf");
//     let mut fiscal = Kkt::new(config);
//     let result = fiscal.run_server().await;
//     match result {
//         Ok(_) => println!("YES OK OK HUYOK"),
//         Err(e) => println!("NO NO NO {e:?}"),
//     }
//     println!("Done!!!")
//     // match result {
//     //     Ok()
//     // }
//     // if result.is_ok() {
//     //     println!("Result: {}", result.try_into)
//     // }
// }
