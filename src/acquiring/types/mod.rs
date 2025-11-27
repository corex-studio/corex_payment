use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
  Ttk,
  Inpas,
}

impl Default for ProtocolType {
  fn default() -> Self {
    Self::Ttk
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
  Tcp,
  Usb,
  Bluetooth,
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
  pub connection_type: ConnectionType,
  pub protocol: ProtocolType,
  pub serial_number: String,
  pub address: Option<String>,
  pub port: Option<u16>,
  pub timeout: Option<u32>,
  pub dc_host: Option<String>,
  pub ncom: Option<String>,
  pub baudrate: Option<u32>,
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
