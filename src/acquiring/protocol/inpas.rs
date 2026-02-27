use crate::ConnectionType;
use crate::acquiring::protocol::base::Acquiring;
use crate::acquiring::response::build_terminal_response_from_raw;
use crate::acquiring::types::{ConnectionConfig, TerminalResponse};
use crate::healthcheck::HealthcheckResult;
use crate::{ProcessSuccess, ProcessError};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use quick_xml::Writer;
use quick_xml::events::{BytesStart, Event};
use std::io::Cursor;

const DEFAULT_ENCODING: &str = "windows-1251";

pub struct InpasAdapter {
    config: ConnectionConfig,
}

#[derive(Debug, Clone)]
pub struct InpasField {
    pub id: String,
    pub value: String,
}

pub struct EnvelopeOptions {
    timeout: Option<u32>,
    ipaddr: Option<String>,
    ncom: Option<String>,
    baudrate: Option<u32>,
}

impl InpasAdapter {
    pub fn new(config: ConnectionConfig) -> Self {
        Self { config }
    }

    fn build_inpas_fields(&self, mut fields: Vec<InpasField>) -> Vec<InpasField> {
        let has_timestamp = fields.iter().any(|f| f.id == "21");
        let has_serial = fields.iter().any(|f| f.id == "27");

        if !has_timestamp {
            fields.push(InpasField {
                id: "21".to_string(),
                value: self.get_current_timestamp(),
            });
        }

        if !has_serial {
            fields.push(InpasField {
                id: "27".to_string(),
                value: self.config.serial_number.clone(),
            });
        }

        fields
    }

    fn get_current_timestamp(&self) -> String {
        use chrono::Local;
        let now = Local::now();
        now.format("%Y%m%d%H%M%S").to_string()
    }

    fn build_inpas_xml(&self, fields: &[InpasField], meta: EnvelopeOptions) -> Result<String> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        writer.write_event(Event::Decl(quick_xml::events::BytesDecl::new(
            "1.0",
            Some(DEFAULT_ENCODING),
            None,
        )))?;

        let request_elem = BytesStart::new("request");
        writer.write_event(Event::Start(request_elem.clone()))?;

        for field in fields {
            let mut field_elem = BytesStart::new("field");
            field_elem.push_attribute(("id", field.id.as_str()));
            writer.write_event(Event::Start(field_elem.clone()))?;
            writer.write_event(Event::Text(quick_xml::events::BytesText::new(&field.value)))?;
            writer.write_event(Event::End(field_elem.to_end()))?;
        }

        if let Some(timeout) = meta.timeout {
            writer.write_event(Event::Start(BytesStart::new("timeout")))?;
            writer.write_event(Event::Text(quick_xml::events::BytesText::new(
                &timeout.to_string(),
            )))?;
            writer.write_event(Event::End(BytesStart::new("timeout").to_end()))?;
        }

        if let Some(ipaddr) = &meta.ipaddr {
            writer.write_event(Event::Start(BytesStart::new("ipaddr")))?;
            writer.write_event(Event::Text(quick_xml::events::BytesText::new(ipaddr)))?;
            writer.write_event(Event::End(BytesStart::new("ipaddr").to_end()))?;
        }

        if let Some(ncom) = &meta.ncom {
            writer.write_event(Event::Start(BytesStart::new("ncom")))?;
            writer.write_event(Event::Text(quick_xml::events::BytesText::new(ncom)))?;
            writer.write_event(Event::End(BytesStart::new("ncom").to_end()))?;
        }

        if let Some(baudrate) = meta.baudrate {
            writer.write_event(Event::Start(BytesStart::new("baudrate")))?;
            writer.write_event(Event::Text(quick_xml::events::BytesText::new(
                &baudrate.to_string(),
            )))?;
            writer.write_event(Event::End(BytesStart::new("baudrate").to_end()))?;
        }

        writer.write_event(Event::End(request_elem.to_end()))?;

