//! Generated file, do not edit by hand, see `xtask/codegen`

#![allow(clippy::all)]
#![allow(bad_style, missing_docs, unreachable_pub)]
#[doc = r" The kind of syntax node, e.g. `IDENT`, `FUNCTION_KW`, or `FOR_STMT`."]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u16)]
pub enum RSyntaxKind {
    #[doc(hidden)]
    TOMBSTONE,
    #[doc = r" Marks the end of the file. May have trivia attached"]
    EOF,
    #[doc = r" Any Unicode BOM character that may be present at the start of"]
    #[doc = r" a file."]
    UNICODE_BOM,
    SEMICOLON,
    COMMA,
    L_CURLY,
    R_CURLY,
    L_BRACK,
    R_BRACK,
    FUNCTION_KW,
    R_INTEGER_VALUE,
    R_DOUBLE_VALUE,
    R_STRING_VALUE,
    R_LOGICAL_VALUE,
    R_NULL_VALUE,
    NEWLINE,
    WHITESPACE,
    IDENT,
    COMMENT,
    R_ROOT,
    R_IDENTIFIER,
    R_FUNCTION_DEFINITION,
    R_PARAMETERS,
    R_PARAMETER_LIST,
    R_PARAMETER,
    R_EXPRESSION_LIST,
    R_BOGUS,
    R_BOGUS_VALUE,
    R_BOGUS_PARAMETER,
    #[doc(hidden)]
    __LAST,
}
use self::RSyntaxKind::*;
impl RSyntaxKind {
    pub const fn is_punct(self) -> bool {
        match self {
            SEMICOLON | COMMA | L_CURLY | R_CURLY | L_BRACK | R_BRACK => true,
            _ => false,
        }
    }
    pub const fn is_literal(self) -> bool {
        match self {
            R_INTEGER_VALUE | R_DOUBLE_VALUE | R_STRING_VALUE | R_LOGICAL_VALUE | R_NULL_VALUE => {
                true
            }
            _ => false,
        }
    }
    pub const fn is_list(self) -> bool {
        match self {
            R_PARAMETER_LIST | R_EXPRESSION_LIST => true,
            _ => false,
        }
    }
    pub fn from_keyword(ident: &str) -> Option<RSyntaxKind> {
        let kw = match ident {
            "function" => FUNCTION_KW,
            _ => return None,
        };
        Some(kw)
    }
    pub const fn to_string(&self) -> Option<&'static str> {
        let tok = match self {
            SEMICOLON => ";",
            COMMA => ",",
            L_CURLY => "{",
            R_CURLY => "}",
            L_BRACK => "[",
            R_BRACK => "]",
            FUNCTION_KW => "function",
            R_STRING_VALUE => "string value",
            _ => return None,
        };
        Some(tok)
    }
}
#[doc = r" Utility macro for creating a SyntaxKind through simple macro syntax"]
#[macro_export]
macro_rules ! T { [;] => { $ crate :: RSyntaxKind :: SEMICOLON } ; [,] => { $ crate :: RSyntaxKind :: COMMA } ; ['{'] => { $ crate :: RSyntaxKind :: L_CURLY } ; ['}'] => { $ crate :: RSyntaxKind :: R_CURLY } ; ['['] => { $ crate :: RSyntaxKind :: L_BRACK } ; [']'] => { $ crate :: RSyntaxKind :: R_BRACK } ; [function] => { $ crate :: RSyntaxKind :: FUNCTION_KW } ; [ident] => { $ crate :: RSyntaxKind :: IDENT } ; [EOF] => { $ crate :: RSyntaxKind :: EOF } ; [UNICODE_BOM] => { $ crate :: RSyntaxKind :: UNICODE_BOM } ; [#] => { $ crate :: RSyntaxKind :: HASH } ; }
