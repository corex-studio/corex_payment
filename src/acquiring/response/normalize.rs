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
        ProtocolType::Ttk => normalize_ttk(raw),
    }
}

fn normalize_ttk(raw: &HashMap<String, String>) -> NormalizedTransactionData {
    NormalizedTransactionData {
        raw: raw.clone(),
        amount: raw
            .get("Transaction Amount")
            .map(|v| v.parse::<f64>().unwrap_or(0.0)),
        status_name: raw.get("Status name").cloned(),
        card_masked_pan: raw.get("PAN").cloned(),
        invoice_number: raw.get("Invoice Number").cloned(),
        authorization_code: raw.get("Authorization ID").cloned(),
        terminal_id: raw.get("Terminal ID").cloned(),
        merchant_id: raw.get("Merchant No").cloned(),
        timestamp: build_ttk_timestamp(raw.get("Date"), raw.get("Time")),
        issuer_name: raw.get("Issuer Name").cloned(),
        host_timestamp: None,
        trx_id: raw.get("Transaction ID").cloned(),
    }
}

fn normalize_inpas(raw: &HashMap<String, String>) -> NormalizedTransactionData {
    NormalizedTransactionData {
        raw: raw.clone(),
        amount: raw
            .get(inpas_prop_codes::AMOUNT)
            .map(|v| v.parse::<f64>().unwrap_or(0.0)),
        card_masked_pan: raw.get(inpas_prop_codes::PAN).cloned(),
        status_name: raw.get(inpas_prop_codes::STATUS).cloned(),
        host_timestamp: raw.get(inpas_prop_codes::DATETIME_HOST).cloned(),
        authorization_code: raw.get(inpas_prop_codes::AUTHORIZATION_CODE).cloned(),
        timestamp: raw.get(inpas_prop_codes::TERMINAL_DATETIME).cloned(),
        invoice_number: raw.get(inpas_prop_codes::TERMINAL_TRX_ID).cloned(),
        terminal_id: raw.get(inpas_prop_codes::TERMINAL_ID).cloned(),
        merchant_id: raw.get(inpas_prop_codes::MERCHANT_ID).cloned(),
        trx_id: raw.get(inpas_prop_codes::TRXID).cloned(),
        issuer_name: None,
    }
}

fn build_ttk_timestamp(date: Option<&String>, time: Option<&String>) -> Option<String> {
    match (date, time) {
        (Some(d), Some(t)) => Some(format!("{}{}", d, t)),
        _ => None,
    }
}
