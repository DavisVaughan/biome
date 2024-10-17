use crate::prelude::*;
use biome_r_syntax::RDoubleValue;
use biome_rowan::AstNode;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRDoubleValue;
impl FormatNodeRule<RDoubleValue> for FormatRDoubleValue {
    fn fmt_fields(&self, node: &RDoubleValue, f: &mut RFormatter) -> FormatResult<()> {
        format_verbatim_node(node.syntax()).fmt(f)
    }
}
