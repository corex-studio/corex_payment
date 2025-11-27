use corex_payment::{Item, Kkt, KktConfig, KktConnectionType, Operator, Payment, SellTask, Tax};
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
        name: "asdf".to_string(),
        vatin: None,
    };

    let sell_task = SellTask {
        taxation_type: Some("osn".to_string()),
        electronically: true,
        operator: Some(operator),
        client_info: None,
        items: vec![Item {
            item_type: "position".to_string(),
            name: "Товар 1".to_string(),
            price: 100.0,
            quantity: 1.0,
            amount: 100.0,
            info_discount_amount: None,
            department: Some(1),
            measurement_unit: 0,
            payment_method: None,
            payment_object: None,
            tax: Some(Tax {
                tax_type: "vat20".to_string(),
            }),
        }],
        payments: vec![Payment {
            payment_type: "electronically".to_string(),
            sum: 100.0,
        }],
        total: 100.0,
        taxes: None,
    };

    let response = fiscal.payment(&sell_task).await;
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
