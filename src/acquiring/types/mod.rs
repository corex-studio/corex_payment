use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ProtocolType {
    #[default]
    Ttk,
    Inpas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionType {
    Tcp,
    Usb,
    Bluetooth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub connection_type: ConnectionType,
    pub protocol: ProtocolType,
    pub serial_number: String,
    pub address: Option<String>,
    pub port: Option<u32>,
    pub timeout: Option<u32>,
    pub dc_host: Option<String>,
    // pub ncom: Option<String>,
    // pub baudrate: Option<u32>,
    pub sc552_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedTransactionData {
    pub message_id: Option<String>,
    pub operation_code: Option<String>,
    pub ecr_number: Option<String>,
    pub response_code: Option<String>,
    pub approve: Option<String>,
    pub status: Option<String>,
    pub status_text: Option<String>,
    pub amount: Option<String>,
    pub additional_amount: Option<String>,
    pub currency: Option<String>,
    pub pan_masked: Option<String>,
    pub rrn: Option<String>,
    pub invoice_number: Option<String>,
    pub authorization_code: Option<String>,
    pub terminal_id: Option<String>,
    pub merchant_id: Option<String>,
    pub batch_number: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
    pub timestamp: Option<String>,
    pub host_timestamp: Option<String>,
    pub card_entry_mode: Option<String>,
    pub cardholder_verification: Option<String>,
    pub text_response: Option<String>,
    pub receipt: Option<String>,
    pub application_label: Option<String>,
    pub issuer_name: Option<String>,
    pub transaction_id: Option<String>,
    pub cashier_request: Option<String>,
    pub cashier_response: Option<String>,
    pub provider_code: Option<String>,
    #[serde(flatten)]
    pub raw: std::collections::HashMap<String, String>,
    pub extras: Option<std::collections::HashMap<String, String>>,
}

impl NormalizedTransactionData {
    pub fn empty() -> Self {
        Self {
            message_id: None,
            operation_code: None,
            ecr_number: None,
            response_code: None,
            approve: None,
            status: None,
            status_text: None,
            amount: None,
            additional_amount: None,
            currency: None,
            pan_masked: None,
            rrn: None,
            invoice_number: None,
            authorization_code: None,
            terminal_id: None,
            merchant_id: None,
            batch_number: None,
            date: None,
            time: None,
            timestamp: None,
            host_timestamp: None,
            card_entry_mode: None,
            cardholder_verification: None,
            text_response: None,
            receipt: None,
            application_label: None,
            issuer_name: None,
            transaction_id: None,
            cashier_request: None,
            cashier_response: None,
            provider_code: None,
            raw: HashMap::new(),
            extras: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalResponse {
    pub success: bool,
    pub code: Option<String>,
    pub message: Option<String>,
    pub data: Option<NormalizedTransactionData>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TagDefinition {
    pub tag: u32,
    pub name: String,
    pub data_type: DataType,
    pub encoding: Option<Encoding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    String,
    Bcd,
    Hex,
    Binary,
    DwordLe,
    DwordBe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Cp1251,
    Cp866,
    Ascii,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    ClientRequest = 0x96f2,
    ServerResponse = 0x97f2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseCode {
    Success,
    InvalidFormat,
    InvalidDocumentNumber,
}

impl ResponseCode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "00" => Some(Self::Success),
            "FE" => Some(Self::InvalidFormat),
            "B4" => Some(Self::InvalidDocumentNumber),
            _ => None,
        }
    }
}

pub mod protocol;

pub use protocol::get_tag_definition;
