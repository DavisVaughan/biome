use crate::kind_src::KindsSrc;

pub const R_KINDS_SRC: KindsSrc = KindsSrc {
    punct: &[
        (";", "SEMICOLON"),
        (",", "COMMA"),
        ("{", "L_CURLY"),
        ("}", "R_CURLY"),
        ("[", "L_BRACK"),
        ("]", "R_BRACK"),
        ("(", "L_PAREN"),
        (")", "R_PAREN"),
        ("+", "PLUS"),
    ],
    keywords: &["function"],
    literals: &[
        "R_INTEGER_LITERAL",
        "R_DOUBLE_LITERAL",
        "R_STRING_LITERAL",
        "R_LOGICAL_LITERAL",
        "R_NULL_LITERAL",
    ],
    tokens: &["NEWLINE", "WHITESPACE", "IDENT", "COMMENT"],
    nodes: &[
        "R_ROOT",
        "R_IDENTIFIER",
        "R_BINARY_EXPRESSION",
        "R_FUNCTION_DEFINITION",
        "R_PARAMETERS",
        "R_PARAMETER_LIST",
        "R_PARAMETER",
        "R_EXPRESSION_LIST",
        "R_INTEGER_VALUE",
        "R_DOUBLE_VALUE",
        "R_STRING_VALUE",
        "R_LOGICAL_VALUE",
        "R_NULL_VALUE",
        // Bogus nodes
        "R_BOGUS",
        "R_BOGUS_VALUE",
        "R_BOGUS_EXPRESSION",
        "R_BOGUS_PARAMETER",
    ],
};
