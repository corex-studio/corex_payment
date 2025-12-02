use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KktConfig {
    pub connection_type: ConnectionType,
    pub address: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionType {
    Usb,
    Com,
    Tcp,
}

impl ConnectionType {
    pub fn raw(&self) -> &str {
        match self {
            ConnectionType::Usb => "usb",
            ConnectionType::Com => "com",
            ConnectionType::Tcp => "tcp",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellTask {
    pub taxation_type: Option<String>,
    pub electronically: bool,
    pub operator: Option<Operator>,
    pub client_info: Option<ClientInfo>,
    pub items: Vec<Item>,
    pub payments: Vec<Payment>,
    pub total: f64,
    pub taxes: Option<Vec<TaxEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftTask {
    pub operator: Operator,
    pub additional_attribute: Option<AdditionalAttribute>,
    pub address: Option<String>,
    pub payment_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellCorrectionTask {
    pub taxation_type: Option<String>,
    pub correction_type: String,
    pub correction_base_date: Option<String>,
    pub correction_base_number: Option<String>,
    pub operator: Option<Operator>,
    pub payments: Vec<Payment>,
    pub taxes: Vec<TaxEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operator {
    pub name: String,
    pub vatin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub email_or_phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    #[serde(rename = "type")]
    pub item_type: String,
    pub name: String,
    pub price: f64,
    pub quantity: f64,
    pub amount: f64,
    pub info_discount_amount: Option<f64>,
    pub department: Option<u32>,
    pub measurement_unit: u32,
    pub payment_method: Option<String>,
    pub payment_object: Option<String>,
    pub tax: Option<Tax>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tax {
    #[serde(rename = "type")]
    pub tax_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    #[serde(rename = "type")]
    pub payment_type: String,
    pub sum: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditionalAttribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxEntry {
    #[serde(rename = "type")]
    pub tax_type: String,
    pub sum: f64,
}
