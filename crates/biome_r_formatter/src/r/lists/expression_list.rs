use crate::prelude::*;
use biome_r_syntax::RExpressionList;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRExpressionList;
impl FormatRule<RExpressionList> for FormatRExpressionList {
    type Context = RFormatContext;
    fn fmt(&self, node: &RExpressionList, f: &mut RFormatter) -> FormatResult<()> {
        f.join().entries(node.iter().formatted()).finish()
    }
}
