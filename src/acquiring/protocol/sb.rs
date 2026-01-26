use anyhow::{Result, anyhow};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use async_trait::async_trait;

use crate::{
    ConnectionConfig, ConnectionType, TerminalResponse, acquiring::protocol::base::Acquiring,
};

pub struct SBAdapter {
    config: ConnectionConfig,
    dir: PathBuf,
}

impl SBAdapter {
    pub fn new(config: ConnectionConfig, dir: PathBuf) -> Self {
        Self { config, dir }
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
        dbg!(self.get_cmd()?);

        let pinpad_ini = dbg!(self.get_pinpad_ini()?);
        let mut ini_editor = IniEditor::load(&pinpad_ini)?;

        match &self.config.connection_type {
            ConnectionType::Usb => {
                let edits = dbg!(self.configure_usb()?);
                ini_editor.edit_many(edits);
            }
            ConnectionType::Tcp => {
                let edits = dbg!(self.configure_tcp()?);
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
        cmd.args(&["1", &format!("{}", amount)]);

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

    async fn refund(&mut self, amount: u64, _: Option<String>) -> Result<TerminalResponse> {
        let mut cmd = self.get_cmd()?;
        cmd.args(&["3", &format!("{}", amount)]);

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

pub struct IniEditor {
    lines: Vec<String>,
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

    pub fn to_string(&self) -> String {
        self.lines.join("\n")
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
