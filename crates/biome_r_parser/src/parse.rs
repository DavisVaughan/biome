use biome_parser::event::Event;
use biome_parser::prelude::ParseDiagnostic;
use biome_parser::prelude::Trivia;
use biome_r_syntax::RSyntaxKind;
use biome_rowan::TextRange;
use biome_rowan::TextSize;
use biome_rowan::TriviaPieceKind;
use biome_unicode_table::Dispatch;
use tree_sitter::Node;
use tree_sitter::Tree;

use crate::treesitter::NodeSyntaxKind;
use crate::treesitter::NodeTypeExt;
use crate::treesitter::WalkEvent;

pub fn parse(text: &str) -> (Vec<Event<RSyntaxKind>>, Vec<Trivia>, Vec<ParseDiagnostic>) {
    let mut parser = tree_sitter::Parser::new();

    parser
        .set_language(&tree_sitter_r::LANGUAGE.into())
        .unwrap();

    let ast = parser.parse(text, None).unwrap();
    let root = ast.root_node();

    if root.has_error() {
        parse_failure(&root)
    } else {
        parse_events(ast, text)
    }
}

fn parse_failure(root: &Node) -> (Vec<Event<RSyntaxKind>>, Vec<Trivia>, Vec<ParseDiagnostic>) {
    let events = vec![];
    let trivia = vec![];

    let start = u32::try_from(root.start_byte()).unwrap();
    let end = u32::try_from(root.end_byte()).unwrap();
    let span = TextRange::new(TextSize::from(start), TextSize::from(end));
    let error = ParseDiagnostic::new("Tree-sitter failed", span);
    let errors = vec![error];

    (events, trivia, errors)
}

fn parse_events(
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
        let mut before_first_token = true;

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
                        NodeSyntaxKind::Leaf(_) => {
                            // We handle leaves on `Leave`
                            ()
                        }
                        NodeSyntaxKind::Node(kind) => self.parse.start(kind),
                    }
                }
                WalkEvent::Leave(node) => {
                    match node.syntax_kind() {
                        NodeSyntaxKind::Comment => {
                            let this_start = TextSize::try_from(node.start_byte()).unwrap();
                            let this_end = TextSize::try_from(node.end_byte()).unwrap();
                            let gap = &self.text[usize::from(last_end)..usize::from(this_start)];

                            let mut trailing = !before_first_token;

                            if gap.contains('\n') {
                                // If the gap has a newline this is a leading comment
                                trailing = false;
                                self.parse.derive_trivia(gap, last_end, before_first_token);
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

                            // Comments are "single line" event if they are consecutive
                            self.parse.trivia.push(Trivia::new(
                                TriviaPieceKind::SingleLineComment,
                                TextRange::new(this_start, this_end),
                                trailing,
                            ));

                            last_end = this_end;
                        }

                        NodeSyntaxKind::Leaf(kind) => {
                            // TODO!: Don't unwrap()
                            let this_start = TextSize::try_from(node.start_byte()).unwrap();
                            let this_end = TextSize::try_from(node.end_byte()).unwrap();

                            // TS gives us all tokens except trivia. So we know
                            // all the relevant trivia tokens are laid out
                            // between the last token's end and this token's
                            // start.
                            let gap = &self.text[usize::from(last_end)..usize::from(this_start)];

                            self.parse.derive_trivia(gap, last_end, before_first_token);
                            self.parse.token(kind, this_end);

                            last_end = this_end;
                            before_first_token = false;
                        }

                        NodeSyntaxKind::Node(_) => self.parse.finish(),
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

    fn push_error(&mut self, error: ParseDiagnostic) {
        self.errors.push(error);
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
    fn derive_trivia(&mut self, text: &str, mut start: TextSize, only_leading: bool) {
        let mut iter = text.as_bytes().iter().peekable();
        let mut end = start;

        // First detect trailing trivia for the last token. Can't have trailing
        // trivia before the first token, so if that's the case then all trivia
        // is leading trivia and we skip this step.
        if !only_leading {
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
        let (_events, trivia, _errors) = parse(text);
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
        let (events, trivia, _errors) = parse("1+1");

        let expect = vec![
            Event::Start {
                kind: RSyntaxKind::R_ROOT,
                forward_parent: None,
            },
            Event::Start {
                kind: RSyntaxKind::R_BINARY_EXPRESSION,
                forward_parent: None,
            },
            Event::Token {
                kind: RSyntaxKind::R_DOUBLE_VALUE,
                end: TextSize::from(1),
            },
            Event::Token {
                kind: RSyntaxKind::PLUS,
                end: TextSize::from(2),
            },
            Event::Token {
                kind: RSyntaxKind::R_DOUBLE_VALUE,
                end: TextSize::from(3),
            },
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
            trivia("1 # foo"),
            vec![ws(1, 2, Pos::Trailing), cmt(2, 7, Pos::Trailing)]
        );
    }

    #[test]
    fn test_parse_trivia_comment_beginning_of_document_test() {
        assert_eq!(trivia("# foo\n1"), vec![cmt(0, 5, Pos::Leading), nl(5, 6)]);
    }

    #[test]
    fn test_parse_trivia_comment_beginning_of_document_with_whitespace_test() {
        assert_eq!(
            trivia(" \n \n# foo"),
            vec![
                ws(0, 1, Pos::Leading),
                nl(1, 2),
                ws(2, 3, Pos::Leading),
                nl(3, 4),
                cmt(4, 9, Pos::Leading),
            ]
        );
    }
}
