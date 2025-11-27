use crate::acquiring::types::{get_tag_definition, DataType, Encoding, TagDefinition};

#[derive(Debug, Clone)]
pub struct TlvItem {
    pub tag: u32,
    pub length: usize,
    pub value: Vec<u8>,
    pub definition: Option<&'static TagDefinition>,
}

pub struct TlvEncoder;

impl TlvEncoder {
    pub fn encode(tag: u32, value: &[u8]) -> Vec<u8> {
        let tag_bytes = Self::encode_tag(tag);
        let length_bytes = Self::encode_length(value.len());
        let mut result = Vec::with_capacity(tag_bytes.len() + length_bytes.len() + value.len());
        result.extend_from_slice(&tag_bytes);
        result.extend_from_slice(&length_bytes);
        result.extend_from_slice(value);
        result
    }

    fn encode_tag(tag: u32) -> Vec<u8> {
        if tag <= 0xff {
            vec![tag as u8]
        } else if tag <= 0xffff {
            vec![((tag >> 8) & 0xff) as u8, (tag & 0xff) as u8]
        } else {
            vec![
                ((tag >> 24) & 0xff) as u8,
                ((tag >> 16) & 0xff) as u8,
                ((tag >> 8) & 0xff) as u8,
                (tag & 0xff) as u8,
            ]
        }
    }

    fn encode_length(length: usize) -> Vec<u8> {
        if length <= 0x7f {
            vec![length as u8]
        } else if length <= 0xff {
            vec![0x81, length as u8]
        } else if length <= 0xffff {
            vec![
                0x82,
                ((length >> 8) & 0xff) as u8,
                (length & 0xff) as u8,
            ]
        } else {
            vec![
                0x83,
                ((length >> 16) & 0xff) as u8,
                ((length >> 8) & 0xff) as u8,
                (length & 0xff) as u8,
            ]
        }
    }

    pub fn decode(data: &[u8]) -> Result<Vec<TlvItem>, String> {
        let mut items = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let (tag, tag_length) = Self::decode_tag(data, offset)?;
            offset += tag_length;

            let (length, length_length) = Self::decode_length(data, offset)?;
            offset += length_length;

            if offset + length > data.len() {
                return Err("Invalid TLV data: length exceeds buffer".to_string());
            }

            let value = data[offset..offset + length].to_vec();
            offset += length;

            let definition = get_tag_definition(tag);

            items.push(TlvItem {
                tag,
                length,
                value,
                definition,
            });
        }

        Ok(items)
    }

    fn decode_tag(data: &[u8], offset: usize) -> Result<(u32, usize), String> {
        if offset >= data.len() {
            return Err("Invalid TLV data: offset out of bounds".to_string());
        }

        let mut tag = data[offset] as u32;
        let mut tag_length = 1;

        if (tag & 0x1f) == 0x1f {
            if offset + 1 >= data.len() {
                return Err("Invalid TLV data: incomplete tag".to_string());
            }
            tag_length = 2;
            tag = (tag << 8) | (data[offset + 1] as u32);

            if (data[offset + 1] & 0x80) == 0x80 {
                if offset + 2 >= data.len() {
                    return Err("Invalid TLV data: incomplete tag".to_string());
                }
                tag_length = 3;
                tag = (tag << 8) | (data[offset + 2] as u32);
            }
        }

        Ok((tag, tag_length))
    }

    fn decode_length(data: &[u8], offset: usize) -> Result<(usize, usize), String> {
        if offset >= data.len() {
            return Err("Invalid TLV data: offset out of bounds".to_string());
        }

        let first_byte = data[offset];

        if (first_byte & 0x80) == 0 {
            Ok((first_byte as usize, 1))
        } else {
            let num_bytes = (first_byte & 0x7f) as usize;
            if offset + num_bytes >= data.len() {
                return Err("Invalid TLV data: incomplete length".to_string());
            }

            let mut length = 0usize;
            for i in 0..num_bytes {
                length = (length << 8) | (data[offset + 1 + i] as usize);
            }

            Ok((length, 1 + num_bytes))
        }
    }

    pub fn value_to_string(item: &TlvItem) -> String {
        let Some(def) = item.definition else {
            return Self::bytes_to_hex(&item.value);
        };

        match def.data_type {
            DataType::String => {
                let encoding = def.encoding.unwrap_or(Encoding::Ascii);
                match encoding {
                    Encoding::Cp1251 => {
                        encoding_rs::WINDOWS_1251
                            .decode(&item.value)
                            .0
                            .to_string()
                    }
                    Encoding::Cp866 => {
                        encoding_rs::IBM866.decode(&item.value).0.to_string()
                    }
                    Encoding::Ascii => String::from_utf8_lossy(&item.value).to_string(),
                }
            }
            DataType::Bcd => item
                .value
                .iter()
                .map(|byte| format!("{:02}", byte))
                .collect::<String>(),
            DataType::Hex => Self::bytes_to_hex(&item.value),
            DataType::DwordLe => {
                let mut bytes = item.value.iter().take(4).copied().collect::<Vec<_>>();
                bytes.reverse();
                format!("0x{}", Self::bytes_to_hex(&bytes))
            }
            DataType::DwordBe => {
                format!("0x{}", Self::bytes_to_hex(&item.value.iter().take(4).copied().collect::<Vec<_>>()))
            }
            DataType::Binary => Self::bytes_to_hex(&item.value),
        }
    }

    fn bytes_to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02X}", b)).collect()
    }
}

