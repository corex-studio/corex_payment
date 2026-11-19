use anyhow::{Result, anyhow};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use async_trait::async_trait;
use encoding_rs::{IBM866, WINDOWS_1251};

use crate::{
    ConnectionConfig, ConnectionType, ProcessError, ProcessSuccess,
    acquiring::{
        protocol::base::Acquiring,
        response::normalize_terminal_response,
        types::{ConnectionStatus, NormalizedTransactionData, RefundMode},
    },
    healthcheck::{HealthcheckResult, Healthchecker},
};

pub struct SBAdapter {
    pub config: ConnectionConfig,
    pub dir: PathBuf,
    is_connected: bool,
}

impl SBAdapter {
    pub fn new(config: ConnectionConfig, dir: PathBuf) -> Self {
        Self {
            config,
            dir,
            is_connected: false,
        }
    }

    pub fn read_e(&self) -> Result<SbPilotE> {
        parse_sb_pilot_e(self.dir.join("e"))
    }

    pub fn get_pilot(&self) -> Result<PathBuf> {
        let sb_pilot = self.dir.join("sb_pilot.exe");

        match sb_pilot.exists() {
            true => Ok(sb_pilot),
            false => Err(anyhow!("Missing sb_pilot.exe")),
        }
    }

    pub fn get_pinpad_ini(&self) -> std::result::Result<PathBuf, ProcessError> {
        let pinpad_ini = self.dir.join("pinpad.ini");

        match pinpad_ini.exists() {
            true => Ok(pinpad_ini),
            false => Err(ProcessError::new(
                "Missing pinpad.ini file",
                Value::String(format!("{:?}", pinpad_ini).to_string()),
            )),
        }
    }

    fn get_cmd(&self) -> std::result::Result<Command, ProcessError> {
        match self.get_pilot() {
            Ok(v) => Ok(Command::new(v)),
            Err(e) => Err(ProcessError::new(
                "Missing sbpilot",
                Value::String(e.to_string()),
            )),
        }
    }

    fn run_refund(
        &self,
        amount: u64,
        currency: Option<String>,
        reference: Option<(&str, RefundMode)>,
    ) -> std::result::Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let mut attempt = new_sb_attempt("refund");
        attempt["amount_minor"] = json!(amount);
        attempt["currency"] = json!(currency);
        attempt["original_rrn"] = json!(reference.map(|(rrn, _)| rrn));
        attempt["refund_mode"] = json!(
            reference
                .map(|(_, mode)| mode)
                .unwrap_or(RefundMode::WithCard)
        );
        let mut args = vec!["3".to_string(), amount.to_string()];
        if let Some((rrn, mode)) = reference {
            if rrn.is_empty() || !rrn.bytes().all(|byte| byte.is_ascii_digit()) {
                let message = "Некорректный RRN исходной оплаты: ожидается непустая строка цифр";
                attempt["outcome"] = json!("not_started");
                attempt["validation_error"] = json!(message);
                attempt["completed_at"] = json!(chrono::Utc::now().to_rfc3339());
                return Err(ProcessError {
                    message: message.to_string(),
                    raw_data: json!({ "error": message, "refund_attempt": attempt }),
                    input_data: Some(amount.to_string()),
                });
            }
            let track2 = match mode {
                RefundMode::WithCard => "0",
                RefundMode::WithoutCard => "QSELECT",
            };
            args.extend(["0".to_string(), track2.to_string(), rrn.to_string()]);
        }
        let PilotExecution {
            started,
            process_success,
            terminal_result,
            technical_error,
        } = self.execute_pilot(&args, &mut attempt);

