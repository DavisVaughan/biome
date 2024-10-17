use crate::prelude::*;
use biome_r_syntax::RBinaryExpression;
use biome_rowan::AstNode;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRBinaryExpression;
impl FormatNodeRule<RBinaryExpression> for FormatRBinaryExpression {
    fn fmt_fields(&self, node: &RBinaryExpression, f: &mut RFormatter) -> FormatResult<()> {
        format_verbatim_node(node.syntax()).fmt(f)
    }
}
