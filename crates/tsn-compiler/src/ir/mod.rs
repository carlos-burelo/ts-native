pub mod lower;
pub mod passes;

use rustc_hash::FxHashMap;
use tsn_core::ast::Program;
use tsn_core::OpCode;
use tsn_core::TypeAnnotations;

#[derive(Clone, Debug)]
pub struct BytecodeIrModule {
    pub entry: IrFunction,
}

impl BytecodeIrModule {
    pub fn optimize(self) -> Self {
        Self {
            entry: passes::optimize_function(self.entry),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FullIrProgram {
    /// Full-fidelity source IR: captures the entire language surface.
    pub ast: Program,
    /// Checker-derived semantic side table.
    pub type_annotations: TypeAnnotations,
    /// Extension dispatch metadata from checker.
    pub extension_calls: FxHashMap<u32, String>,
    pub extension_members: FxHashMap<u32, String>,
    pub extension_set_members: FxHashMap<u32, String>,
    /// Lower executable IR used for optimizer/JIT passes.
    pub bytecode: BytecodeIrModule,
}

#[derive(Clone, Debug)]
pub struct IrFunction {
    pub name: Option<String>,
    pub arity: usize,
    pub is_async: bool,
    pub is_generator: bool,
    pub upvalue_count: usize,
    pub cache_count: usize,
    pub source_file: String,
    pub constants_len: usize,
    pub blocks: Vec<IrBlock>,
    pub children: Vec<IrFunction>,
}

#[derive(Clone, Debug)]
pub struct IrBlock {
    pub id: usize,
    pub start_ip: usize,
    pub end_ip: usize,
    pub instrs: Vec<IrInstr>,
    pub terminator: IrTerminator,
    pub successors: Vec<usize>,
    pub predecessors: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct IrInstr {
    pub ip: usize,
    pub line: u32,
    pub opcode: OpCode,
    pub operand: IrOperand,
}

#[derive(Clone, Debug)]
pub enum IrOperand {
    None,
    One(u16),
    Two(u16, u16),
    Closure {
        function_const: u16,
        upvalues: Vec<IrUpvalueDesc>,
    },
}

#[derive(Clone, Debug)]
pub struct IrUpvalueDesc {
    pub is_local: bool,
    pub index: u16,
}

#[derive(Clone, Debug)]
pub enum IrTerminator {
    Return,
    Throw,
    Jump {
        target_ip: usize,
    },
    Branch {
        opcode: OpCode,
        then_ip: usize,
        else_ip: usize,
    },
    Loop {
        target_ip: usize,
    },
    Fallthrough {
        next_ip: Option<usize>,
    },
}