        if let Some(result) = &terminal_result {
            if result.result_code == 0 && process_success {
                attempt["outcome"] = json!("approved");
                let data = normalize_terminal_response(self.config.protocol, &result.as_hash_map());
                return Ok(ProcessSuccess::new(
                    "Refund command success",
                    data,
                    json!({ "refund_attempt": attempt }),
                ));
            }
            if result.result_code > 0 {
                attempt["outcome"] = json!("declined");
                let reason = result.result_text.trim().trim_end_matches('.');
                let message = if reason.is_empty() {
                    format!("Возврат не выполнен. Код терминала: {}", result.result_code)
                } else {
                    format!(
                        "Возврат не выполнен: {reason}. Код терминала: {}",
                        result.result_code
                    )
                };
                return Err(ProcessError {
                    message,
                    raw_data: json!({ "error": technical_error, "refund_attempt": attempt }),
                    input_data: Some(amount.to_string()),
                });
            }
        }
        let message = if started {
            "Не удалось определить результат возврата. Проверьте состояние операции."
        } else {
            attempt["outcome"] = json!("not_started");
            "Возврат не выполнен: не удалось запустить банковское ПО."
        };
        Err(ProcessError {
            message: message.to_string(),
            raw_data: json!({ "error": technical_error, "refund_attempt": attempt }),
            input_data: Some(amount.to_string()),
        })
    }

    // Shared by all bank operations so failure details are never discarded.
    fn execute_pilot(&self, args: &[String], attempt: &mut Value) -> PilotExecution {
        let result_path = self.dir.join("e");
        // Do not reuse the previous operation's response or remove bank-owned files.
        let previous = read_sb_result_snapshot(&result_path);
        let mut started = false;
        let execution = (|| -> std::io::Result<std::process::Output> {
            // Resolve before changing cwd so relative sc552_path also works.
            let executable = fs::canonicalize(self.dir.join("sb_pilot.exe"))?;
            let child = Command::new(&executable)
                .current_dir(executable.parent().expect("Executable has a parent"))
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            started = true;
            child.wait_with_output()
        })();
        attempt["process"]["started"] = json!(started);
        let mut terminal_result = None;
        let mut process_success = false;
        let technical_error = match execution {
            Ok(output) => {
                process_success = output.status.success();
                attempt["process"]["success"] = json!(process_success);
                attempt["process"]["exit_code"] = json!(output.status.code());
                attempt["process"]["stdout"] = json!(decode_sb_output(&output.stdout));
                attempt["process"]["stderr"] = json!(decode_sb_output(&output.stderr));
                let result = (|| -> Result<SbPilotE> {
                    let current = read_sb_result_snapshot(&result_path)?;
                    attempt["terminal_response"] = json!(decode_sb_result(&current.bytes));
                    match &previous {
                        Ok(old) if *old == current => {
                            return Err(anyhow!("Result file e was not updated"));
                        }
                        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                            return Err(anyhow!("Cannot verify result file freshness: {error}"));
                        }
                        _ => {}
                    }
                    parse_sb_pilot_e_bytes(&current.bytes)
                })();
                match result {
                    Ok(result) => {
                        attempt["terminal_result"] = json!(result);
                        terminal_result = Some(result);
                    }
                    Err(error) => attempt["result_file_error"] = json!(error.to_string()),
                }
                format!(
                    "Process status: {}; result file error: {}",
                    output.status, attempt["result_file_error"]
                )
            }
            Err(error) => {
                let field = if started {
                    "wait_error"
                } else {
                    "launch_error"
                };
                attempt["process"][field] = json!(error.to_string());
                error.to_string()
            }
        };
        attempt["completed_at"] = json!(chrono::Utc::now().to_rfc3339());

        PilotExecution {
            started,
            process_success,
            terminal_result,
            technical_error,
        }
    }

    fn run_command(
        &self,
        operation: &str,
        args: &[String],
        amount: Option<u64>,
        currency: Option<String>,
    ) -> std::result::Result<SbPilotE, ProcessError> {
        let mut attempt = new_sb_attempt(operation);
        attempt["amount_minor"] = json!(amount);
        attempt["currency"] = json!(currency);
        let execution = self.execute_pilot(args, &mut attempt);
        let message = match execution.terminal_result {
            Some(result) if result.result_code == 0 && execution.process_success => {
                return Ok(result);
            }
            Some(result) if result.result_code > 0 => {
                attempt["outcome"] = json!("declined");
                format!(
                    "Failed to run {operation} command: {}. Код терминала: {}",
                    result.result_text.trim().trim_end_matches('.'),
                    result.result_code
                )
            }
            _ => {
                if !execution.started {
                    attempt["outcome"] = json!("not_started");
                }
                match operation {
                    "payment" => "Результат оплаты не подтверждён".to_string(),
                    _ => "Результат сверки не подтверждён".to_string(),
                }
            }
        };
        Err(ProcessError {
            message,
            raw_data: json!({ "error": execution.technical_error, format!("{operation}_attempt"): attempt }),
            input_data: amount.map(|value| value.to_string()),
        })
    }

    fn configure_usb(&self) -> Result<Vec<IniEdit>> {
        let com = match &self.config.address {
            Some(v) => keep_only_digits(v.clone()),
            None => return Err(anyhow!("Param ncom is missing")),
        };

        let baudrate = match self.config.port {
            Some(v) => v.to_string(),
            None => "9600".to_string(),
        };

        Ok(vec![
            IniEdit::SetValue {
                key: "EnableUSB".to_string(),
                value: "1".to_string(),
            },
            IniEdit::SetValue {
                key: "ComPort".to_string(),
                value: com.to_string(),
            },
            IniEdit::SetValue {
                key: "Speed".to_string(),
                value: baudrate,
            },
            IniEdit::CommentOut {
                key: "PinpadIPAddr".to_string(),
            },
            IniEdit::CommentOut {
                key: "PinpadIPPort".to_string(),
            },
        ])
    }

    pub(crate) fn tcp_port(&self) -> u32 {
        self.config.port.unwrap_or(9600)
    }

    pub(crate) fn validate_pinpad_ini(&self, path: &Path) -> Result<()> {
        let ini = IniEditor::load(path)?;
        let expected = match self.config.connection_type {
            ConnectionType::Usb => self.configure_usb()?,
            ConnectionType::Tcp => self.configure_tcp()?,
            // check_port reports unsupported connection types.
            ConnectionType::Bluetooth => return Ok(()),
        };
        ini.validate(&expected)
    }

    fn configure_tcp(&self) -> Result<Vec<IniEdit>> {
        let address = match &self.config.address {
            Some(v) => v,
            None => return Err(anyhow!("Param ncom is missing")),
        };

        let port = self.tcp_port().to_string();

        Ok(vec![
            IniEdit::SetValue {
                key: "EnableUSB".to_string(),
                value: "0".to_string(),
            },
            IniEdit::CommentOut {
                key: "ComPort".to_string(),
            },
            IniEdit::CommentOut {
                key: "Speed".to_string(),
            },
            IniEdit::SetValue {
                key: "PinpadIPAddr".to_string(),
                value: address.to_string(),
            },
            IniEdit::SetValue {
                key: "PinpadIPPort".to_string(),
                value: port,
            },
        ])
    }
}

#[async_trait]
impl Acquiring for SBAdapter {
    async fn connected(
        &self,
    ) -> std::result::Result<ProcessSuccess<ConnectionStatus>, ProcessError> {
        Ok(ProcessSuccess::new(
            "Connection check",
            ConnectionStatus::new(self.is_connected),
            Value::Bool(self.is_connected),
        ))
    }

