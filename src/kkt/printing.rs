use super::Kkt;
use crate::{ProcessError, ProcessSuccess};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::time::Duration;

const MAX_PRINT_BODY: usize = 8 * 1024 * 1024;
const MAX_PRINT_RESPONSE: usize = 128 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrintTask {
    pub items: Vec<PrintItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum PrintItem {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alignment: Option<PrintAlignment>,
        #[serde(default, skip_serializing_if = "is_false")]
        bold: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<PrintTextSize>,
    },
    Qr {
        data: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alignment: Option<PrintAlignment>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        module_size: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        correction: Option<QrCorrection>,
    },
    Image {
        /// PNG or JPEG bytes encoded as standard base64, without a data-URL prefix.
        data: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alignment: Option<PrintAlignment>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
    },
}

fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrintAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrintTextSize {
    Normal,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QrCorrection {
    L,
    M,
    Q,
    H,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrintSupport {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterInfo {
    pub model: u32,
    pub model_name: String,
    pub driver_version: String,
    pub firmware_version: Option<String>,
    pub width_pixels: Option<u32>,
    pub width_chars: Option<u32>,
    pub native_text: PrintSupport,
    pub large_text: PrintSupport,
    pub native_qr: PrintSupport,
    pub raster: PrintSupport,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintAdaptation {
    pub item_index: usize,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrintState {
    Printed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintResult {
    pub state: PrintState,
    pub adaptations: Vec<PrintAdaptation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrintFailureState {
    NotStarted,
    PossiblyPartial,
    Unknown,
}

/// Structured details available in ProcessError.raw_data for the printing API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintFailure {
    pub error: String,
    pub category: String,
    pub state: PrintFailureState,
    pub driver_code: Option<i32>,
    pub driver_description: Option<String>,
    pub item_index: Option<usize>,
}

fn print_response_error(raw: Value) -> ProcessError {
    ProcessError {
        message: raw
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("Не удалось выполнить операцию печати ККТ")
            .to_owned(),
        raw_data: raw,
        input_data: None,
    }
}

fn print_local_error(message: &str, category: &str, state: &str) -> ProcessError {
    print_response_error(json!({"error":message,"category":category,"state":state}))
}

impl Kkt {
    /// Read printer capabilities. Unknown dimensions are never replaced with defaults.
    pub async fn printer_info(&self) -> Result<ProcessSuccess<PrinterInfo>, ProcessError> {
        self.print_request("printer_info", None).await
    }

    /// Print a non-fiscal document. A lost response must not trigger an automatic retry.
    /// Fiscal receipts continue to use payment/refund with electronically=false.
    pub async fn print(
        &self,
        task: &PrintTask,
    ) -> Result<ProcessSuccess<PrintResult>, ProcessError> {
        if task.items.is_empty() || task.items.len() > 256 {
            return Err(print_local_error(
                "Задание должно содержать от 1 до 256 элементов",
                "invalid_task",
                "not_started",
            ));
        }
        self.print_request("print", Some(task)).await
    }

    async fn print_request<T: DeserializeOwned>(
        &self,
        action: &str,
        task: Option<&PrintTask>,
    ) -> Result<ProcessSuccess<T>, ProcessError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| {
                print_local_error(
                    "Не удалось создать HTTP-клиент",
                    "transport_error",
                    "not_started",
                )
            })?;
        let url = self.make_url(action);
        let request = if let Some(task) = task {
            let body = serde_json::to_vec(task).map_err(|_| {
                print_local_error(
                    "Не удалось сформировать задание печати",
                    "invalid_task",
                    "not_started",
                )
            })?;
            if body.len() > MAX_PRINT_BODY {
                return Err(print_local_error(
                    "Задание печати превышает 8 MiB",
                    "invalid_task",
                    "not_started",
                ));
            }
            client
                .post(url)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body)
        } else {
            client.get(url)
        };
        // Once send starts, an error cannot prove that the service did not print.
        let unknown = || {
            print_local_error(
                "Не удалось получить достоверный результат от ККТ; не повторяйте печать автоматически",
                "unknown_result",
                "unknown",
            )
        };
        let mut response = request.send().await.map_err(|_| unknown())?;
        let status = response.status();
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unknown())? {
            if chunk.len() > MAX_PRINT_RESPONSE - body.len() {
                return Err(unknown());
            }
            body.extend_from_slice(&chunk);
        }
        let raw: Value = serde_json::from_slice(&body).map_err(|_| unknown())?;
        if !status.is_success() {
            // Preserve typed details only for the service's defined error shape.
            serde_json::from_value::<PrintFailure>(raw.clone()).map_err(|_| unknown())?;
            return Err(print_response_error(raw));
        }
        let data = serde_json::from_value(raw.clone()).map_err(|_| unknown())?;
        Ok(ProcessSuccess::new("Операция ККТ завершена", data, raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mixed_task_matches_go_contract() {
        let task = PrintTask {
            items: vec![
                PrintItem::Text {
                    text: "Привет".into(),
                    alignment: Some(PrintAlignment::Center),
                    bold: true,
                    size: Some(PrintTextSize::Large),
                },
                PrintItem::Qr {
                    data: "https://example.org".into(),
                    alignment: None,
                    module_size: Some(4),
                    correction: Some(QrCorrection::M),
                },
                PrintItem::Image {
                    data: "aW1hZ2U=".into(),
                    alignment: None,
                    width: Some(384),
                },
            ],
        };
        assert_eq!(
            serde_json::to_value(&task).unwrap(),
            json!({"items":[
                {"type":"text","text":"Привет","alignment":"center","bold":true,"size":"large"},
                {"type":"qr","data":"https://example.org","module_size":4,"correction":"M"},
                {"type":"image","data":"aW1hZ2U=","width":384}
            ]})
        );
    }

    #[test]
    fn structured_error_preserves_partial_state_and_driver_code() {
        let raw = json!({"error":"Нет бумаги", "category":"no_paper", "state":"possibly_partial", "driver_code":44,"item_index":2});
        let e = print_response_error(raw.clone());
        assert_eq!(e.message, "Нет бумаги");
        assert_eq!(e.raw_data, raw);
        assert!(e.input_data.is_none());
    }

    #[test]
    fn only_confirmed_print_result_is_success() {
        assert!(
            serde_json::from_value::<PrintResult>(json!({"state":"unknown","adaptations":[]}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<PrintResult>(json!({"state":"printed","adaptations":[]}))
                .is_ok()
        );
    }

    #[test]
    fn unknown_width_remains_absent() {
        let info: PrinterInfo = serde_json::from_value(json!({"model":0,"model_name":"","driver_version":"10.10.7.0","native_text":"unknown","native_qr":"unknown","large_text":"unknown","raster":"unsupported","notes":[]})).unwrap();
        assert_eq!(info.width_pixels, None);
        assert_eq!(info.raster, PrintSupport::Unsupported);
    }
}
