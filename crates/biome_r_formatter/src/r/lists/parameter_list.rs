use crate::prelude::*;
use crate::separated::FormatAstSeparatedListExtension;
use biome_formatter::separated::TrailingSeparator;
use biome_r_syntax::RParameterList;
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatRParameterList;
impl FormatRule<RParameterList> for FormatRParameterList {
    type Context = RFormatContext;
    fn fmt(&self, _node: &RParameterList, _f: &mut RFormatter) -> FormatResult<()> {
        unreachable!("Implemented through `RParameters` calling `FormatRAnyParameterList::new()` on the `RParameterList`");
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct FormatRAnyParameterList<'a> {
    list: &'a RParameterList,
}

impl<'a> FormatRAnyParameterList<'a> {
    pub fn new(list: &'a RParameterList) -> Self {
        Self { list }
    }
}

impl Format<RFormatContext> for FormatRAnyParameterList<'_> {
    fn fmt(&self, f: &mut Formatter<RFormatContext>) -> FormatResult<()> {
        let mut joiner = f.join_nodes_with_soft_line();
        join_parameter_list(&mut joiner, &self.list)?;
        joiner.finish()
    }
}

fn join_parameter_list<S>(
    joiner: &mut JoinNodesBuilder<'_, '_, S, RFormatContext>,
    list: &RParameterList,
) -> FormatResult<()>
where
    S: Format<RFormatContext>,
{
    let entries = list
        .format_separated(",")
        .with_trailing_separator(TrailingSeparator::Disallowed)
        .zip(list.iter());

    for (format_entry, node) in entries {
        joiner.entry(node?.syntax(), &format_entry);
    }

    Ok(())
}