    async fn connect(
        &mut self,
    ) -> std::result::Result<ProcessSuccess<ConnectionStatus>, ProcessError> {
        self.get_cmd()?;

        let pinpad_ini = self.get_pinpad_ini()?;
        let mut ini_editor = IniEditor::load(&pinpad_ini).map_err(|e| {
            ProcessError::new(
                "Failed to read pinpad.ini file",
                Value::String(e.to_string()),
            )
        })?;

        match &self.config.connection_type {
            ConnectionType::Usb => {
                let edits = self.configure_usb().map_err(|e| {
                    ProcessError::new(
                        "Unable to configure as USB type",
                        Value::String(e.to_string()),
                    )
                })?;
                ini_editor.edit_many(edits);
            }
            ConnectionType::Tcp => {
                let edits = self.configure_tcp().map_err(|e| {
                    ProcessError::new(
                        "Unable to configure as TCP type",
                        Value::String(e.to_string()),
                    )
                })?;
                ini_editor.edit_many(edits);
            }
            _ => {
                return Err(ProcessError::new(
                    "Unimplemented type of connection",
                    Value::String(format!("{:?}", &self.config.connection_type)),
                ));
            }
        };
        ini_editor.save(pinpad_ini).map_err(|e| {
            ProcessError::new("Failed to save pinpad.ini", Value::String(e.to_string()))
        })?;

        self.is_connected = true;

        Ok(ProcessSuccess::new(
            "Files sb_pilot and pinpad.ini are ready to be used",
            ConnectionStatus::new(true),
            Value::Bool(true),
        ))
    }

    async fn disconnect(
        &mut self,
    ) -> std::result::Result<ProcessSuccess<ConnectionStatus>, ProcessError> {
        self.is_connected = false;
        Ok(ProcessSuccess::new(
            "Disconnection is not actually needed for SB adapter",
            ConnectionStatus::new(false),
            Value::Bool(false),
        ))
    }

    async fn payment(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> std::result::Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let terminal_result = self.run_command(
            "payment",
            &["1".to_string(), amount.to_string()],
            Some(amount),
            currency,
        )?;

        let data =
            normalize_terminal_response(self.config.protocol, &terminal_result.as_hash_map());

        Ok(ProcessSuccess::new(
            "Payment command success",
            data,
            Value::String(format!("{terminal_result:?}")),
        ))
    }

    async fn refund(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> std::result::Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        self.run_refund(amount, currency, None)
    }

    async fn refund_with_reference(
        &mut self,
        amount: u64,
        currency: Option<String>,
        rrn: String,
        mode: RefundMode,
    ) -> std::result::Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        self.run_refund(amount, currency, Some((&rrn, mode)))
    }

    async fn totals(
        &mut self,
    ) -> std::result::Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let report_path = self.dir.join("p");
        let previous = read_sb_result_snapshot(&report_path);
        self.run_command("totals", &["7".to_string()], None, None)?;

        let mut data = NormalizedTransactionData::empty();
        if let Ok(report) = read_sb_result_snapshot(&report_path) {
            let fresh = match previous {
                Ok(old) => old != report,
                Err(error) => error.kind() == std::io::ErrorKind::NotFound,
            };
            if fresh {
                data.raw.insert("p".into(), decode_sb_output(&report.bytes));
            }
        }
        Ok(ProcessSuccess::new(
            "Totals command success",
            data,
            Value::Null,
        ))
    }

    async fn healthcheck(&self) -> Result<ProcessSuccess<HealthcheckResult>, ProcessError> {
        let result = self.run_healthcheck().await;
        let raw_result = serde_json::to_value(&result).unwrap_or(Value::Null);
        Ok(ProcessSuccess::new(
            "Healthcheck completed",
            result,
            raw_result,
        ))
    }
}

struct PilotExecution {
    started: bool,
    process_success: bool,
    terminal_result: Option<SbPilotE>,
    technical_error: String,
}

fn new_sb_attempt(operation: &str) -> Value {
    json!({
        "attempt_id": format!("{:032x}", rand::random::<u128>()),
        "operation": operation,
        "validation_error": null,
        "started_at": chrono::Utc::now().to_rfc3339(),
        "completed_at": null,
        "outcome": "unknown",
        "terminal_response": null,
        "terminal_result": null,
        "result_file_error": null,
        "process": {
            "started": false, "success": null, "exit_code": null,
            "stdout": null, "stderr": null, "launch_error": null, "wait_error": null,
        },
    })
}

#[derive(PartialEq, Eq)]
struct SbResultSnapshot {
    bytes: Vec<u8>,
    modified: std::time::SystemTime,
    created: Option<std::time::SystemTime>,
}

fn read_sb_result_snapshot(path: &Path) -> std::io::Result<SbResultSnapshot> {
    let bytes = fs::read(path)?;
    let metadata = fs::metadata(path)?;
    Ok(SbResultSnapshot {
        bytes,
        modified: metadata.modified()?,
        created: metadata.created().ok(),
    })
}

// Both encodings accept almost any byte sequence. Prefer Russian letters over
// box drawing, foreign letters and symbols typical of the wrong decoding.
// Ambiguous text cannot be identified reliably; retain CP866 as the default.
fn decode_sb_result(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    let cp866 = IBM866.decode(bytes).0;
    let cp1251 = WINDOWS_1251.decode(bytes).0;
    let score = |text: &str| -> i64 {
        text.chars()
            .map(|ch| match ch {
                'А'..='я' | 'Ё' | 'ё' => 1,
                ch if ch.is_ascii() => 0,
                _ => -2,
            })
            .sum()
    };
    if score(&cp1251) > score(&cp866) {
        cp1251
    } else {
        cp866
    }
}

