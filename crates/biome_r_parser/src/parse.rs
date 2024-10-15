use biome_parser::event::Event;
use biome_parser::prelude::ParseDiagnostic;
use biome_parser::prelude::Trivia;
use biome_r_syntax::RSyntaxKind;
use biome_rowan::TextRange;
use biome_rowan::TextSize;
use tree_sitter::Node;
use tree_sitter::Tree;
use tree_sitter::TreeCursor;

pub enum Direction {
    Up,
    Down,
}

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

// fn parse_root(parse: &mut RParse, cursor: &mut TreeCursor) {
//     parse.start(RSyntaxKind::R_ROOT);
//     parse_next(parse, cursor);
//     parse.finish();
// }
//
// fn parse_function_definition(parse: &mut RParse, cursor: &mut TreeCursor, node: &Node) {
//     parse.start(RSyntaxKind::R_FUNCTION_DEFINITION);
//
//     let stop = node.child_by_field_name("body").unwrap();
//
//     parse.finish();
// }
//
// fn parse_next(parse: &mut RParse, cursor: &mut TreeCursor) {
//     while let Some(node) = dfs_next(cursor) {
//         match node.kind() {
//             "function_definition" => parse_function_definition(parse, cursor, &node),
//         }
//     }
// }

// // Moves the `cursor` to the next node in the tree using a depth first search.
// // Returns the node if we found one, returns `None` when we've returned to the
// // root.
// fn dfs_next<'tree>(cursor: &mut TreeCursor<'tree>) -> Option<Node<'tree>> {
//     loop {
//         if cursor.goto_first_child() {
//             return Some(cursor.node());
//         }
//
//         if cursor.goto_next_sibling() {
//             return Some(cursor.node());
//         }
//
//         loop {
//             if !cursor.goto_parent() {
//                 // Returned to root
//                 return None;
//             }
//
//             if cursor.goto_next_sibling() {
//                 return Some(cursor.node());
//             }
//         }
//     }
// }

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

    // TODO!: Flesh this out
    fn as_syntax_kind(kind: &str) -> RSyntaxKind {
        match kind {
            "function_definition" => RSyntaxKind::R_FUNCTION_DEFINITION,
            _ => panic!("Not yet implemented"),
        }
    }

    fn walk(&mut self) {
        let root = self.ast.root_node();
        let mut cursor = root.walk();

        self.parse.start(RSyntaxKind::R_ROOT);

        // TODO!: We will probably want to "remember" the `previous_node`
        // for the purpose of computing `Trivia`. We may need a `stack` of
        // previous nodes that we push/pop as we go `Up` and `Down` the tree,
        // so that we only compute a `Trivia` diff between nodes on the same
        // branch "level" of the tree

        while let (Some(node), direction) = Self::dfs_next(&mut cursor) {
            // TODO!: If `node` is a leaf, we want to push an `Event::Token`,
            // not open a new start()

            // TODO!: When we go `Up`, we may have had to go up >1 branches of
            // the tree, meaning we need to `finish()` >1 events. The
            // `Direction::Up` enum variant should probably take an integer value
            // representing the number of times we went `Up` before finding
            // the next node (that way we can finish that many events).

            match direction {
                // We got to `node` by going further down into the tree,
                // emit a new `Start` event
                Direction::Down => {
                    let kind = Self::as_syntax_kind(node.kind());
                    self.parse.start(kind);
                }

                // We got to `node` by walking up the tree and then moving
                // to the next relevant sibling. Emit `Finish` event for the
                // previous node and then `Start` the next one.
                Direction::Up => {
                    self.parse.finish();
                    let kind = Self::as_syntax_kind(node.kind());
                    self.parse.start(kind);
                }
            }
        }

        self.parse.finish();
    }

    // TODO!: I think the depth first search algorithm is right here, but not tested yet
    fn dfs_next<'tree>(cursor: &mut TreeCursor<'tree>) -> (Option<Node<'tree>>, Direction) {
        loop {
            if cursor.goto_first_child() {
                return (Some(cursor.node()), Direction::Down);
            }

            if cursor.goto_next_sibling() {
                // TODO!: Should this be `Direction::Side`?
                return (Some(cursor.node()), Direction::Down);
            }

            loop {
                if !cursor.goto_parent() {
                    // Returned to root
                    return (None, Direction::Up);
                }

                if cursor.goto_next_sibling() {
                    return (Some(cursor.node()), Direction::Up);
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
