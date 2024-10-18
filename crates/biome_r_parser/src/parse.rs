use biome_parser::event::Event;
use biome_parser::prelude::ParseDiagnostic;
use biome_parser::prelude::Trivia;
use biome_parser::AnyParse;
use biome_r_syntax::RSyntaxKind;
use biome_rowan::NodeCache;
use biome_rowan::TextRange;
use biome_rowan::TextSize;
use biome_rowan::TriviaPieceKind;
use biome_unicode_table::Dispatch;
use tree_sitter::Tree;

use crate::treesitter::NodeSyntaxKind;
use crate::treesitter::NodeTypeExt;
use crate::treesitter::WalkEvent;
use crate::RLosslessTreeSink;
use crate::RParserOptions;

// TODO(r): These should really return an intermediate `Parse` type which
// can `.into()` an `AnyParse`, see `biome_js_parser`'s `Parse` type
pub fn parse(text: &str, options: RParserOptions) -> AnyParse {
    let mut cache = NodeCache::default();
    parse_r_with_cache(text, options, &mut cache)
}

pub fn parse_r_with_cache(text: &str, options: RParserOptions, cache: &mut NodeCache) -> AnyParse {
    tracing::debug_span!("parse").in_scope(move || {
        let (events, tokens, errors) = parse_text(text, options);
        let mut tree_sink = RLosslessTreeSink::with_cache(text, &tokens, cache);
        biome_parser::event::process(&mut tree_sink, events, errors);
        let (green, parse_errors) = tree_sink.finish();
        AnyParse::new(green.as_send().unwrap(), parse_errors)
    })
}

pub fn parse_text(
    text: &str,
    _options: RParserOptions,
) -> (Vec<Event<RSyntaxKind>>, Vec<Trivia>, Vec<ParseDiagnostic>) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_r::LANGUAGE.into())
        .unwrap();

    let ast = parser.parse(text, None).unwrap();

    if ast.root_node().has_error() {
        // TODO: In the long term we want an error resiliant parser.
        // This would probably only be able to happen if we swap out tree sitter
        // for a hand written recursive descent pratt parser using the Biome infra.
        return parse_failure();
    }

    parse_tree(ast, text)
}

fn parse_failure() -> (Vec<Event<RSyntaxKind>>, Vec<Trivia>, Vec<ParseDiagnostic>) {
    let events = vec![];
    let trivia = vec![];
    let span: Option<TextRange> = None;
    let error = ParseDiagnostic::new("Tree-sitter failed", span);
    let errors = vec![error];
    (events, trivia, errors)
}

fn parse_tree(
    ast: Tree,
    text: &str,
) -> (Vec<Event<RSyntaxKind>>, Vec<Trivia>, Vec<ParseDiagnostic>) {
    let mut walker = RWalk::new(ast, text);
    walker.walk();
    walker.parse.drain()
}

/// Given an ast with absolutely no ERROR or MISSING nodes, let's walk that tree
/// and collect our `trivia` and `events`.
struct RWalk<'src> {
    ast: Tree,
    text: &'src str,
    parse: RParse,
}

impl<'src> RWalk<'src> {
    fn new(ast: Tree, text: &'src str) -> Self {
        Self {
            ast,
            text,
            parse: RParse::new(),
        }
    }

