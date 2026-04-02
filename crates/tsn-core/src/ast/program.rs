use super::stmt::Stmt;
use crate::source::SourceRange;

#[derive(Clone, Debug)]
pub struct Program {
    pub filename: String,
    pub body: Vec<Stmt>,
    pub range: SourceRange,
}
