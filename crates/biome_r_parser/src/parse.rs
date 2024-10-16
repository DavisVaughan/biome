use biome_parser::event::Event;
use biome_parser::prelude::ParseDiagnostic;
use biome_parser::prelude::Trivia;
use biome_r_syntax::RSyntaxKind;
use biome_rowan::TextRange;
use biome_rowan::TextSize;
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
        parse_events(ast)
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

fn parse_events(ast: Tree) -> (Vec<Event<RSyntaxKind>>, Vec<Trivia>, Vec<ParseDiagnostic>) {
    let mut walker = RWalk::new(ast);
    walker.walk();
    walker.parse.drain()
}

/// Given an ast with absolutely no ERROR or MISSING nodes, let's walk that tree
/// and collect our `trivia` and `events`.
struct RWalk {
    ast: Tree,
    parse: RParse,
}

impl RWalk {
    fn new(ast: Tree) -> Self {
        Self {
            ast,
            parse: RParse::new(),
        }
    }

    fn walk(&mut self) {
        for event in self.ast.root_node().preorder() {
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
                            // TODO: Trivia for comments
                            ()
                        }
                        NodeSyntaxKind::Leaf(kind) => {
                            self.parse.token(kind, node.end_byte().try_into().unwrap());
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_smoke_test() {
        let (events, _trivia, _errors) = parse("1+1");

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
    }
}
