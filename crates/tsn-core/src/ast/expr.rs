use super::decl::ClassDecl;
use super::operators::{AssignOp, BinaryOp, LogicalOp, UnaryOp, UpdateOp};
use super::pattern::{MatchPattern, Param};
use super::stmt::Stmt;
use super::types::TypeNode;
use crate::source::SourceRange;

#[derive(Clone, Debug)]
pub enum Expr {
    IntLiteral {
        value: i64,
        raw: String,
        range: SourceRange,
    },
    FloatLiteral {
        value: f64,
        raw: String,
        range: SourceRange,
    },
    BigIntLiteral {
        raw: String,
        range: SourceRange,
    },
    DecimalLiteral {
        raw: String,
        range: SourceRange,
    },
    StrLiteral {
        value: String,
        range: SourceRange,
    },
    CharLiteral {
        value: char,
        range: SourceRange,
    },
    BoolLiteral {
        value: bool,
        range: SourceRange,
    },
    NullLiteral {
        range: SourceRange,
    },
    RegexLiteral {
        pattern: String,
        flags: String,
        range: SourceRange,
    },
    Template {
        parts: Vec<TemplatePart>,
        range: SourceRange,
    },
    TaggedTemplate {
        tag: Box<Expr>,
        template: Box<Expr>,
        type_args: Vec<TypeNode>,
        range: SourceRange,
    },
    Identifier {
        name: String,
        range: SourceRange,
    },
    This {
        range: SourceRange,
    },
    Super {
        range: SourceRange,
    },
    Array {
        elements: Vec<ArrayEl>,
        range: SourceRange,
    },
    Object {
        properties: Vec<ObjectProp>,
        range: SourceRange,
    },
    Unary {
        op: UnaryOp,
        prefix: bool,
        operand: Box<Expr>,
        range: SourceRange,
    },
    Update {
        op: UpdateOp,
        prefix: bool,
        operand: Box<Expr>,
        range: SourceRange,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        range: SourceRange,
    },
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
        range: SourceRange,
    },
    Assign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
        range: SourceRange,
    },
    Conditional {
        test: Box<Expr>,
        consequent: Box<Expr>,
        alternate: Box<Expr>,
        range: SourceRange,
    },

    Member {
        object: Box<Expr>,
        property: Box<Expr>,
        computed: bool,
        optional: bool,
        range: SourceRange,
    },
    Call {
        callee: Box<Expr>,
        type_args: Vec<TypeNode>,
        args: Vec<Arg>,
        optional: bool,
        range: SourceRange,
    },
    New {
        callee: Box<Expr>,
        type_args: Vec<TypeNode>,
        args: Vec<Arg>,
        range: SourceRange,
    },
    Function {
        id: Option<String>,
        params: Vec<Param>,
        return_type: Option<TypeNode>,
        body: Box<Stmt>,
        is_async: bool,
        is_generator: bool,
        range: SourceRange,
    },
    Arrow {
        params: Vec<Param>,
        return_type: Option<TypeNode>,
        body: Box<ArrowBody>,
        is_async: bool,
        range: SourceRange,
    },
    Sequence {
        expressions: Vec<Expr>,
        range: SourceRange,
    },
    Paren {
        expression: Box<Expr>,
        range: SourceRange,
    },
    Await {
        argument: Box<Expr>,
        range: SourceRange,
    },
    Yield {
        argument: Option<Box<Expr>>,
        delegate: bool,
        range: SourceRange,
    },
    Spread {
        argument: Box<Expr>,
        range: SourceRange,
    },
    Pipeline {
        left: Box<Expr>,
        right: Box<Expr>,
        range: SourceRange,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        range: SourceRange,
    },
    NonNull {
        expression: Box<Expr>,
        range: SourceRange,
    },
    As {
        expression: Box<Expr>,
        type_ann: TypeNode,
        range: SourceRange,
    },
    Satisfies {
        expression: Box<Expr>,
        type_ann: TypeNode,
        range: SourceRange,
    },
    ClassExpr {
        declaration: Box<ClassDecl>,
        range: SourceRange,
    },
    Match {
        subject: Box<Expr>,
        cases: Vec<MatchCase>,
        range: SourceRange,
    },
}

