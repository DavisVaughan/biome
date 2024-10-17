use crate::prelude::*;
use biome_r_syntax::RParameter;
use biome_rowan::AstNode;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRParameter;
impl FormatNodeRule<RParameter> for FormatRParameter {
    fn fmt_fields(&self, node: &RParameter, f: &mut RFormatter) -> FormatResult<()> {
        format_verbatim_node(node.syntax()).fmt(f)
    }
}
