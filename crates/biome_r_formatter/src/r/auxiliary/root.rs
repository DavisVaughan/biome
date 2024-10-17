use crate::prelude::*;
use biome_r_syntax::RRoot;
use biome_rowan::AstNode;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRRoot;
impl FormatNodeRule<RRoot> for FormatRRoot {
    fn fmt_fields(&self, node: &RRoot, f: &mut RFormatter) -> FormatResult<()> {
        format_verbatim_node(node.syntax()).fmt(f)
    }
}
