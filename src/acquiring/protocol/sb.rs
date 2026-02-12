use anyhow::{Result, anyhow};
use serde::Serialize;
use std::{
    fmt::Display, fs, path::{Path, PathBuf}, pin, process::Command
};

use async_trait::async_trait;
use encoding_rs::WINDOWS_1251;

use crate::{
    ConnectionConfig, ConnectionType, TerminalResponse,
    acquiring::{protocol::base::Acquiring, types::NormalizedTransactionData},
};

pub struct SBAdapter {
    config: ConnectionConfig,
    dir: PathBuf,
}

impl SBAdapter {
    pub fn new(config: ConnectionConfig, dir: PathBuf) -> Self {
        Self { config, dir }
    }

    pub fn read_e(&self) -> Result<SbPilotE> {
        parse_sb_pilot_e(self.dir.join("e"))
        // let e_data = self.dir.join("e");
        // if !e_data.exists() {
        //     return Err(anyhow!("Missing e file"));
        // }
        //
        // let bytes = fs::read(e_data)?;
        // Ok(extract_strings(&bytes))
    }

    fn get_pilot(&self) -> Result<PathBuf> {
        let sb_pilot = self.dir.join("sb_pilot.exe");

        match sb_pilot.exists() {
            true => Ok(sb_pilot),
            false => Err(anyhow!("Missing sb_pilot.exe")),
        }
    }

    fn get_pinpad_ini(&self) -> Result<PathBuf> {
        let pinpad_ini = self.dir.join("pinpad.ini");

        match pinpad_ini.exists() {
            true => Ok(pinpad_ini),
            false => Err(anyhow!("Missing pinpad.ini")),
        }
    }

    fn get_cmd(&self) -> Result<Command> {
        match self.get_pilot() {
            Ok(v) => Ok(Command::new(v)),
            Err(e) => Err(anyhow!(e)),
        }
    }

    fn configure_usb(&self) -> Result<Vec<IniEdit>> {
        let com = match &self.config.ncom {
            Some(v) => keep_only_digits(v.clone()),
            None => return Err(anyhow!("Param ncom is missing")),
        };

        let baudrate = match self.config.baudrate {
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
        self.get_cmd().is_ok()
    }

    async fn connect(&mut self) -> Result<bool> {
        self.get_cmd()?;

        let pinpad_ini = self.get_pinpad_ini()?;
        println!("sc552 path: {:?}", pinpad_ini);
        let mut ini_editor = IniEditor::load(&pinpad_ini)?;

        match &self.config.connection_type {
            ConnectionType::Usb => {
                let edits = self.configure_usb()?;
                ini_editor.edit_many(edits);
            }
            ConnectionType::Tcp => {
                let edits = self.configure_tcp()?;
                ini_editor.edit_many(edits);
            }
            _ => return Err(anyhow!("Not yet implemented")),
        };
        ini_editor.save(pinpad_ini)?;

        Ok(true)
    }

    async fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }

    async fn payment(&mut self, amount: u64, _: Option<String>) -> Result<TerminalResponse> {
        let mut cmd = self.get_cmd()?;
        cmd.args(["1", &format!("{}", amount)]);

        let res = cmd.output()?;
        let success = res.status.success();
        let code = res.status.code().map(|v| v.to_string());

        let e_strings = self.read_e();
        let data = match e_strings {
            Ok(_) => Some(NormalizedTransactionData::empty()),
            Err(_) => None,
        };

        Ok(TerminalResponse {
            success,
            code,
            data,
            message: None,
            error: None,
        })
    }

    async fn refund(&mut self, amount: u64, _: Option<String>) -> Result<TerminalResponse> {
        let mut cmd = self.get_cmd()?;
        cmd.args(["3", &format!("{}", amount)]);

        let res = cmd.output()?;
        let success = res.status.success();
        let code = res.status.code().map(|v| v.to_string());

        Ok(TerminalResponse {
            success,
            code,
            data: None,
            message: None,
            error: None,
        })
    }

