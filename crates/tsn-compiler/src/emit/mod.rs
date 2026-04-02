use std::collections::HashMap;

mod decls;
mod exprs;
mod patterns;
mod stmts;

use crate::chunk::{Chunk, FunctionProto, PoolEntry};
use crate::scope::Scope;
use rustc_hash::FxHashMap;
use tsn_core::ast::{Param, Pattern, Stmt};
use tsn_core::{OpCode, TypeAnnotations};

pub struct Compiler {
    chunk: Chunk,
    scope: Scope,
    name: String,
    arity: usize,
    has_rest: bool,
    pub is_async: bool,
    pub is_generator: bool,
    has_this: bool,
    is_function: bool,
    loop_stack: Vec<crate::scope::LoopContext>,
    /// Active try blocks in the current function. Each entry holds the optional
    /// finally body (None = try without finally). Used to inline finally code
    /// before early returns and to emit the correct number of OpPopTry.
    pub(super) try_stack: Vec<Option<tsn_core::ast::Stmt>>,
    line: u32,
    last_instr_start: usize,
    parent: *mut Compiler,
    pub(super) type_annotations: Option<*const TypeAnnotations>,
    pub(super) extension_calls: Option<*const FxHashMap<u32, String>>,
    pub(super) extension_members: Option<*const FxHashMap<u32, String>>,
    pub(super) extension_set_members: Option<*const FxHashMap<u32, String>>,
    cache_count: usize,

    pub(super) class_vtable_layout: Option<HashMap<String, usize>>,

    pub(super) class_field_layout: Option<HashMap<String, usize>>,
    pub(super) class_field_count: usize,
    /// False when the parent class is external (not in class_slot_registry),
    /// meaning we don't know the inherited field count and cannot use
    /// GET/SET_FIXED_FIELD — must fall back to GET/SET_PROPERTY.
    pub(super) class_field_layout_valid: bool,

    pub(super) pending_field_inits: Vec<(String, usize, tsn_core::ast::Expr)>,

    pub(super) class_slot_registry: HashMap<String, (HashMap<String, usize>, usize)>,
}

impl Compiler {
    pub fn new(name: String, arity: usize, is_async: bool, is_generator: bool) -> Self {
        Compiler {
            chunk: Chunk::new(),
            scope: Scope::new(),
            name,
            arity,
            has_rest: false,
            is_async,
            is_generator,
            has_this: false,
            is_function: false,
            loop_stack: vec![],
            try_stack: vec![],
            line: 1,
            last_instr_start: 0,
            parent: std::ptr::null_mut(),
            type_annotations: None,
            extension_calls: None,
            extension_members: None,
            extension_set_members: None,
            cache_count: 0,
            class_vtable_layout: None,
            class_field_layout: None,
            class_field_count: 0,
            class_field_layout_valid: true,
            pending_field_inits: Vec::new(),
            class_slot_registry: Self::builtin_class_registry(),
        }
    }

    /// Pre-seeded field layouts for builtin classes defined in tsn-stdlib/builtins/.
    /// This lets subclasses of builtins use SET_FIXED_FIELD with correct slot numbers
    /// even though the builtins are compiled in a separate pass.
    fn builtin_class_registry() -> HashMap<String, (HashMap<String, usize>, usize)> {
        let mut reg = HashMap::new();
        // Error { message:0, name:1, stack:2 }
        let error_fields: HashMap<String, usize> = [
            ("message".to_owned(), 0usize),
            ("name".to_owned(), 1),
            ("stack".to_owned(), 2),
        ]
        .into_iter()
        .collect();
        // TypeError / RangeError extend Error with no own fields
        reg.insert(
            tsn_core::well_known::ERROR.to_owned(),
            (error_fields.clone(), 3),
        );
        reg.insert(
            tsn_core::well_known::TYPE_ERROR.to_owned(),
            (error_fields.clone(), 3),
        );
        reg.insert(
            tsn_core::well_known::RANGE_ERROR.to_owned(),
            (error_fields, 3),
        );
        reg
    }

    pub fn finish(self) -> (FunctionProto, Vec<crate::scope::Upvalue>) {
        let upvalues = self.scope.upvalues.clone();
        let upvalue_count = upvalues.len();
        (
            FunctionProto {
                name: Some(self.name),
                arity: self.arity,
                has_rest: self.has_rest,
                is_async: self.is_async,
                is_generator: self.is_generator,
                has_this: self.has_this,
                upvalue_count,
                cache_count: self.cache_count,
                chunk: self.chunk,
            },
            upvalues,
        )
    }