fn decode_sb_output(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| decode_sb_result(bytes).into_owned())
}

#[derive(Debug, Clone)]
pub enum IniEdit {
    SetValue { key: String, value: String },
    CommentOut { key: String },
}

#[derive(Debug)]
pub struct IniEditor {
    lines: Vec<String>,
}

impl Display for IniEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lines.join("\n"))
    }
}

impl IniEditor {
    fn validate(&self, expected: &[IniEdit]) -> Result<()> {
        for edit in expected {
            let (key, value) = match edit {
                IniEdit::SetValue { key, value } => (key, Some(value.as_str())),
                IniEdit::CommentOut { key } => (key, None),
            };
            let mut found = false;
            for (name, actual) in self.lines.iter().filter_map(|line| line.split_once('=')) {
                if name.trim().eq_ignore_ascii_case(key) {
                    found = true;
                    if Some(actual.trim()) != value {
                        return Err(anyhow!(
                            "Параметр {key} не соответствует настройкам подключения"
                        ));
                    }
                }
            }
            if value.is_some() && !found {
                return Err(anyhow!("Отсутствует активный параметр {key}"));
            }
        }
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let lines = content.lines().map(|s| s.to_string()).collect();

        Ok(Self { lines })
    }

    pub fn edit(&mut self, operation: IniEdit) -> &mut Self {
        match operation {
            IniEdit::SetValue { key, value } => {
                self.set_value(&key, &value);
            }
            IniEdit::CommentOut { key } => {
                self.comment_out(&key);
            }
        }
        self
    }

    pub fn edit_many(&mut self, operations: impl IntoIterator<Item = IniEdit>) -> &mut Self {
        for op in operations {
            self.edit(op);
        }
        self
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = self.lines.join("\n");
        match fs::write(path, content) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e)),
        }
    }

    fn set_value(&mut self, key: &str, new_value: &str) {
        for line in &mut self.lines {
            let trimmed = line.trim_start();

            if trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }

            if let Some((line_key, _)) = trimmed.split_once('=')
                && line_key.trim() == key
            {
                let indent = line.len() - trimmed.len();
                *line = format!("{}{key}={new_value}", " ".repeat(indent));
                return;
            }
        }

        self.lines.push(format!("{key}={new_value}"));
    }

    fn comment_out(&mut self, key: &str) {
        for line in &mut self.lines {
            let trimmed = line.trim_start();

            if trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }

            if let Some((line_key, _)) = trimmed.split_once('=')
                && line_key.trim() == key
            {
                let indent = line.len() - trimmed.len();
                *line = format!("{};{trimmed}", " ".repeat(indent));
                return;
            }
        }
    }
}

fn keep_only_digits(s: String) -> String {
    s.chars().filter(|c| c.is_numeric()).collect()
}

#[derive(Debug, Serialize)]
pub struct SbPilotE {
    pub result_code: i32,
    pub result_text: String,
    pub masked_pan_or_phone: Option<String>,
    pub card_expiry: Option<String>,
    pub auth_code: Option<String>,
    pub operation_id: Option<String>,
    pub card_type: Option<String>,
    pub is_sber_card: Option<bool>,
    pub terminal_id: Option<String>,
    pub datetime: Option<String>,
    pub rrn: Option<String>,
    pub card_hash: Option<String>,
    pub merchant_id: Option<String>,
}

impl SbPilotE {
    pub fn empty() -> Self {
        Self {
            result_code: -1,
            result_text: String::new(),
            masked_pan_or_phone: None,
            card_expiry: None,
            auth_code: None,
            operation_id: None,
            card_type: None,
            is_sber_card: None,
            terminal_id: None,
            datetime: None,
            rrn: None,
            card_hash: None,
            merchant_id: None,
        }
    }

    pub fn as_hash_map(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("result_text".to_string(), self.result_text.clone());
        m.insert("result_code".to_string(), self.result_code.to_string());
        if let Some(v) = &self.masked_pan_or_phone {
            m.insert("masked_pan".to_string(), v.clone());
        }
        if let Some(v) = &self.card_expiry {
            m.insert("card_expiry".to_string(), v.clone());
        }
        if let Some(v) = &self.auth_code {
            m.insert("auth_code".to_string(), v.clone());
        }
        if let Some(v) = &self.operation_id {
            m.insert("operation_id".to_string(), v.clone());
        }
        if let Some(v) = &self.card_type {
            m.insert("card_type".to_string(), v.clone());
        }
        if let Some(v) = &self.is_sber_card {
            m.insert("is_sber_card".to_string(), v.to_string());
        }
        if let Some(v) = &self.terminal_id {
            m.insert("terminal_id".to_string(), v.clone());
        }
        if let Some(v) = &self.datetime {
            let ts = chrono::NaiveDateTime::parse_from_str(v, "%Y%m%d%H%M%S")
                .ok()
                .map(|dt| dt.and_utc().timestamp_millis().to_string())
                .unwrap_or_else(|| v.clone());
            m.insert("datetime".to_string(), ts);
        }
        if let Some(v) = &self.rrn {
            m.insert("rrn".to_string(), v.clone());
        }
        if let Some(v) = &self.card_hash {
            m.insert("card_hash".to_string(), v.clone());
        }
        if let Some(v) = &self.merchant_id {
            m.insert("merchant_id".to_string(), v.clone());
        }
        m
    }
}

