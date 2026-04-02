use super::decl::Decl;
use super::expr::Expr;
use super::operators::VarKind;
use super::pattern::Pattern;
use super::types::TypeNode;
use crate::source::SourceRange;

#[derive(Clone, Debug)]
pub struct VarDeclarator {
    pub id: Pattern,
    pub type_ann: Option<TypeNode>,
    pub init: Option<Expr>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct VariableDecl {
    pub kind: VarKind,
    pub declarators: Vec<VarDeclarator>,
    pub doc: Option<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Block {
        stmts: Vec<Stmt>,
        range: SourceRange,
    },
    Empty {
        range: SourceRange,
    },
    Expr {
        expression: Box<Expr>,
        range: SourceRange,
    },
    Decl(Box<Decl>),

    If {
        test: Box<Expr>,
        consequent: Box<Stmt>,
        alternate: Option<Box<Stmt>>,
        range: SourceRange,
    },
    While {
        test: Box<Expr>,
        body: Box<Stmt>,
        range: SourceRange,
    },
    DoWhile {
        body: Box<Stmt>,
        test: Box<Expr>,
        range: SourceRange,
    },
    For {
        init: Option<Box<ForInit>>,
        test: Option<Box<Expr>>,
        update: Option<Box<Expr>>,
        body: Box<Stmt>,
        range: SourceRange,
    },
    ForIn {
        kind: VarKind,
        left: Pattern,
        right: Box<Expr>,
        body: Box<Stmt>,
        range: SourceRange,
    },
    ForOf {
        kind: VarKind,
        left: Pattern,
        right: Box<Expr>,
        body: Box<Stmt>,
        is_await: bool,
        range: SourceRange,
    },
    Switch {
        discriminant: Box<Expr>,
        cases: Vec<SwitchCase>,
        range: SourceRange,
    },

    Return {
        argument: Option<Box<Expr>>,
        range: SourceRange,
    },
    Break {
        label: Option<String>,
        range: SourceRange,
    },
    Continue {
        label: Option<String>,
        range: SourceRange,
    },
    Throw {
        argument: Box<Expr>,
        range: SourceRange,
    },
    Try {
        block: Box<Stmt>,
        catch: Option<Box<CatchClause>>,
        finally: Option<Box<Stmt>>,
        range: SourceRange,
    },
    Using {
        declarations: Vec<VarDeclarator>,
        is_await: bool,
        range: SourceRange,
    },
    Labeled {
        label: String,
        body: Box<Stmt>,
        range: SourceRange,
    },
    Debugger {
        range: SourceRange,
    },
}

impl Stmt {
    pub fn range(&self) -> &SourceRange {
        match self {
            Stmt::Block { range, .. } => range,
            Stmt::Empty { range } => range,
            Stmt::Expr { range, .. } => range,
            Stmt::Decl(d) => d.range(),
            Stmt::If { range, .. } => range,
            Stmt::While { range, .. } => range,
            Stmt::DoWhile { range, .. } => range,
            Stmt::For { range, .. } => range,
            Stmt::ForIn { range, .. } => range,
            Stmt::ForOf { range, .. } => range,
            Stmt::Switch { range, .. } => range,
            Stmt::Return { range, .. } => range,
            Stmt::Break { range, .. } => range,
            Stmt::Continue { range, .. } => range,
            Stmt::Throw { range, .. } => range,
            Stmt::Try { range, .. } => range,
            Stmt::Using { range, .. } => range,
            Stmt::Labeled { range, .. } => range,
            Stmt::Debugger { range } => range,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ForInit {
    Var {
        kind: VarKind,
        declarators: Vec<VarDeclarator>,
    },
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct SwitchCase {
    pub test: Option<Expr>,
    pub body: Vec<Stmt>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct CatchClause {
    pub param: Option<Pattern>,
    pub body: Box<Stmt>,
    pub range: SourceRange,
}
