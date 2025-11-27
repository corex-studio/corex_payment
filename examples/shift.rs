use corex_payment::{Kkt, KktConfig, KktConnectionType, Operator};
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let config = KktConfig {
        connection_type: KktConnectionType::Usb,
        address: Some(String::from("asdf")),
        port: Some(123),
    };

    let mut fiscal = Kkt::new(config);
    let result = fiscal.run_server().await;
    match result {
        Ok(_) => println!("YES OK OK HUYOK"),
        Err(e) => println!("NO NO NO {e:?}"),
    }
    println!("Run!!!");
    thread::sleep(Duration::from_secs(3));
    println!("Slept");

    let operator = Operator {
        name: "Степан".to_string(),
        vatin: None,
    };

    let response = fiscal.open_shift(&operator).await;
    match response {
        Ok(v) => println!("Successful: {v:?}"),
        Err(e) => println!("Error: {e:?}"),
    }

    let _ = fiscal.stop_server().await;
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
