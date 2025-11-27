use crate::acquiring::types::{NormalizedTransactionData, ProtocolType, TerminalResponse};
use std::collections::HashMap;

const SUCCESS_CODE_REGEXP: &str = r"^0+$";

pub fn build_terminal_response_from_raw(
    protocol: ProtocolType,
    raw: HashMap<String, String>,
) -> TerminalResponse {
    let data = normalize_terminal_response(protocol, &raw);
    let success = determine_success(&data);
    let code = data
        .response_code
        .clone()
        .or_else(|| data.status.clone())
        .or_else(|| data.approve.clone());
    let message = data.text_response.clone().or_else(|| data.status_text.clone());

    TerminalResponse {
        success,
        code: code.clone(),
        message: message.clone(),
        data: Some(data),
        error: if success {
            None
        } else {
            Some(
                message
                    .unwrap_or_else(|| {
                        code.map(|c| format!("Response code: {}", c))
                            .unwrap_or_else(|| "Unknown terminal error".to_string())
                    })
                    .to_string(),
            )
        },
    }
}

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
    let known_keys: std::collections::HashSet<&str> = [
        "Message ID",
        "ECR Number",
        "Response Code",
        "Approve",
        "Transaction Amount",
        "Transaction Amount #2",
        "RRN",
        "Invoice Number",
        "Authorization ID",
        "Terminal ID",
        "Merchant No",
        "Batch No",
        "PAN",
        "Date",
        "Time",
        "POS Entry Mode",
        "Cardholder Verification",
        "Visual Host Response",
        "Receipt",
        "Application Label",
        "Issuer Name",
    ]
    .into_iter()
    .collect();

    let mut data = NormalizedTransactionData {
        raw: raw.clone(),
        message_id: raw.get("Message ID").cloned(),
        ecr_number: raw.get("ECR Number").cloned(),
        response_code: raw.get("Response Code").cloned(),
        approve: raw.get("Approve").cloned(),
        amount: raw
            .get("Transaction Amount")
            .or_else(|| raw.get("Transaction Amount #2"))
            .cloned(),
        rrn: raw.get("RRN").cloned(),
        invoice_number: raw.get("Invoice Number").cloned(),
        authorization_code: raw.get("Authorization ID").cloned(),
        terminal_id: raw.get("Terminal ID").cloned(),
        merchant_id: raw.get("Merchant No").cloned(),
        batch_number: raw.get("Batch No").cloned(),
        pan_masked: raw.get("PAN").cloned(),
        date: raw.get("Date").cloned(),
        time: raw.get("Time").cloned(),
        timestamp: build_ttk_timestamp(raw.get("Date"), raw.get("Time")),
        card_entry_mode: raw.get("POS Entry Mode").cloned(),
        cardholder_verification: raw.get("Cardholder Verification").cloned(),
        text_response: raw.get("Visual Host Response").cloned(),
        receipt: raw.get("Receipt").cloned(),
        application_label: raw.get("Application Label").cloned(),
        issuer_name: raw.get("Issuer Name").cloned(),
        operation_code: None,
        status: None,
        status_text: None,
        additional_amount: None,
        currency: None,
        host_timestamp: None,
        transaction_id: None,
        cashier_request: None,
        cashier_response: None,
        provider_code: None,
        extras: None,
    };

    let extras = collect_extras(raw, &known_keys);
    if !extras.is_empty() {
        data.extras = Some(extras);
    }

    data
}

fn normalize_inpas(raw: &HashMap<String, String>) -> NormalizedTransactionData {
    let known_keys: std::collections::HashSet<&str> = [
        "00", "01", "04", "06", "08", "09", "10", "13", "14", "15", "19", "21", "23", "25",
        "26", "27", "28", "39", "76", "77", "82", "90",
    ]
    .into_iter()
    .collect();

    let mut data = NormalizedTransactionData {
        raw: raw.clone(),
        response_code: raw.get("15").cloned(),
        text_response: raw.get("19").cloned(),
        amount: raw.get("00").cloned(),
        additional_amount: raw.get("01").cloned(),
        currency: raw.get("04").cloned(),
        host_timestamp: raw.get("06").cloned(),
        card_entry_mode: raw.get("08").cloned(),
        cardholder_verification: raw.get("09").map(|s| map_pin_coding_mode(s.clone())),
        pan_masked: raw.get("10").cloned(),
        authorization_code: raw.get("13").cloned(),
        rrn: raw.get("14").cloned(),
        timestamp: raw.get("21").cloned(),
        transaction_id: raw.get("23").cloned(),
        operation_code: raw.get("25").cloned(),
        invoice_number: raw.get("26").cloned(),
        terminal_id: raw.get("27").cloned(),
        merchant_id: raw.get("28").cloned(),
        status: raw.get("39").cloned(),
        cashier_request: raw.get("76").cloned(),
        cashier_response: raw.get("77").cloned(),
        provider_code: raw.get("82").cloned(),
        receipt: raw.get("90").cloned(),
        message_id: None,
        ecr_number: None,
        approve: None,
        status_text: None,
        date: None,
        time: None,
        batch_number: None,
        application_label: None,
        issuer_name: None,
        extras: None,
    };

    let extras = collect_extras(raw, &known_keys);
    if !extras.is_empty() {
        data.extras = Some(extras);
    }

    data
}

fn build_ttk_timestamp(date: Option<&String>, time: Option<&String>) -> Option<String> {
    match (date, time) {
        (Some(d), Some(t)) => Some(format!("{}{}", d, t)),
        _ => None,
    }
}

fn collect_extras(
    raw: &HashMap<String, String>,
    known_keys: &std::collections::HashSet<&str>,
) -> HashMap<String, String> {
    raw.iter()
        .filter(|(key, _)| !known_keys.contains(key.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn determine_success(data: &NormalizedTransactionData) -> bool {
    if let Some(ref response_code) = data.response_code {
        let re = regex::Regex::new(SUCCESS_CODE_REGEXP).unwrap();
        if re.is_match(response_code) {
            return true;
        }
    }

    if let Some(ref approve) = data.approve {
        if approve.to_uppercase() == "Y" {
            return true;
        }
    }

    if let Some(ref status) = data.status {
        let re = regex::Regex::new(SUCCESS_CODE_REGEXP).unwrap();
        if re.is_match(status) {
            return true;
        }
    }

    true
}

fn map_pin_coding_mode(value: String) -> String {
    match value.as_str() {
        "1" | "2" => "PIN".to_string(),
        _ => value,
    }
}

