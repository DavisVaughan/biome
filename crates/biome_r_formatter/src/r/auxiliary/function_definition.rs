use crate::prelude::*;
use biome_r_syntax::RFunctionDefinition;
use biome_rowan::AstNode;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRFunctionDefinition;
impl FormatNodeRule<RFunctionDefinition> for FormatRFunctionDefinition {
    fn fmt_fields(&self, node: &RFunctionDefinition, f: &mut RFormatter) -> FormatResult<()> {
        format_verbatim_node(node.syntax()).fmt(f)
    }
}
