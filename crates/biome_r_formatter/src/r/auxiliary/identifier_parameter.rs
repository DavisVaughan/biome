use crate::prelude::*;
use biome_r_syntax::RIdentifierParameter;
use biome_rowan::AstNode;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRIdentifierParameter;
impl FormatNodeRule<RIdentifierParameter> for FormatRIdentifierParameter {
    fn fmt_fields(&self, node: &RIdentifierParameter, f: &mut RFormatter) -> FormatResult<()> {
        format_verbatim_node(node.syntax()).fmt(f)
    }
}