    pub(super) fn emit_closure(
        &mut self,
        proto: FunctionProto,
        upvalues: Vec<crate::scope::Upvalue>,
    ) {
        let line = self.line;
        let fn_idx = self.add_const(PoolEntry::Function(proto));
        self.last_instr_start = self.chunk.code.len();
        self.chunk.emit1(OpCode::OpClosure, fn_idx, line);
        for uv in upvalues {
            self.chunk.write(if uv.is_local { 1 } else { 0 }, line);
            self.chunk.write(uv.index as u16, line);
        }
    }

    pub(self) fn alloc_cache_slot(&mut self) -> u16 {
        let idx = self.cache_count;
        self.cache_count += 1;
        idx as u16
    }

    pub(super) fn lookup_numeric(&self, offset: u32) -> Option<tsn_core::NumericKind> {
        self.type_annotations
            .and_then(|ptr| unsafe { ptr.as_ref() })
            .and_then(|ann| ann.get_numeric(offset))
    }

    pub(super) fn lookup_extension_call(&self, call_offset: u32) -> Option<&str> {
        self.extension_calls
            .and_then(|ptr| unsafe { ptr.as_ref() })
            .and_then(|map| map.get(&call_offset))
            .map(|s| s.as_str())
    }

    pub(super) fn lookup_extension_member(&self, member_offset: u32) -> Option<&str> {
        self.extension_members
            .and_then(|ptr| unsafe { ptr.as_ref() })
            .and_then(|map| map.get(&member_offset))
            .map(|s| s.as_str())
    }

    pub(super) fn lookup_extension_set_member(&self, member_offset: u32) -> Option<&str> {
        self.extension_set_members
            .and_then(|ptr| unsafe { ptr.as_ref() })
            .and_then(|map| map.get(&member_offset))
            .map(|s| s.as_str())
    }

    pub(self) fn emit(&mut self, op: OpCode) {
        self.last_instr_start = self.chunk.code.len();
        self.chunk.emit(op, self.line);
    }

    pub(self) fn emit1(&mut self, op: OpCode, operand: u16) {
        self.last_instr_start = self.chunk.code.len();
        self.chunk.emit1(op, operand, self.line);
    }

    pub(crate) fn emit2(&mut self, op: OpCode, a: u16, b: u16) {
        self.last_instr_start = self.chunk.code.len();
        self.chunk.emit2(op, a, b, self.line);
    }

    pub(self) fn emit_jump(&mut self, op: OpCode) -> usize {
        self.last_instr_start = self.chunk.code.len();
        self.chunk.emit_jump(op, self.line)
    }

    pub(self) fn patch_jump(&mut self, pos: usize) {
        self.chunk.patch_jump(pos);
    }

    pub(self) fn emit_loop(&mut self, loop_start: usize) {
        self.last_instr_start = self.chunk.code.len();
        self.chunk.emit_loop(loop_start, self.line);
    }

    pub(self) fn add_const(&mut self, entry: PoolEntry) -> u16 {
        self.chunk.add_constant(entry)
    }

    pub(self) fn add_str(&mut self, s: impl AsRef<str>) -> u16 {
        self.chunk.add_str(s)
    }

    pub(self) fn chunk_len(&self) -> usize {
        self.chunk.len()
    }

    fn resolve_upvalue(&mut self, name: &str) -> Option<u8> {
        if self.parent.is_null() {
            return None;
        }
        let parent = unsafe { &mut *self.parent };
        if let Some(local_slot) = parent.scope.resolve_local(name) {
            parent.scope.locals[local_slot as usize].is_captured = true;
            return Some(self.scope.add_upvalue(true, local_slot as u8));
        }
        if let Some(upval_slot) = parent.resolve_upvalue(name) {
            return Some(self.scope.add_upvalue(false, upval_slot));
        }

        None
    }

    pub(self) fn resolve_variable(&mut self, name: &str) -> VarResolution {
        if let Some(slot) = self.scope.resolve_local(name) {
            return VarResolution::Local(slot);
        }

        if let Some(uv) = self.resolve_upvalue(name) {
            return VarResolution::Upvalue(uv);
        }

        VarResolution::Global(name.to_owned())
    }

    pub(self) fn emit_get_var(&mut self, name: &str) {
        match self.resolve_variable(name) {
            VarResolution::Local(slot) => self.emit1(OpCode::OpGetLocal, slot),
            VarResolution::Upvalue(uv) => self.emit1(OpCode::OpGetUpvalue, uv as u16),
            VarResolution::Global(n) => {
                let idx = self.add_str(&n);
                self.emit1(OpCode::OpGetGlobal, idx);
            }
        }
    }

    pub(self) fn emit_set_var(&mut self, name: &str) {
        match self.resolve_variable(name) {
            VarResolution::Local(slot) => self.emit1(OpCode::OpSetLocal, slot),
            VarResolution::Upvalue(uv) => self.emit1(OpCode::OpSetUpvalue, uv as u16),
            VarResolution::Global(n) => {
                let idx = self.add_str(&n);
                self.emit1(OpCode::OpSetGlobal, idx);
            }
        }
    }

