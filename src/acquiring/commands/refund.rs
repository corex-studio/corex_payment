use crate::acquiring::commands::base::{execute_command, CommandContext};
use crate::acquiring::protocol::inpas::InpasField;
use crate::acquiring::protocol::TlvItem;
use crate::acquiring::types::{get_tag_definition, TerminalResponse};

pub struct RefundCommand {
    amount: u64,
    currency: String,
}

impl RefundCommand {
    pub fn new(amount: u64, currency: Option<String>) -> Self {
        Self {
            amount,
            currency: currency.unwrap_or_else(|| "643".to_string()),
        }
    }

    fn prepare(&self, context: &CommandContext) -> Vec<TlvItem> {
        let message_id_tag = get_tag_definition(0x01).expect("MESSAGE_ID tag not found");
        let ern_tag = get_tag_definition(0x03).expect("ERN tag not found");
        let transaction_amount_tag =
            get_tag_definition(0x04).expect("TRANSACTION_AMOUNT tag not found");

        vec![
            TlvItem {
                tag: message_id_tag.tag,
                length: 3,
                value: context.string_to_bytes("REF"),
                definition: Some(message_id_tag),
            },
            TlvItem {
                tag: ern_tag.tag,
                length: 10,
                value: context.int_to_bcd(context.ern, 10),
                definition: Some(ern_tag),
            },
            TlvItem {
                tag: transaction_amount_tag.tag,
                length: 12,
                value: context.string_to_bytes(&context.int_to_string(self.amount, 12)),
                definition: Some(transaction_amount_tag),
            },
        ]
    }

    fn prepare_inpas(&self, _context: &CommandContext) -> Vec<InpasField> {
        vec![
            InpasField {
                id: "00".to_string(),
                value: self.amount.to_string(),
            },
            InpasField {
                id: "04".to_string(),
                value: self.currency.clone(),
            },
            InpasField {
                id: "25".to_string(),
                value: "29".to_string(),
            },
        ]
    }

    pub async fn execute(
        self,
        context: CommandContext,
    ) -> Result<TerminalResponse, Box<dyn std::error::Error>> {
        execute_command(
            context,
            |ctx| self.prepare(ctx),
            |ctx| self.prepare_inpas(ctx),
        )
        .await
    }
}

