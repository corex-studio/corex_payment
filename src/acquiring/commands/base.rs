use crate::acquiring::connection::BaseConnection;
use crate::acquiring::protocol::inpas::{InpasField, send_inpas_request};
use crate::acquiring::protocol::{TlvItem, TtkBuffer};
use crate::acquiring::response::build_terminal_response_from_raw;
use crate::acquiring::types::{
    ConnectionConfig, MessageType, ProtocolType, TerminalResponse, get_tag_definition,
};
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;

pub trait BaseCommand: Send + Sync {
    fn prepare(&self) -> Vec<TlvItem>;
    fn prepare_inpas_fields(&self) -> Vec<InpasField> {
        panic!("Inpas protocol is not implemented for this command")
    }

    fn execute(
        &mut self,
    ) -> impl Future<Output = Result<TerminalResponse, Box<dyn std::error::Error>>>;
}

pub struct CommandContext {
    pub connection: Arc<Mutex<Box<dyn BaseConnection>>>,
    pub request_id: u32,
    pub ern: u64,
}

impl CommandContext {
    pub fn new(connection: Arc<Mutex<Box<dyn BaseConnection>>>) -> Self {
        let mut rng = rand::thread_rng();
        let rid = rand::thread_rng().r#gen::<u32>() & 0x7fffffff;
        Self {
            connection,
            request_id: rid,
            ern: rng.gen_range(0..9999999999),
        }
    }

    pub fn config(&self) -> ConnectionConfig {
        let conn = self
            .connection
            .try_lock()
            .expect("Failed to lock connection");
        conn.config().clone()
    }

    pub fn should_use_inpas(&self) -> bool {
        matches!(self.config().protocol, ProtocolType::Inpas)
    }

    pub fn build_inpas_fields(&self, mut fields: Vec<InpasField>) -> Vec<InpasField> {
        let has_timestamp = fields.iter().any(|f| f.id == "21");
        let has_serial = fields.iter().any(|f| f.id == "27");
        let config = self.config();

        if !has_timestamp {
            fields.push(InpasField {
                id: "21".to_string(),
                value: self.get_current_timestamp(),
            });
        }

        if !has_serial {
            fields.push(InpasField {
                id: "27".to_string(),
                value: config.serial_number.clone(),
            });
        }

        fields
    }

    fn get_current_timestamp(&self) -> String {
        use chrono::Local;
        let now = Local::now();
        now.format("%Y%m%d%H%M%S").to_string()
    }

    pub fn int_to_bcd(&self, value: u64, length: usize) -> Vec<u8> {
        let str_value = format!("{:0width$}", value, width = length);
        str_value
            .chars()
            .map(|c| c.to_digit(10).unwrap() as u8)
            .collect()
    }

    pub fn string_to_bytes(&self, str: &str) -> Vec<u8> {
        str.as_bytes().to_vec()
    }

    pub fn int_to_string(&self, number: u64, len: usize) -> String {
        format!("{:0width$}", number, width = len)
    }
}

pub async fn execute_command(
    context: CommandContext,
    prepare_fn: impl Fn(&CommandContext) -> Vec<TlvItem>,
    prepare_inpas_fn: impl Fn(&CommandContext) -> Vec<InpasField>,
) -> Result<TerminalResponse, Box<dyn std::error::Error>> {
    if context.should_use_inpas() {
        if context.config().dc_host.is_none() {
            return Err("dcHost property is required for inpas protocol".into());
        }

        let fields = prepare_inpas_fn(&context);
        let fields = context.build_inpas_fields(fields);
        let config = context.config();
        return send_inpas_request(&config, &fields).await;
    }

    let mut items = prepare_fn(&context);
    let config = context.config();
    let ecr_tag = get_tag_definition(0x02).ok_or("ECR_NUMBER tag not found")?;
    items.insert(
        0,
        TlvItem {
            tag: ecr_tag.tag,
            length: 8,
            value: context.string_to_bytes(&config.serial_number),
            definition: Some(ecr_tag),
        },
    );

    let message = TtkBuffer::create_message(MessageType::ClientRequest, &items);
    {
        let mut conn = context.connection.lock().await;
        conn.write(&message).await?;
    }

    let timeout_ms = context.config().timeout.unwrap_or(30000);
    loop {
        let response_data = {
            let mut conn = context.connection.lock().await;
            conn.read(Some(timeout_ms)).await?
        };
        let (response_type, response_items) = TtkBuffer::parse_message(&response_data)?;

        if response_type == MessageType::ServerResponse {
            if response_items.iter().any(|item| {
                item.definition
                    .map(|d| d.name == "Response Code")
                    .unwrap_or(false)
            }) {
                let response_data = TtkBuffer::items_to_object(&response_items);
                return Ok(build_terminal_response_from_raw(
                    ProtocolType::Ttk,
                    response_data,
                ));
            }
        }
    }
}