        let result = writer.into_inner().into_inner();
        let xml_string = encoding_rs::WINDOWS_1251.decode(&result).0.to_string();
        Ok(xml_string)
    }

    pub async fn send_inpas_request(&self, fields: &[InpasField]) -> Result<TerminalResponse> {
        let dc_host = &self
            .config
            .dc_host
            .as_ref()
            .ok_or(anyhow!("dcHost property is required for inpas protocol"))?;

        let mut envelope = EnvelopeOptions {
            timeout: self.config.timeout,
            ipaddr: None,
            ncom: None,
            baudrate: None,
        };

        match &self.config.connection_type {
            ConnectionType::Tcp => {
                let address = self.config.address.as_ref().ok_or(anyhow!(
                    "Fields address and port are required for tcp connection in inpas mode"
                ))?;
                let port = self.config.port.ok_or(anyhow!(
                    "Fields address and port are required for tcp connection in inpas mode"
                ))?;
                envelope.ipaddr = Some(format!("{}:{}", address, port));
            }
            ConnectionType::Usb => {
                let ncom = self.config.address.as_ref().ok_or(
                anyhow!("Fields ncom and baudrate (USB port) are required for usb connection in inpas mode"),
            )?;
                let baudrate = self.config.port.ok_or(
                anyhow!("Fields ncom and baudrate (USB port) are required for usb connection in inpas mode"),
            )?;
                envelope.ncom = Some(ncom.clone());
                envelope.baudrate = Some(baudrate);
            }
            _ => {}
        }

        let xml_body = self.build_inpas_xml(fields, envelope)?;
        let response = self.post_xml(dc_host, &xml_body).await?;
        self.parse_inpas_response(&response)
    }

    fn parse_inpas_response(&self, xml: &str) -> Result<TerminalResponse> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut data = std::collections::HashMap::new();
        let mut error_code: Option<String> = None;
        let mut error_description: Option<String> = None;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.name().as_ref() {
                    b"field" => {
                        let mut id = None;
                        let mut value = String::new();

                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"id" {
                                id = Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }

                        let mut text_buf = Vec::new();
                        loop {
                            match reader.read_event_into(&mut text_buf)? {
                                Event::Text(t) => {
                                    value.push_str(&String::from_utf8_lossy(&t.into_inner()));
                                }
                                Event::End(e) if e.name().as_ref() == b"field" => break,
                                _ => {}
                            }
                        }

                        if let Some(id) = id {
                            data.insert(format!("{:0>2}", id), value);
                        }
                    }
                    b"errorcode" => {
                        let mut text_buf = Vec::new();
                        loop {
                            match reader.read_event_into(&mut text_buf)? {
                                Event::Text(t) => {
                                    error_code =
                                        Some(String::from_utf8_lossy(&t.into_inner()).to_string());
                                }
                                Event::End(e) if e.name().as_ref() == b"errorcode" => break,
                                _ => {}
                            }
                        }
                    }
                    b"errordescription" | b"errorDescription" => {
                        let mut text_buf = Vec::new();
                        loop {
                            match reader.read_event_into(&mut text_buf)? {
                                Event::Text(t) => {
                                    error_description =
                                        Some(String::from_utf8_lossy(&t.into_inner()).to_string());
                                }
                                Event::End(e)
                                    if e.name().as_ref() == b"errordescription"
                                        || e.name().as_ref() == b"errorDescription" =>
                                {
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        if let Some(code) = error_code {
            if !code.is_empty() {
                let error_msg = error_description
                    .as_ref()
                    .filter(|d| !d.is_empty())
                    .map(|d| d.clone())
                    .unwrap_or_else(|| format!("DualConnector error code {}", code));
                return Ok(TerminalResponse {
                    success: false,
                    error: Some(error_msg),
                    code: Some(code),
                    message: error_description,
                    data: None,
                });
            }
        }

        Ok(build_terminal_response_from_raw(
            crate::acquiring::types::ProtocolType::Inpas,
            data,
        ))
    }

    async fn post_xml(&self, url_str: &str, xml_body: &str) -> Result<String> {
        let url = self.normalize_dc_url(url_str)?;
        let body_bytes: Vec<u8> = encoding_rs::WINDOWS_1251.encode(xml_body).0.to_vec();

        let client = reqwest::Client::new();
        let request = client
            .post(url.clone())
            .header(
                "Content-Type",
                format!("text/xml; charset={}", DEFAULT_ENCODING),
            )
            .header("Accept", "text/xml")
            .header("Accept-Charset", DEFAULT_ENCODING)
            .header("User-Agent", "corex-ttk2")
            .body(body_bytes);

        let response = request.send().await?;

        let status = response.status();
        let content_type_header = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        let bytes = response.bytes().await?;

        if status.is_client_error() || status.is_server_error() {
            let text = String::from_utf8_lossy(&bytes);
            return Err(anyhow!("DualConnector HTTP error {}: {}", status.as_u16(), text).into());
        }
        let charset = extract_charset(content_type_header.as_deref());
        let decoded = match charset {
            Some("windows-1251") | Some("cp1251") => {
                encoding_rs::WINDOWS_1251.decode(&bytes).0.to_string()
            }
            _ => String::from_utf8_lossy(&bytes).to_string(),
        };

        Ok(decoded)
    }

    fn normalize_dc_url(&self, host: &str) -> Result<reqwest::Url> {
        if host.starts_with("http://") || host.starts_with("https://") {
            Ok(host.parse()?)
        } else {
            Ok(format!("http://{}", host).parse()?)
        }
    }
}

#[async_trait]
impl Acquiring for InpasAdapter {
    async fn connected(&self) -> bool {
        true
    }

    async fn connect(&mut self) -> std::result::Result<ProcessSuccess<bool>, ProcessError> {
        Ok(ProcessSuccess::new("CONNECT_SUCCESSFUL", true))
    }

    async fn disconnect(&mut self) -> std::result::Result<ProcessSuccess<()>, ProcessError> {
        Ok(ProcessSuccess::new("DISCONNECT_SUCCESSFUL", ()))
    }

    async fn payment(&mut self, amount: u64, currency: Option<String>) -> std::result::Result<ProcessSuccess<TerminalResponse>, ProcessError> {
        let mut fields = self.build_inpas_fields(vec![
            InpasField {
                id: "00".to_string(),
                value: amount.to_string(),
            },
            InpasField {
                id: "25".to_string(),
                value: "1".to_string(),
            },
        ]);

        if let Some(c) = currency {
            fields.push(InpasField {
                id: "04".to_string(),
                value: c.clone(),
            });
        }

        self.send_inpas_request(&fields).await
            .map(|resp| ProcessSuccess::new("PAYMENT_SUCCESSFUL", resp))
            .map_err(|e| ProcessError::new("PAYMENT_FAIL", "Не удалось провести оплату", e.to_string()))
    }

    async fn refund(&mut self, amount: u64, currency: Option<String>) -> std::result::Result<ProcessSuccess<TerminalResponse>, ProcessError> {
        let mut fields = self.build_inpas_fields(vec![
            InpasField {
                id: "00".to_string(),
                value: amount.to_string(),
            },
            InpasField {
                id: "25".to_string(),
                value: "29".to_string(),
            },
        ]);

        if let Some(c) = currency {
            fields.push(InpasField {
                id: "04".to_string(),
                value: c.clone(),
            });
        }

        self.send_inpas_request(&fields).await
            .map(|resp| ProcessSuccess::new("REFUND_SUCCESSFUL", resp))
            .map_err(|e| ProcessError::new("REFUND_FAIL", "Не удалось выполнить возврат", e.to_string()))
    }

    async fn totals(&mut self) -> std::result::Result<ProcessSuccess<TerminalResponse>, ProcessError> {
        let fields = self.build_inpas_fields(vec![InpasField {
            id: "25".to_string(),
            value: "59".to_string(),
        }]);

        self.send_inpas_request(&fields).await
            .map(|resp| ProcessSuccess::new("TOTALS_SUCCESSFUL", resp))
            .map_err(|e| ProcessError::new("TOTALS_FAIL", "Не удалось выполнить сверку итогов", e.to_string()))
    }

    async fn healthcheck(&self) -> HealthcheckResult {
        HealthcheckResult::success()
    }
}

fn extract_charset(content_type: Option<&str>) -> Option<&str> {
    content_type?.split(';').find_map(|part| {
        let part = part.trim();
        if part.to_lowercase().starts_with("charset=") {
            Some(&part[8..])
        } else {
            None
        }
    })
}