    fn walk(&mut self) {
        let mut last_end = TextSize::from(0);

        // We are currently between the start of file and the first token
        let mut between_two_tokens = false;

        let root = self.ast.root_node();
        let mut iter = root.preorder();

        while let Some(event) = iter.next() {
            match event {
                WalkEvent::Enter(node) => {
                    match node.syntax_kind() {
                        NodeSyntaxKind::Comment => {
                            // We handle comments on `Leave`
                            ()
                        }
                        NodeSyntaxKind::Token(_) => {
                            // We handle tokens on `Leave`
                            ()
                        }
                        NodeSyntaxKind::Value(kind, _) => {
                            // Open the node kind
                            self.parse.start(kind);
                        }
                        NodeSyntaxKind::Node(kind) => {
                            self.parse.start(kind);
                            if kind == RSyntaxKind::R_ROOT {
                                self.parse.push_event(Event::Start {
                                    kind: RSyntaxKind::R_EXPRESSION_LIST,
                                    forward_parent: None,
                                });
                            }
                        }
                    }
                }
                WalkEvent::Leave(node) => {
                    match node.syntax_kind() {
                        NodeSyntaxKind::Comment => {
                            let this_start = TextSize::try_from(node.start_byte()).unwrap();
                            let this_end = TextSize::try_from(node.end_byte()).unwrap();
                            let gap = &self.text[usize::from(last_end)..usize::from(this_start)];

                            let mut trailing = between_two_tokens;

                            if gap.contains('\n') {
                                // If the gap has a newline this is a leading comment
                                trailing = false;
                                self.parse.derive_trivia(gap, last_end, between_two_tokens);
                            } else {
                                // Otherwise we're just after a token and this is a trailing comment,
                                // unless we are at the beginning of the document, in which case
                                // the whitespace and comment are leading.
                                //
                                // We also make sure we don't add an empty whitespace trivia.
                                if this_start != last_end {
                                    self.parse.trivia.push(Trivia::new(
                                        TriviaPieceKind::Whitespace,
                                        TextRange::new(last_end, this_start),
                                        trailing,
                                    ));
                                }
                            }

                            // Comments are "single line" even if they are consecutive
                            self.parse.trivia.push(Trivia::new(
                                TriviaPieceKind::SingleLineComment,
                                TextRange::new(this_start, this_end),
                                trailing,
                            ));

                            last_end = this_end;
                        }

                        NodeSyntaxKind::Token(kind) => {
                            // TODO!: Don't unwrap()
                            let this_start = TextSize::try_from(node.start_byte()).unwrap();
                            let this_end = TextSize::try_from(node.end_byte()).unwrap();

                            // TS gives us all tokens except trivia. So we know
                            // all the relevant trivia tokens are laid out
                            // between the last token's end and this token's
                            // start.
                            let gap = &self.text[usize::from(last_end)..usize::from(this_start)];

                            self.parse.derive_trivia(gap, last_end, between_two_tokens);
                            self.parse.token(kind, this_end);

                            last_end = this_end;
                            between_two_tokens = true;
                        }

                        NodeSyntaxKind::Value(_, kind) => {
                            // TODO!: Don't unwrap()
                            let this_start = TextSize::try_from(node.start_byte()).unwrap();
                            let this_end = TextSize::try_from(node.end_byte()).unwrap();

                            // TS gives us all tokens except trivia. So we know
                            // all the relevant trivia tokens are laid out
                            // between the last token's end and this token's
                            // start.
                            let gap = &self.text[usize::from(last_end)..usize::from(this_start)];

                            self.parse.derive_trivia(gap, last_end, between_two_tokens);

                            // Push the token
                            self.parse.token(kind, this_end);

                            // Then close the node
                            self.parse.finish();

                            last_end = this_end;
                            between_two_tokens = true;
                        }

                        NodeSyntaxKind::Node(kind) => {
                            match kind {
                                RSyntaxKind::R_ROOT => {
                                    // Finish expression list
                                    self.parse.finish();

                                    // No longer between two tokens.
                                    // Now between last token and EOF.
                                    between_two_tokens = false;

                                    // TODO!: Don't unwrap()
                                    let this_end = TextSize::try_from(node.end_byte()).unwrap();
                                    let gap =
                                        &self.text[usize::from(last_end)..usize::from(this_end)];

                                    // Derive trivia between last token and end of document.
                                    // It is always leading trivia of the `EOF` token,
                                    // which `TreeSink` adds for us.
                                    self.parse.derive_trivia(gap, last_end, between_two_tokens);

                                    // Finish node
                                    self.parse.finish();
                                }
                                _ => {
                                    // Finish node
                                    self.parse.finish();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

struct RParse {
    events: Vec<Event<RSyntaxKind>>,
    trivia: Vec<Trivia>,
    errors: Vec<ParseDiagnostic>,
}

impl RParse {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            trivia: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn start(&mut self, kind: RSyntaxKind) {
        self.push_event(Event::Start {
            kind,
            forward_parent: None,
        });
    }

    fn token(&mut self, kind: RSyntaxKind, end: TextSize) {
        self.push_event(Event::Token { kind, end });
    }

    fn finish(&mut self) {
        self.push_event(Event::Finish);
    }

    fn push_event(&mut self, event: Event<RSyntaxKind>) {
        self.events.push(event);
    }

    fn push_trivia(&mut self, trivia: Trivia) {
        self.trivia.push(trivia);
    }

    fn drain(self) -> (Vec<Event<RSyntaxKind>>, Vec<Trivia>, Vec<ParseDiagnostic>) {
        (self.events, self.trivia, self.errors)
    }

    // TODO!: Need to handle comments too. It will be like `derive_trivia()`
    // but whitespace after the final token on a line but before a trailing
    // comment is also considered trailing trivia (I think the trick to
    // recognize is that any whitespace before a comment is considered trailing
    // until you see your first newline)

    /// Given:
    /// - A slice of `text` starting at byte `start`
    /// - Which only contains whitespace or newlines
    /// - And represents a "gap" between two tokens
    ///
    /// Derive the implied stream of trivia that exists in that gap
    ///
    /// SAFETY: `last_end <= next_start`
    /// SAFETY: `last_end` and `next_start` must define a range within `text`
    fn derive_trivia(&mut self, text: &str, mut start: TextSize, between_two_tokens: bool) {
        let mut iter = text.as_bytes().iter().peekable();
        let mut end = start;

        // - Between the start of file and the first token, all trivia is leading
        //   (it leads the first token), so we skip this.
        // - Between the last token and the end of file, all trivia is leading
        //   (it leads the EOF token that `TreeSink` adds), so we skip this.
        // - Between two tokens, all trivia is leading unless we see a newline,
        //   which this branch handles specially.
        if between_two_tokens {
            let mut trailing = false;

            // All whitespace between two tokens is leading until we hit the
            // first `\r`, `\r\n`, or `\n`, at which point the whitespace is
            // considered trailing of the last token, and the newline and
            // everything after it is considered leading of the next token.
            // A lone `\r` not attached to an `\n` should not happen in a
            // well-formed file (unless inside a string token), so we just
            // treat it as a `\r\n` line ending.
            while let Some(byte) = iter.peek() {
                if let b'\r' | b'\n' = byte {
                    // We found a newline, so all trivia up to this point is
                    // trailing to the last token. Don't advance the iterator so
                    // that this newline may be processed as leading trivia.
                    trailing = true;

                    // Break and fallthrough
                    break;
                }
                end += TextSize::from(1);
                let _ = iter.next();
            }

            if start != end {
                let range = TextRange::new(start, end);
                self.push_trivia(Trivia::new(TriviaPieceKind::Whitespace, range, trailing));
                start = end;
            }

            // Fallthrough so that our current byte can be processed as leading
            // trivia
        }

        // Now push all leading trivia
        let trailing = false;

        while let Some(byte) = iter.next() {
            end += TextSize::from(1);

            if Self::is_whitespace(*byte) {
                // Finish out stream of whitespace
                while let Some(_) = iter.next_if(|byte| Self::is_whitespace(**byte)) {
                    end += TextSize::from(1);
                }
                let range = TextRange::new(start, end);
                self.push_trivia(Trivia::new(TriviaPieceKind::Whitespace, range, trailing));
                start = end;
                continue;
            }

            if let b'\r' = byte {
                match iter.next_if(|byte| **byte == b'\n') {
                    Some(_) => {
                        // Finish out `\r\n`
                        end += TextSize::from(1);
                        let range = TextRange::new(start, end);
                        self.push_trivia(Trivia::new(TriviaPieceKind::Newline, range, trailing));
                        start = end;
                    }
                    None => {
                        // Finish out `\r`
                        let range = TextRange::new(start, end);
                        self.push_trivia(Trivia::new(TriviaPieceKind::Newline, range, trailing));
                        start = end;
                    }
                }
                continue;
            }

            if let b'\n' = byte {
                // Finish out `\n`
                let range = TextRange::new(start, end);
                self.push_trivia(Trivia::new(TriviaPieceKind::Newline, range, trailing));
                start = end;
                continue;
            }

            unreachable!("Detected non trivia character!");
        }
    }

    fn is_whitespace(byte: u8) -> bool {
        // `WHS` maps newlines as "whitespace" but we handle that specially
        match biome_unicode_table::lookup_byte(byte) {
            Dispatch::WHS => byte != b'\r' && byte != b'\n',
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    enum Pos {
        Leading,
        Trailing,
    }

    fn trivia(text: &str) -> Vec<Trivia> {
        let (_events, trivia, _errors) = parse_text(text, RParserOptions::default());
        trivia
    }

    fn ws(start: u32, end: u32, position: Pos) -> Trivia {
        Trivia::new(
            TriviaPieceKind::Whitespace,
            TextRange::new(TextSize::from(start), TextSize::from(end)),
            matches!(position, Pos::Trailing),
        )
    }

    fn nl(start: u32, end: u32) -> Trivia {
        Trivia::new(
            TriviaPieceKind::Newline,
            TextRange::new(TextSize::from(start), TextSize::from(end)),
            false,
        )
    }

    fn cmt(start: u32, end: u32, position: Pos) -> Trivia {
        Trivia::new(
            TriviaPieceKind::SingleLineComment,
            TextRange::new(TextSize::from(start), TextSize::from(end)),
            matches!(position, Pos::Trailing),
        )
    }

    #[test]
    fn test_parse_smoke_test() {
        let (events, trivia, _errors) = parse_text("1+1", RParserOptions::default());

        let expect = vec![
            Event::Start {
                kind: RSyntaxKind::R_ROOT,
                forward_parent: None,
            },
            Event::Start {
                kind: RSyntaxKind::R_EXPRESSION_LIST,
                forward_parent: None,
            },
            Event::Start {
                kind: RSyntaxKind::R_BINARY_EXPRESSION,
                forward_parent: None,
            },
            Event::Start {
                kind: RSyntaxKind::R_DOUBLE_VALUE,
                forward_parent: None,
            },
            Event::Token {
                kind: RSyntaxKind::R_DOUBLE_LITERAL,
                end: TextSize::from(1),
            },
            Event::Finish,
            Event::Token {
                kind: RSyntaxKind::PLUS,
                end: TextSize::from(2),
            },
            Event::Start {
                kind: RSyntaxKind::R_DOUBLE_VALUE,
                forward_parent: None,
            },
            Event::Token {
                kind: RSyntaxKind::R_DOUBLE_LITERAL,
                end: TextSize::from(3),
            },
            Event::Finish,
            Event::Finish,
            Event::Finish,
            Event::Finish,
        ];

        assert_eq!(events, expect);
        assert!(trivia.is_empty());
    }

    #[test]
    fn test_parse_trivia_smoke_test() {
        assert_eq!(
            trivia("1 + 1"),
            vec![ws(1, 2, Pos::Leading), ws(3, 4, Pos::Leading)]
        );
    }

    #[test]
    fn test_parse_trivia_tab_test() {
        assert_eq!(
            trivia("1\t+\t\n\t1"),
            vec![
                ws(1, 2, Pos::Leading),
                ws(3, 4, Pos::Trailing),
                nl(4, 5),
                ws(5, 6, Pos::Leading)
            ]
        );
    }

    #[test]
    fn test_parse_trivia_trailing_test() {
        assert_eq!(
            trivia("1 + \n1"),
            vec![ws(1, 2, Pos::Leading), ws(3, 4, Pos::Trailing), nl(4, 5)]
        );
    }

    #[test]
    fn test_parse_trivia_trailing_trivia_test() {
        // Note that trivia between the last token and `EOF` is always
        // leading and will be attached to an `EOF` token by `TreeSink`.
        assert_eq!(
            trivia("1  \n "),
            vec![ws(1, 3, Pos::Leading), nl(3, 4), ws(4, 5, Pos::Leading)]
        );
    }

    #[test]
    fn test_parse_trivia_trailing_crlf_test() {
        assert_eq!(
            trivia("1 + \r\n1"),
            vec![ws(1, 2, Pos::Leading), ws(3, 4, Pos::Trailing), nl(4, 6)]
        );
    }

    #[test]
    fn test_parse_trivia_before_first_token() {
        assert_eq!(trivia("  \n1"), vec![ws(0, 2, Pos::Leading), nl(2, 3)]);
    }

    #[test]
    fn test_parse_trivia_comment_test() {
        assert_eq!(
            trivia("1 #"),
            vec![ws(1, 2, Pos::Trailing), cmt(2, 3, Pos::Trailing)]
        );
    }

    #[test]
    fn test_parse_trivia_comment_nothing_else_test() {
        assert_eq!(trivia("#"), vec![cmt(0, 1, Pos::Leading)]);
    }

    #[test]
    fn test_parse_trivia_comment_end_of_document_test() {
        assert_eq!(trivia("1\n#"), vec![nl(1, 2), cmt(2, 3, Pos::Leading)]);
    }

    #[test]
    fn test_parse_trivia_whitespace_between_comments_test() {
        let text = "
1 #
#
2
"
        .trim();
        assert_eq!(
            trivia(text),
            vec![
                ws(1, 2, Pos::Trailing),
                cmt(2, 3, Pos::Trailing),
                nl(3, 4),
                cmt(4, 5, Pos::Leading),
                nl(5, 6),
            ]
        );
    }

    #[test]
    fn test_parse_trivia_comment_beginning_of_document_test() {
        assert_eq!(trivia("#\n1"), vec![cmt(0, 1, Pos::Leading), nl(1, 2)]);
    }

    #[test]
    fn test_parse_trivia_comment_beginning_of_document_with_whitespace_test() {
        assert_eq!(
            trivia(" \n \n#"),
            vec![
                ws(0, 1, Pos::Leading),
                nl(1, 2),
                ws(2, 3, Pos::Leading),
                nl(3, 4),
                cmt(4, 5, Pos::Leading),
            ]
        );
    }
}