pub fn parse_sb_pilot_e(path: PathBuf) -> Result<SbPilotE> {
    let bytes = fs::read(path)?;
    parse_sb_pilot_e_bytes(&bytes)
}

fn parse_sb_pilot_e_bytes(bytes: &[u8]) -> Result<SbPilotE> {
    let text = decode_sb_result(bytes);
    let lines: Vec<String> = text
        .into_owned()
        .lines()
        .map(|l| l.trim().to_string())
        .collect();

    // 1. Result code
    let (code, status_text) = lines
        .first()
        .ok_or(anyhow!("Empty result file"))?
        .split_once(',')
        .ok_or(anyhow!("Invalid result line"))?;
    let code = code.trim();
    let result_text = match code {
        "0" => "Успешно".to_string(),
        _ => status_text.to_string(),
    };

    let result_code: i32 = code.parse()?;
    if result_code < 0 {
        return Err(anyhow!("Invalid terminal result code: {result_code}"));
    }

    let mut i = 1;

    let mut next = || {
        if i < lines.len() {
            let v = lines[i].clone();
            i += 1;
            Some(v)
        } else {
            None
        }
    };

    let mut e = SbPilotE {
        result_code,
        result_text,

        masked_pan_or_phone: None,
        card_expiry: None,
        auth_code: None,
        operation_id: None,
        card_type: None,
        is_sber_card: None,
        terminal_id: None,
        datetime: None,
        rrn: None,
        card_hash: None,
        merchant_id: None,
    };

    if result_code != 0 {
        return Ok(e);
    }

    e.masked_pan_or_phone = next();
    e.card_expiry = next();
    e.auth_code = next();
    e.operation_id = next();
    e.card_type = next();
    e.is_sber_card = next().map(|v| v == "1");
    e.terminal_id = next();
    e.datetime = next();
    e.rrn = next().filter(|v| !v.is_empty());
    e.card_hash = next();
    next(); // пустая строка (line 12)
    next(); // строка 0 (line 13, не описана)
    e.merchant_id = next().filter(|v| !v.is_empty());

    Ok(e)
}

