use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ProtocolType {
    #[default]
    Ttk,
    Inpas,
    Sb,
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
    pub sc552_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedTransactionData {
    pub status_name: Option<String>,
    pub amount: Option<f64>,
    pub card_masked_pan: Option<String>,
    pub invoice_number: Option<String>,
    pub authorization_code: Option<String>,
    pub terminal_id: Option<String>,
    pub merchant_id: Option<String>,
    pub timestamp: Option<String>,
    pub host_timestamp: Option<String>,
    pub issuer_name: Option<String>,
    pub trx_id: Option<String>,
    #[serde(flatten)]
    pub raw: std::collections::HashMap<String, String>,
}

impl NormalizedTransactionData {
    pub fn empty() -> Self {
        Self {
            amount: None,
            invoice_number: None,
            status_name: None,
            card_masked_pan: None,
            authorization_code: None,
            terminal_id: None,
            merchant_id: None,
            timestamp: None,
            host_timestamp: None,
            issuer_name: None,
            trx_id: None,
            raw: HashMap::new(),
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
