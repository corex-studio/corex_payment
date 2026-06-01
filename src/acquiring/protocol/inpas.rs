use crate::ConnectionType;
use crate::acquiring::protocol::base::Acquiring;
use crate::acquiring::response::normalize_terminal_response;
use crate::acquiring::types::{ConnectionConfig, NormalizedTransactionData};
use crate::healthcheck::HealthcheckResult;
use crate::{ProcessError, ProcessSuccess};
use async_trait::async_trait;
use quick_xml::Writer;
use quick_xml::events::{BytesStart, Event};
use serde_json::Value;
use std::io::Cursor;
use std::result::Result;

const DEFAULT_ENCODING: &str = "windows-1251";

pub mod inpas_prop_codes {
    pub const AMOUNT: &str = "00";
    pub const CURRENCY: &str = "04";
    pub const DATETIME_HOST: &str = "06";
    pub const PAN: &str = "10";
    pub const AUTHORIZATION_CODE: &str = "13";
    pub const REFERENCE_NUMBER: &str = "14";
    pub const TERMINAL_DATETIME: &str = "21";
    pub const TRXID: &str = "23";
    pub const OPERATION_CODE: &str = "25";
    pub const TERMINAL_TRX_ID: &str = "26";
    pub const TERMINAL_ID: &str = "27";
    pub const MERCHANT_ID: &str = "28";
    pub const STATUS: &str = "39";
}

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

    pub async fn send_inpas_request(
        &self,
        fields: &[InpasField],
    ) -> Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let dc_host = self.config.dc_host.as_ref().ok_or(ProcessError::new(
            "dcHost property is required for inpas protocol",
            Value::Null,
        ))?;

        let mut envelope = EnvelopeOptions {
            timeout: self.config.timeout,
            ipaddr: None,
            ncom: None,
            baudrate: None,
        };

        match &self.config.connection_type {
            ConnectionType::Tcp => {
                let address = self.config.address.as_ref().ok_or(ProcessError::new(
                    "Fields address and port are required for tcp connection in inpas mode",
                    Value::Null,
                ))?;
                let port = self.config.port.ok_or(ProcessError::new(
                    "Fields address and port are required for tcp connection in inpas mode",
                    Value::Null,
                ))?;
                envelope.ipaddr = Some(format!("{}:{}", address, port));
            }
            ConnectionType::Usb => {
                let ncom = self.config.address.as_ref().ok_or(
                ProcessError::new("Fields ncom and baudrate (USB port) are required for usb connection in inpas mode", Value::Null),
            )?;
                let baudrate = self.config.port.ok_or(
                ProcessError::new("Fields ncom and baudrate (USB port) are required for usb connection in inpas mode", Value::Null),
            )?;
                envelope.ncom = Some(ncom.clone());
                envelope.baudrate = Some(baudrate);
            }
            _ => {}
        }

        let xml_body = self.build_inpas_xml(fields, envelope)?;
        let response = self.post_xml(dc_host, &xml_body).await?;
        self.parse_inpas_response(&response, xml_body)
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

    fn build_inpas_xml(
        &self,
        fields: &[InpasField],
        meta: EnvelopeOptions,
    ) -> anyhow::Result<String> {
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

        let now = chrono::Utc::now();
        let session_id = now.timestamp_millis().to_string();
        writer.write_event(Event::Start(BytesStart::new("sessionID")))?;
        writer.write_event(Event::Text(quick_xml::events::BytesText::new(&session_id)))?;
        writer.write_event(Event::End(BytesStart::new("sessionID").to_end()))?;

        writer.write_event(Event::End(request_elem.to_end()))?;

        let result = writer.into_inner().into_inner();
        let xml_string = encoding_rs::WINDOWS_1251.decode(&result).0.to_string();
        Ok(xml_string)
    }

    fn parse_inpas_response(
        &self,
        xml: &str,
        input_data: String,
    ) -> Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut data = std::collections::HashMap::new();
        let mut error_code: Option<String> = None;
        let mut error_description: Option<String> = None;
        let mut buf = Vec::new();

        loop {
            let read_event = reader.read_event_into(&mut buf).map_err(|e| {
                ProcessError::new(
                    "Could not read XML response from Inpas".to_string(),
                    Value::String(e.to_string()),
                )
            })?;

            match read_event {
                Event::Start(e) => match e.name().as_ref() {
                    b"field" => {
                        let mut id = None;
                        let mut value = String::new();

                        for attr in e.attributes() {
                            let attr = attr.map_err(|e| {
                                ProcessError::new(
                                    "Could not read XML attr".to_string(),
                                    Value::String(e.to_string()),
                                )
                            })?;
                            if attr.key.as_ref() == b"id" {
                                id = Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }

                        let mut text_buf = Vec::new();
                        loop {
                            let read_event_node =
                                reader.read_event_into(&mut text_buf).map_err(|e| {
                                    ProcessError::new(
                                        "Could not read XML node data".to_string(),
                                        Value::String(e.to_string()),
                                    )
                                })?;

                            match read_event_node {
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
                            let read_event_node =
                                reader.read_event_into(&mut text_buf).map_err(|e| {
                                    ProcessError::new(
                                        "Could not read XML node data".to_string(),
                                        Value::String(e.to_string()),
                                    )
                                })?;

                            match read_event_node {
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
                            let read_event_node =
                                reader.read_event_into(&mut text_buf).map_err(|e| {
                                    ProcessError::new(
                                        "Could not read XML node data".to_string(),
                                        Value::String(e.to_string()),
                                    )
                                })?;

                            match read_event_node {
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

        if let Some(code) = error_code
            && !code.is_empty()
        {
            let error_msg = error_description
                .as_ref()
                .filter(|d| !d.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("DualConnector error code {}", code));

            return Err(ProcessError::new_with_input(
                format!("Inpas request has failed: {code}"),
                Value::String(error_msg),
                Some(input_data),
            ));
        }

        fn parse_datetime(s: &str) -> String {
            chrono::NaiveDateTime::parse_from_str(s, "%Y%m%d%H%M%S")
                .map(|dt| dt.and_utc().timestamp_millis().to_string())
                .unwrap_or_else(|_| s.to_string())
        }
        if let Some(v) = data.get(inpas_prop_codes::DATETIME_HOST) {
            data.insert(inpas_prop_codes::DATETIME_HOST.to_string(), parse_datetime(v));
        }
        if let Some(v) = data.get(inpas_prop_codes::TERMINAL_DATETIME) {
            data.insert(inpas_prop_codes::TERMINAL_DATETIME.to_string(), parse_datetime(v));
        }

        let normalized_data = normalize_terminal_response(crate::ProtocolType::Inpas, &data);
        Ok(ProcessSuccess::new(
            "Successful Inpas request",
            normalized_data,
            Value::String(xml.to_string()),
        ))
    }

    async fn post_xml(&self, url_str: &str, xml_body: &str) -> Result<String, ProcessError> {
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
            return Err(ProcessError::new(
                format!("DualConnector HTTP error {}", status.as_u16()),
                Value::String(text.to_string()),
            ));
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

    fn normalize_dc_url(&self, host: &str) -> Result<reqwest::Url, ProcessError> {
        let mut host_str = host.to_string();
        if !host.starts_with("http://") && !host.starts_with("https://") {
            host_str = format!("http://{}", host);
        }

        match host_str.parse() {
            Ok(s) => Ok(s),
            Err(e) => Err(ProcessError::new(
                "Could not parse DC host url",
                Value::String(e.to_string()),
            )),
        }
    }
}

#[async_trait]
impl Acquiring for InpasAdapter {
    async fn connected(&self) -> bool {
        true
    }

    async fn connect(&mut self) -> Result<ProcessSuccess<bool>, ProcessError> {
        Ok(ProcessSuccess::new(
            "No need to connect since Inpas works via HTTP requests".to_string(),
            true,
            Value::Bool(true),
        ))
    }

    async fn disconnect(&mut self) -> Result<ProcessSuccess<()>, ProcessError> {
        Ok(ProcessSuccess::new(
            "No need to disconnect since Inpas works via HTTP requests".to_string(),
            (),
            Value::Null,
        ))
    }

    async fn payment(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let mut fields = self.build_inpas_fields(vec![
            InpasField {
                id: inpas_prop_codes::AMOUNT.to_string(),
                value: amount.to_string(),
            },
            InpasField {
                id: inpas_prop_codes::OPERATION_CODE.to_string(),
                value: "1".to_string(),
            },
        ]);

        if let Some(c) = currency {
            fields.push(InpasField {
                id: inpas_prop_codes::CURRENCY.to_string(),
                value: c.clone(),
            });
        }

        self.send_inpas_request(&fields).await
    }

    async fn refund(
        &mut self,
        amount: u64,
        currency: Option<String>,
    ) -> Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let mut fields = self.build_inpas_fields(vec![
            InpasField {
                id: inpas_prop_codes::AMOUNT.to_string(),
                value: amount.to_string(),
            },
            InpasField {
                id: inpas_prop_codes::OPERATION_CODE.to_string(),
                value: "29".to_string(),
            },
        ]);

        if let Some(c) = currency {
            fields.push(InpasField {
                id: inpas_prop_codes::CURRENCY.to_string(),
                value: c.clone(),
            });
        }

        self.send_inpas_request(&fields).await
    }

    async fn totals(&mut self) -> Result<ProcessSuccess<NormalizedTransactionData>, ProcessError> {
        let fields = self.build_inpas_fields(vec![InpasField {
            id: inpas_prop_codes::OPERATION_CODE.to_string(),
            value: "59".to_string(),
        }]);

        self.send_inpas_request(&fields).await
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
