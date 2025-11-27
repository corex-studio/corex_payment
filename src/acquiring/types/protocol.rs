use super::{DataType, Encoding, TagDefinition};
use once_cell::sync::Lazy;

pub static TAG_DEFINITIONS: Lazy<Vec<(&'static str, TagDefinition)>> = Lazy::new(|| {
  vec![
    (
      "MESSAGE_ID",
      TagDefinition {
        tag: 0x01,
        name: "Message ID".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "ECR_NUMBER",
      TagDefinition {
        tag: 0x02,
        name: "ECR Number".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "ERN",
      TagDefinition {
        tag: 0x03,
        name: "ERN".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "TRANSACTION_AMOUNT",
      TagDefinition {
        tag: 0x04,
        name: "Transaction Amount".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "INVOICE_NUMBER",
      TagDefinition {
        tag: 0x0b,
        name: "Invoice Number".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "AUTHORIZATION_ID",
      TagDefinition {
        tag: 0x0c,
        name: "Authorization ID".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "SRV_SUBFUNCTION",
      TagDefinition {
        tag: 0x1a,
        name: "SRV Subfunction".to_string(),
        data_type: DataType::Hex,
        encoding: None,
      },
    ),
    (
      "CURRENCY",
      TagDefinition {
        tag: 0x1b,
        name: "Currency".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "INPUT_CODE",
      TagDefinition {
        tag: 0x1f00,
        name: "Input Code".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "INPUT_DATA",
      TagDefinition {
        tag: 0x1f01,
        name: "Input Data".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "SERVER_MESSAGE_ID",
      TagDefinition {
        tag: 0x81,
        name: "Message ID".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "SERVER_ECR_NUMBER",
      TagDefinition {
        tag: 0x82,
        name: "ECR Number".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "SERVER_ERN",
      TagDefinition {
        tag: 0x83,
        name: "ERN".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "RESPONSE_CODE",
      TagDefinition {
        tag: 0x9b,
        name: "Response Code".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "SERVER_TRANSACTION_AMOUNT",
      TagDefinition {
        tag: 0x84,
        name: "Transaction Amount".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "PAN",
      TagDefinition {
        tag: 0x89,
        name: "PAN".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "SERVER_INVOICE_NUMBER",
      TagDefinition {
        tag: 0x8b,
        name: "Invoice Number".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "SERVER_AUTHORIZATION_ID",
      TagDefinition {
        tag: 0x8c,
        name: "Authorization ID".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "DATE",
      TagDefinition {
        tag: 0x8d,
        name: "Date".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "TIME",
      TagDefinition {
        tag: 0x8e,
        name: "Time".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "ISSUER_NAME",
      TagDefinition {
        tag: 0x8f,
        name: "Issuer Name".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "MERCHANT_NO",
      TagDefinition {
        tag: 0x90,
        name: "Merchant No".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "PROCESSING_CODE",
      TagDefinition {
        tag: 0x91,
        name: "Processing Code".to_string(),
        data_type: DataType::Hex,
        encoding: None,
      },
    ),
    (
      "POS_ENTRY_MODE",
      TagDefinition {
        tag: 0x92,
        name: "POS Entry Mode".to_string(),
        data_type: DataType::Hex,
        encoding: None,
      },
    ),
    (
      "POS_CONDITION_CODE",
      TagDefinition {
        tag: 0x93,
        name: "POS Condition Code".to_string(),
        data_type: DataType::Hex,
        encoding: None,
      },
    ),
    (
      "CARDHOLDER_VERIFICATION",
      TagDefinition {
        tag: 0x94,
        name: "Cardholder Verification".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "TVR",
      TagDefinition {
        tag: 0x95,
        name: "TVR".to_string(),
        data_type: DataType::Hex,
        encoding: None,
      },
    ),
    (
      "RRN",
      TagDefinition {
        tag: 0x98,
        name: "RRN".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "BATCH_NO",
      TagDefinition {
        tag: 0x99,
        name: "Batch No".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "RECEIPT",
      TagDefinition {
        tag: 0x9c,
        name: "Receipt".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "TERMINAL_ID",
      TagDefinition {
        tag: 0x9d,
        name: "Terminal ID".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "RECEIPT_PDS",
      TagDefinition {
        tag: 0x9e,
        name: "Receipt PDS".to_string(),
        data_type: DataType::Binary,
        encoding: None,
      },
    ),
    (
      "RECEIPT_SECOND_PDS",
      TagDefinition {
        tag: 0x9f0e,
        name: "Receipt Second PDS".to_string(),
        data_type: DataType::Binary,
        encoding: None,
      },
    ),
    (
      "APPLICATION_ID",
      TagDefinition {
        tag: 0x9f06,
        name: "Application ID".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "TC",
      TagDefinition {
        tag: 0x9f26,
        name: "TC".to_string(),
        data_type: DataType::Hex,
        encoding: None,
      },
    ),
    (
      "VISUAL_HOST_RESPONSE",
      TagDefinition {
        tag: 0xa0,
        name: "Visual Host Response".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "APPROVE",
      TagDefinition {
        tag: 0xa1,
        name: "Approve".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
    (
      "TRANSACTION_AMOUNT_2",
      TagDefinition {
        tag: 0xa2,
        name: "Transaction Amount #2".to_string(),
        data_type: DataType::Bcd,
        encoding: None,
      },
    ),
    (
      "APPLICATION_LABEL",
      TagDefinition {
        tag: 0x50,
        name: "Application Label".to_string(),
        data_type: DataType::String,
        encoding: Some(Encoding::Ascii),
      },
    ),
  ]
});

pub fn get_tag_definition(tag: u32) -> Option<&'static TagDefinition> {
  TAG_DEFINITIONS
    .iter()
    .find(|(_, def)| def.tag == tag)
    .map(|(_, def)| def)
}

pub const MESSAGE_IDS: &[(&str, &str)] = &[
  ("PUR", "PUR"),
  ("REF", "REF"),
  ("VOI", "VOI"),
  ("JRN", "JRN"),
  ("AUH", "AUH"),
  ("AUT", "AUT"),
  ("CMP", "CMP"),
  ("CSH", "CSH"),
  ("CRE", "CRE"),
  ("BAL", "BAL"),
  ("SRV", "SRV"),
  ("INF", "INF"),
  ("DLG", "DLG"),
];

pub const SERVICE_OPERATIONS: &[(&str, &str)] = &[
  ("TOTALS", "2"),
  ("TEST_SERVER", "3"),
  ("TEST_HOST", "4"),
  ("PRINT_REPORTS", "5"),
  ("CALL_MENU", "C"),
];
