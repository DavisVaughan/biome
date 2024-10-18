use crate::prelude::*;
use biome_formatter::format_args;
use biome_formatter::write;
use biome_r_syntax::RBinaryExpression;
use biome_r_syntax::RBinaryExpressionFields;

#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRBinaryExpression;
impl FormatNodeRule<RBinaryExpression> for FormatRBinaryExpression {
    fn fmt_fields(&self, node: &RBinaryExpression, f: &mut RFormatter) -> FormatResult<()> {
        let RBinaryExpressionFields {
            left,
            operator_token_token,
            right,
        } = node.as_fields();

        write!(
            f,
            [group(&format_args![
                left.format(),
                space(),
                operator_token_token.format(),
                soft_line_break_or_space(),
                right.format()
            ])]
        )
    }
}
