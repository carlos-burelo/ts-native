pub mod chunk;
pub mod emit;
pub mod ir;
pub mod lowering;
pub mod scope;

pub use chunk::{Chunk, FunctionProto, Literal, PoolEntry};
pub use emit::Compiler;

use rustc_hash::FxHashMap;
use tsn_core::ast::Program;
use tsn_core::TypeAnnotations;

pub fn compile(program: &Program) -> Result<FunctionProto, String> {
    let mut compiler = Compiler::new("<anonymous>".to_owned(), 0, false, false);
    compiler.compile_program(program)?;
    let (mut proto, _) = compiler.finish();
    proto.chunk.source_file = canonical_path(&program.filename);
    Ok(proto)
}

pub fn compile_to_ir(program: &Program) -> Result<ir::BytecodeIrModule, String> {
    let proto = compile(program)?;
    ir::lower::lower_module(&proto)
}

pub fn compile_to_optimized_ir(program: &Program) -> Result<ir::BytecodeIrModule, String> {
    Ok(compile_to_ir(program)?.optimize())
}

pub fn compile_with_annotations(
    program: &Program,
    annotations: &TypeAnnotations,
) -> Result<FunctionProto, String> {
    let mut compiler = Compiler::new("<anonymous>".to_owned(), 0, false, false);
    compiler.type_annotations = Some(annotations as *const TypeAnnotations);
    compiler.compile_program(program)?;
    let (mut proto, _) = compiler.finish();
    proto.chunk.source_file = canonical_path(&program.filename);
    Ok(proto)
}

pub fn compile_to_ir_with_annotations(
    program: &Program,
    annotations: &TypeAnnotations,
) -> Result<ir::BytecodeIrModule, String> {
    let proto = compile_with_annotations(program, annotations)?;
    ir::lower::lower_module(&proto)
}

pub fn compile_to_optimized_ir_with_annotations(
    program: &Program,
    annotations: &TypeAnnotations,
) -> Result<ir::BytecodeIrModule, String> {
    Ok(compile_to_ir_with_annotations(program, annotations)?.optimize())
}

pub fn compile_with_check_result(
    program: &Program,
    annotations: &TypeAnnotations,
    extension_calls: &FxHashMap<u32, String>,
    extension_members: &FxHashMap<u32, String>,
    extension_set_members: &FxHashMap<u32, String>,
) -> Result<FunctionProto, String> {
    let mut compiler = Compiler::new("<anonymous>".to_owned(), 0, false, false);
    compiler.type_annotations = Some(annotations as *const TypeAnnotations);
    compiler.extension_calls = Some(extension_calls as *const FxHashMap<u32, String>);
    compiler.extension_members = Some(extension_members as *const FxHashMap<u32, String>);
    compiler.extension_set_members = Some(extension_set_members as *const FxHashMap<u32, String>);
    compiler.compile_program(program)?;
    let (mut proto, _) = compiler.finish();
    proto.chunk.source_file = canonical_path(&program.filename);
    Ok(proto)
}

pub fn compile_to_ir_with_check_result(
    program: &Program,
    annotations: &TypeAnnotations,
    extension_calls: &FxHashMap<u32, String>,
    extension_members: &FxHashMap<u32, String>,
    extension_set_members: &FxHashMap<u32, String>,
) -> Result<ir::BytecodeIrModule, String> {
    let proto = compile_with_check_result(
        program,
        annotations,
        extension_calls,
        extension_members,
        extension_set_members,
    )?;
    ir::lower::lower_module(&proto)
}

pub fn compile_to_optimized_ir_with_check_result(
    program: &Program,
    annotations: &TypeAnnotations,
    extension_calls: &FxHashMap<u32, String>,
    extension_members: &FxHashMap<u32, String>,
    extension_set_members: &FxHashMap<u32, String>,
) -> Result<ir::BytecodeIrModule, String> {
    Ok(compile_to_ir_with_check_result(
        program,
        annotations,
        extension_calls,
        extension_members,
        extension_set_members,
    )?
    .optimize())
}

pub fn compile_to_full_ir_with_check_result(
    program: &Program,
    annotations: &TypeAnnotations,
    extension_calls: &FxHashMap<u32, String>,
    extension_members: &FxHashMap<u32, String>,
    extension_set_members: &FxHashMap<u32, String>,
) -> Result<ir::FullIrProgram, String> {
    let bytecode = compile_to_optimized_ir_with_check_result(
        program,
        annotations,
        extension_calls,
        extension_members,
        extension_set_members,
    )?;

    Ok(ir::FullIrProgram {
        ast: program.clone(),
        type_annotations: annotations.clone(),
        extension_calls: extension_calls.clone(),
        extension_members: extension_members.clone(),
        extension_set_members: extension_set_members.clone(),
        bytecode,
    })
}

fn canonical_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_owned())
}
