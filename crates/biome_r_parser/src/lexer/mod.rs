use biome_parser::{diagnostic::ParseDiagnostic, lexer::{Lexer, LexerCheckpoint}};
use biome_r_syntax::{RSyntaxKind, RSyntaxKind::*, TextSize};

// TODO: Work on the lexer, figure out the connection to tree-sitter

pub(crate) struct RLexer<'src> {
    /// Source text
    source: &'src str,

    /// The start byte position in the source text of the next token.
    position: usize,

    /// the current token
    current_kind: RSyntaxKind,

    /// diagnostics emitted during the parsing phase
    diagnostics: Vec<ParseDiagnostic>,
}

// impl<'src> Lexer<'src> for RLexer<'src> {
//     const NEWLINE: Self::Kind = NEWLINE;
//
//     const WHITESPACE: Self::Kind = WHITESPACE;
//     type Kind = RSyntaxKind;
//     type LexContext = RLexContext;
//     type ReLexContext = RReLexContext;
//
//     fn source(&self) -> &'src str {
//         self.source
//     }
//
//     fn current(&self) -> Self::Kind {
//         self.current_kind
//     }
//
//     fn position(&self) -> usize {
//         self.position
//     }
//
//     fn current_start(&self) -> TextSize {
//         // TODO
//     }
//
//     fn push_diagnostic(&mut self, diagnostic: ParseDiagnostic) {
//         self.diagnostics.push(diagnostic);
//     }
//
//     fn next_token(&mut self, context: Self::LexContext) -> Self::Kind {
//         // TODO
//     }
//
//     fn has_preceding_line_break(&self) -> bool {
//         // TODO
//         return false;
//     }
//
//     fn has_unicode_escape(&self) -> bool {
//         // TODO
//         return false;
//     }
//
//     fn rewind(&mut self, checkpoint: LexerCheckpoint<Self::Kind>) {
//         let LexerCheckpoint {
//             position,
//             current_start: _,
//             current_flags: _,
//             current_kind,
//             after_line_break: _,
//             unicode_bom_length: _,
//             diagnostics_pos,
//         } = checkpoint;
//
//         let new_pos = u32::from(position) as usize;
//
//         self.position = new_pos;
//         self.current_kind = current_kind;
//         self.diagnostics.truncate(diagnostics_pos as usize);
//     }
//
//     fn finish(self) -> Vec<ParseDiagnostic> {
//         self.diagnostics
//     }
// }