#[cfg(all(test, unix))]
mod refund_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(script: &str, response: Option<&str>) -> Self {
            let dir =
                std::env::temp_dir().join(format!("corex-refund-{:032x}", rand::random::<u128>()));
            fs::create_dir(&dir).unwrap();
            let pilot = dir.join("sb_pilot.exe");
            fs::write(&pilot, format!("#!/bin/sh\n{script}\n")).unwrap();
            fs::set_permissions(&pilot, fs::Permissions::from_mode(0o700)).unwrap();
            if let Some(response) = response {
                fs::write(dir.join("response"), IBM866.encode(response).0).unwrap();
            }
            Self(dir)
        }

        fn adapter(&self) -> SBAdapter {
            SBAdapter::new(
                ConnectionConfig {
                    connection_type: ConnectionType::Usb,
                    protocol: crate::ProtocolType::Sb,
                    serial_number: String::new(),
                    address: None,
                    port: None,
                    timeout: None,
                    dc_host: None,
                    sc552_path: None,
                },
                self.0.clone(),
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn payment_and_totals_preserve_bank_failure_diagnostics() {
        for (operation, args, key) in [
            (
                "payment",
                "test \"$#\" = 2 && test \"$1\" = 1 && test \"$2\" = 39000",
                "payment_attempt",
            ),
            (
                "totals",
                "test \"$#\" = 1 && test \"$1\" = 7",
                "totals_attempt",
            ),
        ] {
            let script = format!(
                "{args} || exit 88\ncp response e\necho diagnostic-output\necho diagnostic-error >&2\nexit 99"
            );
            let fixture = Fixture::new(&script, Some("362,Ошибка чтения карты"));
            let mut adapter = fixture.adapter();
            let error = if operation == "payment" {
                adapter.payment(39000, None).await.unwrap_err()
            } else {
                adapter.totals().await.unwrap_err()
            };
            let attempt = &error.raw_data[key];
            assert_eq!(attempt["process"]["exit_code"], 99);
            assert_eq!(attempt["process"]["stdout"], "diagnostic-output\n");
            assert_eq!(attempt["process"]["stderr"], "diagnostic-error\n");
            assert_eq!(attempt["terminal_response"], "362,Ошибка чтения карты");
            assert_eq!(attempt["terminal_result"]["result_code"], 362);
            assert!(error.message.contains("Ошибка чтения карты"));
            assert!(!error.message.contains("refund"));
            assert_eq!(
                error.input_data.as_deref(),
                if operation == "payment" {
                    Some("39000")
                } else {
                    None
                }
            );
        }
    }

    #[tokio::test]
    async fn totals_keeps_unreadable_or_stale_response_diagnostics() {
        for response in [None, Some("invalid"), Some("362,old response")] {
            let fixture = Fixture::new("echo failure >&2\nexit 99", None);
            if let Some(response) = response {
                fs::write(fixture.0.join("e"), response).unwrap();
            }
            let error = fixture.adapter().totals().await.unwrap_err();
            let attempt = &error.raw_data["totals_attempt"];
            assert_eq!(attempt["process"]["stderr"], "failure\n");
            assert!(attempt["result_file_error"].is_string());
            assert!(attempt["terminal_result"].is_null());
            assert!(!error.message.contains("old response"));
            assert_eq!(attempt["terminal_response"], json!(response));
        }
    }

    #[tokio::test]
    async fn totals_records_launch_failure() {
        let fixture = Fixture::new("exit 0", None);
        fs::remove_file(fixture.0.join("sb_pilot.exe")).unwrap();
        let error = fixture.adapter().totals().await.unwrap_err();
        let attempt = &error.raw_data["totals_attempt"];
        assert_eq!(attempt["process"]["started"], false);
        assert!(attempt["process"]["launch_error"].is_string());
        assert!(attempt["process"]["exit_code"].is_null());
    }

    #[test]
    fn decodes_russian_process_output() {
        let text = "НЕТ СВЯЗИ С ТЕРМИНАЛОМ";
        for bytes in [
            text.as_bytes().to_vec(),
            IBM866.encode(text).0.into_owned(),
            WINDOWS_1251.encode(text).0.into_owned(),
        ] {
            assert_eq!(decode_sb_output(&bytes), text);
        }
    }

    #[tokio::test]
    async fn totals_preserves_fresh_malformed_response() {
        let fixture = Fixture::new("cp response e\nexit 99", Some("broken response"));
        let error = fixture.adapter().totals().await.unwrap_err();
        let attempt = &error.raw_data["totals_attempt"];
        assert_eq!(attempt["terminal_response"], "broken response");
        assert!(attempt["terminal_result"].is_null());
        assert!(attempt["result_file_error"].is_string());
    }

    #[tokio::test]
    async fn successful_payment_and_totals_keep_existing_responses() {
        let fixture = Fixture::new(
            "cp response e",
            Some(
                "0,OK\n1234****5678\n12/30\nAUTH42\n7\nCard\n1\nTERM42\n20261119123456\n000000000042",
            ),
        );
        let result = fixture.adapter().payment(39000, None).await.unwrap();
        assert!(result.raw_data.is_string());
        assert_eq!(result.data.is_approved, Some(true));
        assert_eq!(result.data.authorization_code.as_deref(), Some("AUTH42"));
        assert_eq!(result.data.terminal_id.as_deref(), Some("TERM42"));
        assert_eq!(result.data.raw["rrn"], "000000000042");
        let fixture = Fixture::new("cp response e && cp report p", Some("0,OK"));
        let report = "  Сверка итогов\r\nИтоги совпали\r\n";
        fs::write(fixture.0.join("report"), IBM866.encode(report).0).unwrap();
        let result = fixture.adapter().totals().await.unwrap();
        assert!(result.raw_data.is_null());
        assert_eq!(result.message, "Totals command success");
        assert_eq!(result.data.raw["p"], report);
    }

    #[tokio::test]
    async fn payment_returns_fresh_decline_on_successful_exit() {
        let fixture = Fixture::new("cp response e", Some("362,Ошибка чтения карты"));
        let error = fixture.adapter().payment(39000, None).await.unwrap_err();
        assert!(error.message.contains("362"));
        assert!(error.message.contains("Ошибка чтения карты"));
        let attempt = &error.raw_data["payment_attempt"];
        assert_eq!(attempt["outcome"], "declined");
        assert_eq!(attempt["process"]["exit_code"], 0);
        assert_eq!(attempt["terminal_result"]["result_code"], 362);
    }

    #[tokio::test]
    async fn payment_requires_confirmed_result() {
        for (script, previous, response) in [
            ("exit 0", Some("0,OK"), None),
            ("exit 0", None, None),
            ("cp response e", None, Some("invalid")),
            ("cp response e\nexit 99", None, Some("0,OK")),
        ] {
            let fixture = Fixture::new(script, response);
            if let Some(previous) = previous {
                fs::write(fixture.0.join("e"), previous).unwrap();
            }
            let error = fixture.adapter().payment(39000, None).await.unwrap_err();
            assert!(error.message.contains("Результат оплаты не подтверждён"));
            assert_eq!(error.raw_data["payment_attempt"]["outcome"], "unknown");
        }
    }

    #[tokio::test]
    async fn totals_returns_fresh_decline_on_successful_exit() {
        let fixture = Fixture::new("cp response e\nexit 0", Some("362,Ошибка чтения карты"));
        let error = fixture.adapter().totals().await.unwrap_err();
        assert!(error.message.contains("Ошибка чтения карты"));
        assert!(error.message.contains("362"));
        let attempt = &error.raw_data["totals_attempt"];
        assert_eq!(attempt["process"]["exit_code"], 0);
        assert_eq!(attempt["terminal_result"]["result_code"], 362);
        assert_eq!(attempt["outcome"], "declined");
    }

    #[tokio::test]
    async fn totals_requires_matching_process_and_terminal_success() {
        for (script, response, previous) in [
            ("exit 0", None, None),
            ("exit 0", None, Some("0,OK")),
            ("cp response e", Some("invalid"), None),
            ("cp response e\nexit 99", Some("0,OK"), None),
        ] {
            let fixture = Fixture::new(script, response);
            if let Some(previous) = previous {
                fs::write(fixture.0.join("e"), previous).unwrap();
            }
            let error = fixture.adapter().totals().await.unwrap_err();
            assert!(error.message.contains("Результат сверки не подтверждён"));
            assert_eq!(error.raw_data["totals_attempt"]["outcome"], "unknown");
        }
    }

    #[tokio::test]
    async fn confirmed_totals_omits_missing_or_stale_report() {
        for previous in [None, Some("old receipt")] {
            let fixture = Fixture::new("cp response e", Some("0,OK"));
            if let Some(previous) = previous {
                fs::write(fixture.0.join("p"), previous).unwrap();
            }
            let result = fixture.adapter().totals().await.unwrap();
            assert!(!result.data.raw.contains_key("p"));
        }
    }

    const WRITE_RESPONSE: &str = "test \"$1\" = 3 && test \"$2\" = 39000 || exit 88\ncp \"$(dirname \"$0\")/response\" \"$(dirname \"$0\")/e\"\necho diagnostic-output\necho diagnostic-error >&2\nexit 99";

    #[tokio::test]
    async fn legacy_refund_keeps_two_command_arguments() {
        let fixture = Fixture::new(
            "test \"$#\" = 2 && test \"$1\" = 3 && test \"$2\" = 19000 || exit 88\ncp response e\nexit 0",
            Some("0,OK"),
        );
        let result = fixture.adapter().refund(19000, None).await.unwrap();
        assert_eq!(result.raw_data["refund_attempt"]["outcome"], "approved");
        assert!(result.raw_data["refund_attempt"]["original_rrn"].is_null());
    }

    #[tokio::test]
    async fn passes_reference_and_explicit_card_mode_through_terminal() {
        for (mode, track2) in [
            (RefundMode::WithCard, "0"),
            (RefundMode::WithoutCard, "QSELECT"),
        ] {
            let script = format!(
                "test \"$#\" = 5 && test \"$1\" = 3 && test \"$2\" = 19000 && test \"$3\" = 0 && test \"$4\" = {track2} && test \"$5\" = 001234567890 || exit 88\ncp response e\nexit 0"
            );
            let fixture = Fixture::new(&script, Some("0,OK"));
            for protocol in [crate::ProtocolType::Sb, crate::ProtocolType::Ttk] {
                let mut config = fixture.adapter().config;
                config.protocol = protocol;
                config.sc552_path = Some(fixture.0.to_string_lossy().into_owned());
                let mut terminal = crate::Terminal::new(config);
                let result = terminal
                    .refund_with_reference(19000, None, "001234567890".into(), mode)
                    .await
                    .unwrap();
                let attempt = &result.raw_data["refund_attempt"];
                assert_eq!(attempt["original_rrn"], "001234567890");
                assert_eq!(attempt["refund_mode"], serde_json::to_value(mode).unwrap());
                assert_eq!(attempt["outcome"], "approved");
            }
        }
    }

    #[tokio::test]
    async fn invalid_reference_does_not_start_bank_software() {
        let fixture = Fixture::new("touch launched\nexit 0", None);
        for rrn in ["", " ", "123\n456", "QSELECT", "12/34", "１２３"] {
            let error = fixture
                .adapter()
                .refund_with_reference(19000, None, rrn.into(), RefundMode::WithCard)
                .await
                .unwrap_err();
            assert!(!fixture.0.join("launched").exists());
            assert_eq!(
                error.raw_data["refund_attempt"]["process"]["started"],
                false
            );
            assert_eq!(error.raw_data["refund_attempt"]["outcome"], "not_started");
            assert_eq!(error.raw_data["refund_attempt"]["original_rrn"], rrn);
        }
    }

    #[tokio::test]
    async fn reference_is_preserved_when_terminal_declines() {
        let fixture = Fixture::new(WRITE_RESPONSE, Some("4476,Нет исходной операции"));
        let error = fixture
            .adapter()
            .refund_with_reference(39000, None, "001234567890".into(), RefundMode::WithCard)
            .await
            .unwrap_err();
        assert!(error.message.contains("4476"));
        assert_eq!(
            error.raw_data["refund_attempt"]["original_rrn"],
            "001234567890"
        );
        assert_eq!(error.raw_data["refund_attempt"]["refund_mode"], "with_card");
    }

    #[tokio::test]
    async fn inpas_rejects_reference_without_attempting_network_call() {
        let fixture = Fixture::new("exit 0", None);
        let mut config = fixture.adapter().config;
        config.protocol = crate::ProtocolType::Inpas;
        // dc_host is absent; trying to send an Inpas operation would panic.
        let mut terminal = crate::Terminal::new(config);
        let error = terminal
            .refund_with_reference(19000, None, "001234567890".into(), RefundMode::WithoutCard)
            .await
            .unwrap_err();
        assert!(error.message.contains("не поддерживается"));
    }

    #[tokio::test]
    async fn returns_terminal_decline_instead_of_process_exit_code() {
        for encoding in [IBM866, WINDOWS_1251] {
            for (code, reason) in [
                (4476, "Нет исходной операции"),
                (362, "Ошибка чтения карты"),
            ] {
                let response = format!("{code},{reason}\r\n");
                let fixture = Fixture::new(WRITE_RESPONSE, None);
                fs::write(fixture.0.join("response"), encoding.encode(&response).0).unwrap();
                let error = fixture
                    .adapter()
                    .refund(39000, Some("643".into()))
                    .await
                    .unwrap_err();
                assert!(error.message.contains(reason), "{}", error.message);
                assert!(error.message.contains(&code.to_string()));
                assert_eq!(error.input_data.as_deref(), Some("39000"));
                let attempt = &error.raw_data["refund_attempt"];
                assert_eq!(attempt["terminal_result"]["result_code"], code);
                assert_eq!(attempt["terminal_response"], format!("{code},{reason}\r\n"));
                assert_eq!(attempt["process"]["exit_code"], 99);
                assert_eq!(attempt["process"]["stdout"], "diagnostic-output\n");
                assert_eq!(attempt["process"]["stderr"], "diagnostic-error\n");
                assert_eq!(attempt["amount_minor"], 39000);
                assert_eq!(attempt["currency"], "643");
                assert_eq!(attempt["outcome"], "declined");
                assert!(
                    attempt["attempt_id"]
                        .as_str()
                        .is_some_and(|id| !id.is_empty())
                );
                assert!(
                    chrono::DateTime::parse_from_rfc3339(attempt["started_at"].as_str().unwrap())
                        .is_ok()
                );
                assert!(
                    chrono::DateTime::parse_from_rfc3339(attempt["completed_at"].as_str().unwrap())
                        .is_ok()
                );
            }
        }
    }

    #[tokio::test]
    async fn ignores_unchanged_previous_result() {
        let fixture = Fixture::new("exit 99", None);
        fs::write(
            fixture.0.join("e"),
            IBM866.encode("4476,Нет исходной операции").0,
        )
        .unwrap();
        let error = fixture.adapter().refund(39000, None).await.unwrap_err();
        assert!(!error.message.contains("4476"));
        assert!(error.raw_data["refund_attempt"]["terminal_result"].is_null());
        assert!(error.raw_data["refund_attempt"]["result_file_error"].is_string());
    }

    #[tokio::test]
    async fn does_not_report_success_without_fresh_valid_result() {
        for response in [None, Some(""), Some("invalid"), Some("-1,unknown")] {
            let script = if response.is_some() {
                "cp response e\nexit 0"
            } else {
                "exit 0"
            };
            let fixture = Fixture::new(script, response);
            let error = fixture.adapter().refund(39000, None).await.unwrap_err();
            assert_eq!(error.raw_data["refund_attempt"]["outcome"], "unknown");
            assert_eq!(error.raw_data["refund_attempt"]["process"]["exit_code"], 0);
        }
    }

    #[tokio::test]
    async fn terminal_decline_overrides_successful_process_exit() {
        let fixture = Fixture::new("cp response e\nexit 0", Some("362,Ошибка чтения карты"));
        let error = fixture.adapter().refund(39000, None).await.unwrap_err();
        assert!(error.message.contains("362"));
    }

    #[tokio::test]
    async fn records_approved_refund_and_uses_bank_directory() {
        let fixture = Fixture::new("cp response e\nexit 0", Some("0,OK\r\n"));
        let success = fixture.adapter().refund(39000, None).await.unwrap();
        assert_eq!(success.data.is_approved, Some(true));
        assert_eq!(success.raw_data["refund_attempt"]["outcome"], "approved");
        assert_eq!(
            success.raw_data["refund_attempt"]["terminal_response"],
            "0,OK\r\n"
        );
    }

    #[tokio::test]
    async fn records_launch_failure_and_distinct_attempt_ids() {
        let fixture = Fixture::new("exit 0", None);
        fs::set_permissions(
            fixture.0.join("sb_pilot.exe"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let first = fixture.adapter().refund(39000, None).await.unwrap_err();
        let second = fixture.adapter().refund(39000, None).await.unwrap_err();
        let attempt = &first.raw_data["refund_attempt"];
        assert_eq!(attempt["process"]["started"], false);
        assert_eq!(attempt["outcome"], "not_started");
        assert!(attempt["process"]["launch_error"].is_string());
        assert_ne!(
            attempt["attempt_id"],
            second.raw_data["refund_attempt"]["attempt_id"]
        );
    }

    #[tokio::test]
    async fn contradictory_process_failure_is_not_approved() {
        let fixture = Fixture::new(WRITE_RESPONSE, Some("0,OK"));
        let error = fixture.adapter().refund(39000, None).await.unwrap_err();
        assert!(!error.message.contains("Успешно"));
        assert_eq!(error.raw_data["refund_attempt"]["outcome"], "unknown");
    }

    #[test]
    fn empty_result_file_is_an_error_not_a_panic() {
        let fixture = Fixture::new("exit 0", None);
        fs::write(fixture.0.join("e"), "").unwrap();
        assert!(parse_sb_pilot_e(fixture.0.join("e")).is_err());
    }
}

#[cfg(test)]
mod result_encoding_tests {
    use super::*;

    #[test]
    fn reads_russian_results_in_both_encodings() {
        for encoding in [IBM866, WINDOWS_1251] {
            for (code, reason) in [
                (2000, "Операция прервана клиентом"),
                (4476, "Нет исходной операции"),
                (362, "Ошибка чтения карты"),
                (99, "НЕТ СВЯЗИ С ТЕРМИНАЛОМ"),
                (12, "Операция не разрешена (код 12)"),
            ] {
                let input = format!("{code},{reason}\r\n");
                let (bytes, _, errors) = encoding.encode(&input);
                assert!(!errors);
                let result = parse_sb_pilot_e_bytes(&bytes).unwrap();
                assert_eq!(result.result_code, code);
                assert_eq!(result.result_text, reason, "{}", encoding.name());
            }
        }
    }

    #[test]
    fn reads_ascii_result_without_encoding_specific_characters() {
        let result = parse_sb_pilot_e_bytes(b"362,Card read error\r\n").unwrap();
        assert_eq!(result.result_code, 362);
        assert_eq!(result.result_text, "Card read error");
    }

    #[test]
    fn reads_cp866_cancellation_from_real_result_line() {
        // First line of the supplied e file; no card or transaction details.
        let bytes = b"\x32\x30\x30\x30\x2c\x8e\xaf\xa5\xe0\xa0\xe6\xa8\xef\x20\xaf\xe0\xa5\xe0\xa2\xa0\xad\xa0\x20\xaa\xab\xa8\xa5\xad\xe2\xae\xac\x0d\x0a";
        let result = parse_sb_pilot_e_bytes(bytes).unwrap();
        assert_eq!(result.result_code, 2000);
        assert_eq!(result.result_text, "Операция прервана клиентом");
    }
}