    pub(self) fn emit_define_global(&mut self, name: &str) {
        let idx = self.add_str(name);
        self.emit1(OpCode::OpDefineGlobal, idx);
    }

    pub(super) fn emit_smart_pop(&mut self) {
        let start = self.last_instr_start;
        let n = self.chunk.code.len();

        if n == start + 2 {
            match OpCode::from_u16(self.chunk.code[start]) {
                Some(OpCode::OpSetGlobal) => {
                    self.chunk.code[start] = OpCode::OpDefineGlobal as u16;
                    return;
                }

                Some(OpCode::OpSetLocal) => {
                    self.chunk.code[start] = OpCode::OpSetLocalDrop as u16;
                    return;
                }
                Some(OpCode::OpPushConst) => {
                    self.chunk.code.truncate(start);
                    self.chunk.lines.truncate(start);
                    return;
                }
                _ => {}
            }
        }

        if n == start + 1 {
            match OpCode::from_u16(self.chunk.code[start]) {
                Some(OpCode::OpPushNull | OpCode::OpPushTrue | OpCode::OpPushFalse) => {
                    self.chunk.code.truncate(start);
                    self.chunk.lines.truncate(start);
                    return;
                }
                _ => {}
            }
        }

        self.emit(OpCode::OpPop);
    }
}

pub(self) fn compile_function_with_parent(
    name: &str,
    params: &[Param],
    body: &Stmt,
    is_async: bool,
    is_generator: bool,
    has_this: bool,
    parent: &mut Compiler,
) -> Result<(FunctionProto, Vec<crate::scope::Upvalue>), String> {
    compile_function_inner(
        name,
        params,
        body,
        is_async,
        is_generator,
        has_this,
        parent as *mut Compiler,
    )
}

fn compile_function_inner(
    name: &str,
    params: &[Param],
    body: &Stmt,
    is_async: bool,
    is_generator: bool,
    has_this: bool,
    parent: *mut Compiler,
) -> Result<(FunctionProto, Vec<crate::scope::Upvalue>), String> {
    let mut arity = params.len();
    if has_this {
        arity += 1;
    }
    let has_rest = params.iter().any(|p| p.is_rest);
    let mut c = Compiler::new(name.to_owned(), arity, is_async, is_generator);
    c.has_rest = has_rest;
    c.has_this = has_this;
    c.is_function = true;
    c.parent = parent;
    if !parent.is_null() {
        let par = unsafe { &*parent };
        c.type_annotations = par.type_annotations;
        c.extension_calls = par.extension_calls;
        c.extension_members = par.extension_members;
        c.extension_set_members = par.extension_set_members;

        c.class_field_layout = par.class_field_layout.clone();
        c.class_field_count = par.class_field_count;
        c.class_field_layout_valid = par.class_field_layout_valid;

        c.class_slot_registry = par.class_slot_registry.clone();
    }
    if has_this {
        c.scope.declare_local("this");
    }

    // Track non-identifier params that need destructuring: (slot, pattern)
    let mut destr_params: Vec<(u16, Pattern)> = Vec::new();

    for param in params {
        match &param.pattern {
            Pattern::Identifier { name, .. } => {
                c.scope.declare_local(name);
            }
            other => {
                let slot = c.scope.local_count() as u16;
                c.scope
                    .declare_local(&format!("__destr_param_{}", destr_params.len()));
                destr_params.push((slot, other.clone()));
            }
        }
    }

    // Emit preamble to destructure non-identifier params into named locals
    for (slot, pattern) in &destr_params {
        c.emit1(OpCode::OpGetLocal, *slot);
        c.declare_pattern_local(pattern)?;
    }

    if has_this && name == "constructor" && !parent.is_null() {
        let field_inits = unsafe { std::mem::take(&mut (*parent).pending_field_inits) };
        for (field_name, slot, expr) in &field_inits {
            c.emit1(OpCode::OpGetLocal, 0);
            c.compile_expr(expr)?;
            if c.class_field_layout_valid {
                c.emit1(OpCode::OpSetFixedField, *slot as u16);
            } else {
                let key_idx = c.add_str(field_name);
                let cs = c.alloc_cache_slot();
                c.emit(OpCode::OpSwap);
                c.emit2(OpCode::OpSetProperty, key_idx, cs);
            }
            c.emit(OpCode::OpPop);
        }
    }

    c.compile_stmt(body)?;

    c.emit(OpCode::OpPushNull);
    c.emit(OpCode::OpReturn);

    Ok(c.finish())
}

#[derive(Debug)]
pub(self) enum VarResolution {
    Local(u16),
    Upvalue(u8),
    Global(String),
}