impl Expr {
    pub fn range(&self) -> &SourceRange {
        match self {
            Expr::IntLiteral { range, .. } => range,
            Expr::FloatLiteral { range, .. } => range,
            Expr::BigIntLiteral { range, .. } => range,
            Expr::DecimalLiteral { range, .. } => range,
            Expr::StrLiteral { range, .. } => range,
            Expr::CharLiteral { range, .. } => range,
            Expr::BoolLiteral { range, .. } => range,
            Expr::NullLiteral { range } => range,
            Expr::RegexLiteral { range, .. } => range,
            Expr::Template { range, .. } => range,
            Expr::TaggedTemplate { range, .. } => range,
            Expr::Identifier { range, .. } => range,
            Expr::This { range } => range,
            Expr::Super { range } => range,
            Expr::Array { range, .. } => range,
            Expr::Object { range, .. } => range,
            Expr::Unary { range, .. } => range,
            Expr::Update { range, .. } => range,
            Expr::Binary { range, .. } => range,
            Expr::Logical { range, .. } => range,
            Expr::Assign { range, .. } => range,
            Expr::Conditional { range, .. } => range,
            Expr::Member { range, .. } => range,
            Expr::Call { range, .. } => range,
            Expr::New { range, .. } => range,
            Expr::Function { range, .. } => range,
            Expr::Arrow { range, .. } => range,
            Expr::Sequence { range, .. } => range,
            Expr::Paren { range, .. } => range,
            Expr::Await { range, .. } => range,
            Expr::Yield { range, .. } => range,
            Expr::Spread { range, .. } => range,
            Expr::Pipeline { range, .. } => range,
            Expr::Range { range, .. } => range,
            Expr::NonNull { range, .. } => range,
            Expr::As { range, .. } => range,
            Expr::Satisfies { range, .. } => range,
            Expr::ClassExpr { range, .. } => range,
            Expr::Match { range, .. } => range,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ArrowBody {
    Block(Stmt),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub enum TemplatePart {
    Literal(String),
    Interpolation(Expr),
}

#[derive(Clone, Debug)]
pub enum ArrayEl {
    Hole,
    Expr(Expr),
    Spread(Expr),
}

#[derive(Clone, Debug)]
pub enum ObjectProp {
    Property {
        key: PropKey,
        value: Expr,
        shorthand: bool,
        computed: bool,
        range: SourceRange,
    },
    Method {
        key: PropKey,
        params: Vec<Param>,
        body: Stmt,
        return_type: Option<TypeNode>,
        is_async: bool,
        is_generator: bool,
        range: SourceRange,
    },
    Getter {
        key: PropKey,
        body: Stmt,
        return_type: Option<TypeNode>,
        range: SourceRange,
    },
    Setter {
        key: PropKey,
        param: Param,
        body: Stmt,
        range: SourceRange,
    },
    Spread {
        argument: Expr,
        range: SourceRange,
    },
}

impl ObjectProp {
    pub fn range(&self) -> &SourceRange {
        match self {
            ObjectProp::Property { range, .. } => range,
            ObjectProp::Method { range, .. } => range,
            ObjectProp::Getter { range, .. } => range,
            ObjectProp::Setter { range, .. } => range,
            ObjectProp::Spread { range, .. } => range,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PropKey {
    Identifier(String),
    Str(String),
    Int(i64),
    Computed(Expr),
}

#[derive(Clone, Debug)]
pub enum Arg {
    Positional(Expr),
    Spread(Expr),
    Named { label: String, value: Expr },
}

#[derive(Clone, Debug)]
pub struct MatchCase {
    pub pattern: MatchPattern,
    pub guard: Option<Expr>,
    pub body: MatchBody,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub enum MatchBody {
    Block(Stmt),
    Expr(Expr),
}
