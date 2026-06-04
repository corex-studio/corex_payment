use crate::acquiring::{
    protocol::inpas::inpas_prop_codes,
    types::{NormalizedTransactionData, ProtocolType},
};
use std::collections::HashMap;

pub fn normalize_terminal_response(
    protocol: ProtocolType,
    raw: &HashMap<String, String>,
) -> NormalizedTransactionData {
    match protocol {
        ProtocolType::Inpas => normalize_inpas(raw),
        ProtocolType::Ttk => normalize_sb(raw),
        ProtocolType::Sb => normalize_sb(raw),
    }
}

fn normalize_sb(raw: &HashMap<String, String>) -> NormalizedTransactionData {
    let status = raw
        .get("result_code")
        .cloned()
        .unwrap_or("".to_string())
        .trim()
        .to_string();
    let is_approved = matches!(status.as_str(), "0");

    let timestamp = match raw.get("datetime") {
        Some(v) => v.trim().parse::<i64>().ok(),
        _ => None,
    };
    let host_timestamp = Some(chrono::Utc::now().timestamp_millis());

    NormalizedTransactionData {
        raw: raw.clone(),
        amount: None,
        is_approved: Some(is_approved),
        status_name: Some(status),
        card_masked_pan: raw.get("masked_pan").cloned(),
        invoice_number: None,
        authorization_code: raw.get("auth_code").cloned(),
        terminal_id: raw.get("terminal_id").cloned(),
        merchant_id: raw.get("merchant_id").cloned(),
        timestamp,
        host_timestamp,
        issuer_name: None,
        trx_id: None,
    }
}

fn normalize_inpas(raw: &HashMap<String, String>) -> NormalizedTransactionData {
    let status = raw
        .get(inpas_prop_codes::STATUS)
        .cloned()
        .unwrap_or("".to_string())
        .trim()
        .to_string();
    let is_approved = matches!(status.as_str(), "1");

    let timestamp = match raw.get(inpas_prop_codes::TERMINAL_DATETIME) {
        Some(v) => v.trim().parse::<i64>().ok(),
        _ => None,
    };
    let host_timestamp = Some(chrono::Utc::now().timestamp_millis());

    NormalizedTransactionData {
        raw: raw.clone(),
        amount: raw
            .get(inpas_prop_codes::AMOUNT)
            .map(|v| v.parse::<f64>().unwrap_or(0.0)),
        card_masked_pan: raw.get(inpas_prop_codes::PAN).cloned(),
        is_approved: Some(is_approved),
        status_name: Some(status),
        timestamp,
        host_timestamp,
        authorization_code: raw.get(inpas_prop_codes::AUTHORIZATION_CODE).cloned(),
        invoice_number: raw.get(inpas_prop_codes::TERMINAL_TRX_ID).cloned(),
        terminal_id: raw.get(inpas_prop_codes::TERMINAL_ID).cloned(),
        merchant_id: raw.get(inpas_prop_codes::MERCHANT_ID).cloned(),
        trx_id: raw.get(inpas_prop_codes::TRXID).cloned(),
        issuer_name: None,
    }
}
