use crate::acquiring::protocol::tlv::{TlvEncoder, TlvItem};
use crate::acquiring::types::MessageType;

pub struct TtkBuffer;

impl TtkBuffer {
    pub fn create_message(message_type: MessageType, items: &[TlvItem]) -> Vec<u8> {
        let tlv_data: Vec<u8> = items
            .iter()
            .flat_map(|item| TlvEncoder::encode(item.tag, &item.value))
            .collect();

        let message_length = tlv_data.len() + 2;
        let mut header = vec![0u8; 4];
        header[0] = ((message_length >> 8) & 0xff) as u8;
        header[1] = (message_length & 0xff) as u8;
        header[2] = ((message_type as u16 >> 8) & 0xff) as u8;
        header[3] = (message_type as u16 & 0xff) as u8;

        let mut message = Vec::with_capacity(header.len() + tlv_data.len());
        message.extend_from_slice(&header);
        message.extend_from_slice(&tlv_data);
        message
    }

    pub fn parse_message(data: &[u8]) -> Result<(MessageType, Vec<TlvItem>), String> {
        if data.len() < 4 {
            return Err("Invalid message length".to_string());
        }

        let length = ((data[0] as u16) << 8) | (data[1] as u16);
        let message_type = ((data[2] as u16) << 8) | (data[3] as u16);

        if data.len() != (length as usize + 2) {
            return Err("Message length mismatch".to_string());
        }

        let tlv_data = &data[4..];
        let items = TlvEncoder::decode(tlv_data)?;

        let msg_type = match message_type {
            0x96f2 => MessageType::ClientRequest,
            0x97f2 => MessageType::ServerResponse,
            _ => return Err(format!("Unknown message type: 0x{:04x}", message_type)),
        };

        Ok((msg_type, items))
    }

    pub fn items_to_object(items: &[TlvItem]) -> std::collections::HashMap<String, String> {
        let mut result = std::collections::HashMap::new();

        for item in items {
            let key = if let Some(def) = item.definition {
                def.name.clone()
            } else {
                format!("TAG_{:X}", item.tag)
            };
            result.insert(key, TlvEncoder::value_to_string(item));
        }

        result
    }
}

