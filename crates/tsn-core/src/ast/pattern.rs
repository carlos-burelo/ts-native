use super::expr::Expr;
use super::operators::Modifiers;
use super::types::TypeNode;
use crate::source::SourceRange;

#[derive(Clone, Debug)]
pub enum Pattern {
    Identifier {
        name: String,
        type_ann: Option<TypeNode>,
        range: SourceRange,
    },
    Array {
        elements: Vec<Option<ArrayPatternEl>>,
        rest: Option<Box<Pattern>>,
        range: SourceRange,
    },
    Object {
        properties: Vec<ObjPatternProp>,
        rest: Option<Box<Pattern>>,
        range: SourceRange,
    },

    Assignment {
        left: Box<Pattern>,
        right: Box<Expr>,
        range: SourceRange,
    },
    Rest {
        argument: Box<Pattern>,
        range: SourceRange,
    },
}

impl Pattern {
    pub fn range(&self) -> &SourceRange {
        match self {
            Pattern::Identifier { range, .. } => range,
            Pattern::Array { range, .. } => range,
            Pattern::Object { range, .. } => range,
            Pattern::Assignment { range, .. } => range,
            Pattern::Rest { range, .. } => range,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArrayPatternEl {
    pub pattern: Pattern,
}

#[derive(Clone, Debug)]
pub struct ObjPatternProp {
    pub key: String,
    pub value: Pattern,
    pub shorthand: bool,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub pattern: Pattern,
    pub type_ann: Option<TypeNode>,
    pub default: Option<Box<Expr>>,
    pub is_rest: bool,
    pub is_optional: bool,
    pub modifiers: Modifiers,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub enum MatchPattern {
    Wildcard,
    Literal(Expr),
    Identifier(String),
    Record {
        fields: Vec<(String, Option<MatchPattern>)>,
        rest: bool,
    },
    Sequence(Vec<MatchPattern>),
    Type {
        type_name: String,
        binding: Option<String>,
    },
}
