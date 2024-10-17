use crate::prelude::*;
use biome_r_syntax::RParameters;
use biome_rowan::AstNode;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRParameters;
impl FormatNodeRule<RParameters> for FormatRParameters {
    fn fmt_fields(&self, node: &RParameters, f: &mut RFormatter) -> FormatResult<()> {
        format_verbatim_node(node.syntax()).fmt(f)
    }
}
