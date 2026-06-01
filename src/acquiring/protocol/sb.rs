use anyhow::{Result, anyhow};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use async_trait::async_trait;
use encoding_rs::WINDOWS_1251;

use crate::{
    ConnectionConfig, ConnectionType, ProcessError, ProcessSuccess,
    acquiring::{
        protocol::base::Acquiring, response::normalize_terminal_response,
        types::NormalizedTransactionData,
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

    fn configure_tcp(&self) -> Result<Vec<IniEdit>> {
        let address = match &self.config.address {
            Some(v) => v,
            None => return Err(anyhow!("Param ncom is missing")),
        };

        let port = match self.config.port {
            Some(v) => v.to_string(),
            None => "9600".to_string(),
        };

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
    async fn connected(&self) -> bool {
        self.is_connected
    }

    async fn connect(&mut self) -> std::result::Result<ProcessSuccess<bool>, ProcessError> {
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
            true,
            Value::Bool(true),
        ))
    }

    async fn disconnect(&mut self) -> std::result::Result<ProcessSuccess<()>, ProcessError> {
        self.is_connected = false;
        Ok(ProcessSuccess::new(
            "Disconnection is not actually needed for SB adapter",
            (),
            Value::Null,
        ))
    }

    async fn payment(
        &mut self,
        amount: u64,
        _: Option<String>,
    ) -> std::result::Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let mut cmd = self.get_cmd()?;
        cmd.args(["1", &format!("{}", amount)]);

        let res = cmd
            .output()
            .map_err(|e| ProcessError::from_error("Failed to run payment command", e))?;
        let success = res.status.success();

        if !success {
            let code = res.status.code().unwrap_or(-1);
            return Err(ProcessError::new_with_input(
                "Failed to run payment command",
                Value::String(format!("Operation status code: {code}")),
                Some(format!("{amount}")),
            ));
        }

        let e_strings = self.read_e().unwrap_or(SbPilotE::empty());
        let data = normalize_terminal_response(crate::ProtocolType::Ttk, &e_strings.as_hash_map());

        Ok(ProcessSuccess::new(
            "Payment command success",
            data,
            Value::String(format!("{:?}", e_strings)),
        ))
    }

    async fn refund(
        &mut self,
        amount: u64,
        _: Option<String>,
    ) -> std::result::Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let mut cmd = self.get_cmd()?;
        cmd.args(["3", &format!("{}", amount)]);

        let res = cmd
            .output()
            .map_err(|e| ProcessError::from_error("Failed to run refund command", e))?;
        let success = res.status.success();

        if !success {
            let code = res.status.code().unwrap_or(-1);
            return Err(ProcessError::new_with_input(
                "Failed to run refund command",
                Value::String(format!("Operation status code: {code}")),
                Some(format!("{amount}")),
            ));
        }

        let e_strings = self.read_e().unwrap_or(SbPilotE::empty());
        let data = normalize_terminal_response(crate::ProtocolType::Ttk, &e_strings.as_hash_map());

        Ok(ProcessSuccess::new(
            "Refund command success",
            data,
            Value::String(format!("{:?}", e_strings)),
        ))
    }

    async fn totals(
        &mut self,
    ) -> std::result::Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let mut cmd = self.get_cmd()?;
        cmd.arg("7");

        let res = cmd
            .output()
            .map_err(|e| ProcessError::from_error("Failed to run totals command", e))?;
        let success = res.status.success();

        if !success {
            let code = res.status.code().unwrap_or(-1);
            return Err(ProcessError::new(
                "Failed to run refund command",
                Value::String(format!("Operation status code: {code}")),
            ));
        }

        Ok(ProcessSuccess::new(
            "Totals command success",
            NormalizedTransactionData::empty(),
            Value::Null,
        ))
    }

    async fn healthcheck(&self) -> HealthcheckResult {
        self.run_healthcheck().await
    }
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
    let (text, _, _) = WINDOWS_1251.decode(&bytes);
    let lines: Vec<String> = text
        .into_owned()
        .lines()
        .map(|l| l.trim().to_string())
        .collect();

    // 1. Result code
    let (code, status_text) = lines[0]
        .split_once(',')
        .ok_or(anyhow!("Invalid result line"))?;
    let result_text = match code {
        "0" => "Успешно".to_string(),
        _ => status_text.to_string(),
    };

    let result_code: i32 = code.parse()?;

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
