use std::sync::Arc;

use tsn_core::OpCode;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    BigInt(i128),
    Decimal(rust_decimal::Decimal),
    Symbol(crate::value::SymbolKind),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum PoolEntry {
    Literal(Literal),
    Function(FunctionProto),
}

impl PoolEntry {
    pub fn as_str(&self) -> Option<&str> {
        if let PoolEntry::Literal(Literal::Str(s)) = self {
            Some(s.as_ref())
        } else {
            None
        }
    }
}

pub const INVALID_CACHE_SHAPE: u32 = 0;

#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    pub id: u32,
    pub slot: u16,
    pub is_class: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FunctionProto {
    pub name: Option<String>,
    pub arity: usize,
    pub has_rest: bool,
    pub is_async: bool,
    pub is_generator: bool,
    pub has_this: bool,
    pub upvalue_count: usize,
    pub cache_count: usize,
    pub chunk: Chunk,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Chunk {
    pub code: Vec<u16>,

    pub lines: Vec<u32>,

    pub constants: Vec<PoolEntry>,

    pub source_file: String,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk::default()
    }

    pub fn write(&mut self, word: u16, line: u32) {
        self.code.push(word);
        self.lines.push(line);
    }

    pub fn emit(&mut self, op: OpCode, line: u32) {
        self.write(op as u16, line);
    }

    pub fn emit1(&mut self, op: OpCode, operand: u16, line: u32) {
        self.write(op as u16, line);
        self.write(operand, line);
    }

    pub fn emit2(&mut self, op: OpCode, op1: u16, op2: u16, line: u32) {
        self.write(op as u16, line);
        self.write(op1, line);
        self.write(op2, line);
    }

    pub fn emit_jump(&mut self, op: OpCode, line: u32) -> usize {
        self.write(op as u16, line);
        let patch_pos = self.code.len();
        self.write(0xFFFF, line);
        patch_pos
    }

    pub fn patch_jump(&mut self, patch_pos: usize) {
        let offset = self.code.len() - patch_pos - 1;
        if offset > u16::MAX as usize {
            panic!("Jump offset too large: {}", offset);
        }
        self.code[patch_pos] = offset as u16;
    }

    pub fn emit_loop(&mut self, loop_start: usize, line: u32) {
        let offset = self.code.len() - loop_start + 2;
        if offset > u16::MAX as usize {
            panic!("Loop offset too large: {}", offset);
        }
        self.emit1(OpCode::OpLoop, offset as u16, line);
    }

    pub fn add_constant(&mut self, entry: PoolEntry) -> u16 {
        if let PoolEntry::Literal(Literal::Str(ref s)) = entry {
            for (i, e) in self.constants.iter().enumerate() {
                if let PoolEntry::Literal(Literal::Str(ref existing)) = e {
                    if existing == s {
                        return i as u16;
                    }
                }
            }
        } else if let PoolEntry::Literal(Literal::Symbol(ref s)) = entry {
            for (i, e) in self.constants.iter().enumerate() {
                if let PoolEntry::Literal(Literal::Symbol(ref existing)) = e {
                    if existing == s {
                        return i as u16;
                    }
                }
            }
        }
        let idx = self.constants.len();
        assert!(idx < u16::MAX as usize, "constant pool overflow");
        self.constants.push(entry);
        idx as u16
    }

    pub fn add_str(&mut self, s: impl AsRef<str>) -> u16 {
        self.add_constant(PoolEntry::Literal(Literal::Str(Arc::from(s.as_ref()))))
    }

    pub fn add_symbol(&mut self, s: crate::value::SymbolKind) -> u16 {
        self.add_constant(PoolEntry::Literal(Literal::Symbol(s)))
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }
}