    async fn totals(&mut self) -> Result<TerminalResponse> {
        let mut cmd = self.get_cmd()?;
        cmd.arg("7");

        let res = cmd.output()?;
        let success = res.status.success();
        let code = res.status.code().map(|v| v.to_string());

        Ok(TerminalResponse {
            success,
            code,
            data: None,
            message: None,
            error: None,
        })
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

fn extract_strings(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();
    let mut buf = Vec::new();

    for &b in data {
        if (0x20..=0x7E).contains(&b) {
            buf.push(b);
        } else {
            if buf.len() >= 3 {
                strings.push(String::from_utf8_lossy(&buf).to_string());
            }
            buf.clear();
        }
    }

    strings
}

fn keep_only_digits(s: String) -> String {
    // Итератор по char + filter + collect в новый String
    s.chars().filter(|c| c.is_numeric()).collect()
}

#[derive(Debug, Serialize)]
pub struct SbPilotE {
    pub result_code: i32,
    pub result_text: String,

    pub masked_pan_or_phone: Option<String>,
    pub terminal_serial: Option<String>,
    pub card_expiry: Option<String>,
    pub auth_code: Option<String>,
    pub operation_id: Option<String>,
    pub card_type: Option<String>,
    pub is_sber_card: Option<bool>,
    pub terminal_id: Option<String>,
    pub datetime: Option<String>,
    pub rrn: Option<String>,
    pub card_hash: Option<String>,
    pub bonus_amount: Option<u64>,
    pub merchant_id: Option<String>,
    pub monitoring_type: Option<String>,
    pub monitoring_state: Option<String>,
    pub monitoring_message: Option<String>,
    pub loyalty_program: Option<u32>,
    pub user_reply: Option<String>,
    pub request_id: Option<String>,
    pub flags: Option<u32>,
    pub mifare_loyalty: Option<String>,
    pub has_vas: Option<bool>,
    pub hash_type: Option<String>,
    pub extended_hash: Option<String>,
    pub par: Option<String>,
    pub card_type_id: Option<String>,
    pub entry_mode: Option<String>,
    pub sbp_url: Option<String>,
    pub sbp_order_id: Option<String>,
    pub vendor_terminal_serial: Option<String>,
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
    let (code, text) = lines[0]
        .split_once(',')
        .ok_or(anyhow!("Invalid result line"))?;

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
        result_text: text.to_string(),

        masked_pan_or_phone: None,
        terminal_serial: None,
        card_expiry: None,
        auth_code: None,
        operation_id: None,
        card_type: None,
        is_sber_card: None,
        terminal_id: None,
        datetime: None,
        rrn: None,
        card_hash: None,
        bonus_amount: None,
        merchant_id: None,
        monitoring_type: None,
        monitoring_state: None,
        monitoring_message: None,
        loyalty_program: None,
        user_reply: None,
        request_id: None,
        flags: None,
        mifare_loyalty: None,
        has_vas: None,
        hash_type: None,
        extended_hash: None,
        par: None,
        card_type_id: None,
        entry_mode: None,
        sbp_url: None,
        sbp_order_id: None,
        vendor_terminal_serial: None,
    };

    if result_code != 0 {
        return Ok(e);
    }

    e.masked_pan_or_phone = next();
    e.terminal_serial = next();
    e.card_expiry = next();
    e.auth_code = next();
    e.operation_id = next();
    e.card_type = next();
    e.is_sber_card = next().map(|v| v == "1");
    e.terminal_id = next();
    e.datetime = next();
    e.rrn = next();
    next(); // sb_pilot version (skip)
    e.card_hash = next();
    next(); // track3
    e.bonus_amount = next().and_then(|v| v.parse().ok());
    e.merchant_id = next();
    e.monitoring_type = next();
    e.monitoring_state = next();
    e.monitoring_message = next();
    e.loyalty_program = next().and_then(|v| v.parse().ok());
    e.user_reply = next();
    e.request_id = next();
    e.flags = next().and_then(|v| u32::from_str_radix(&v, 16).ok());
    e.mifare_loyalty = next();
    e.has_vas = next().map(|v| v == "1");
    e.hash_type = next();
    e.extended_hash = next();
    e.par = next();
    e.card_type_id = next();
    e.entry_mode = next();
    e.sbp_url = next();
    e.sbp_order_id = next();
    e.vendor_terminal_serial = next();

    Ok(e)
}
