use crate::acquiring::commands::base::{execute_command, CommandContext};
use crate::acquiring::protocol::inpas::InpasField;
use crate::acquiring::protocol::TlvItem;
use crate::acquiring::types::{get_tag_definition, TerminalResponse};

pub struct TotalsCommand;

impl TotalsCommand {
    pub fn new() -> Self {
        Self
    }

    fn prepare(&self, context: &CommandContext) -> Vec<TlvItem> {
        let message_id_tag = get_tag_definition(0x01).expect("MESSAGE_ID tag not found");
        let ern_tag = get_tag_definition(0x03).expect("ERN tag not found");
        let srv_subfunction_tag =
            get_tag_definition(0x1a).expect("SRV_SUBFUNCTION tag not found");

        vec![
            TlvItem {
                tag: message_id_tag.tag,
                length: 3,
                value: context.string_to_bytes("SRV"),
                definition: Some(message_id_tag),
            },
            TlvItem {
                tag: ern_tag.tag,
                length: 10,
                value: context.int_to_bcd(context.ern, 10),
                definition: Some(ern_tag),
            },
            TlvItem {
                tag: srv_subfunction_tag.tag,
                length: 1,
                value: context.string_to_bytes("2"),
                definition: Some(srv_subfunction_tag),
            },
        ]
    }

    fn prepare_inpas(&self, _context: &CommandContext) -> Vec<InpasField> {
        vec![InpasField {
            id: "25".to_string(),
            value: "59".to_string(),
        }]
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

