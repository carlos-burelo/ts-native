use std::fmt;

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub name: String,
    pub text: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}

impl SourceLocation {
    pub fn new(line: u32, column: u32, offset: u32) -> Self {
        SourceLocation {
            line,
            column,
            offset,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct SourceRange {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

impl SourceRange {
    pub fn new(start: SourceLocation, end: SourceLocation) -> Self {
        SourceRange { start, end }
    }

    pub fn zero(loc: SourceLocation) -> Self {
        SourceRange {
            start: loc,
            end: loc,
        }
    }

    pub fn to(&self, other: SourceRange) -> Self {
        SourceRange {
            start: self.start,
            end: other.end,
        }
    }
}
