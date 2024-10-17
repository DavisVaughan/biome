use crate::prelude::*;
use biome_r_syntax::RParameterList;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRParameterList;
impl FormatRule<RParameterList> for FormatRParameterList {
    type Context = RFormatContext;
    fn fmt(&self, node: &RParameterList, f: &mut RFormatter) -> FormatResult<()> {
        format_verbatim_node(node.syntax()).fmt(f)
    }
}
