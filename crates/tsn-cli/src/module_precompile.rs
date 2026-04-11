use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

use tsn_compiler::FunctionProto;
use tsn_core::ast::Program;

pub type PrecompiledMap = HashMap<String, Arc<FunctionProto>>;

pub fn precompile_direct_imports(program: &Program, base_file: &str) -> PrecompiledMap {
    let mut precompiled = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    let mut queue = VecDeque::new();

    let imports = crate::import_collector::collect_imports(program);

    let main_dir = Path::new(base_file)
        .parent()
        .unwrap_or_else(|| Path::new("."));

    for import_spec in imports {
        if let Some(resolved_path) = resolve_import_specifier(&import_spec, main_dir) {
            queue.push_back(resolved_path);
        }
    }

    while let Some(module_path) = queue.pop_front() {
        if visited.contains(&module_path) {
            continue;
        }
        visited.insert(module_path.clone());

        let Ok(source) = std::fs::read_to_string(&module_path) else {
            continue;
        };

        let tokens = tsn_lexer::scan(&source, &module_path);
        let Ok(mod_program) = tsn_parser::parse(tokens, &module_path) else {
            continue;
        };

        let module_dir = Path::new(&module_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));

        for child_import in crate::import_collector::collect_imports(&mod_program) {
            if let Some(child_path) = resolve_import_specifier(&child_import, module_dir) {
                if !visited.contains(&child_path) {
                    queue.push_back(child_path);
                }
            }
        }

        let mod_check = tsn_checker::Checker::check(&mod_program);
        let Ok(module_proto) = tsn_compiler::compile_with_check_result(
            &mod_program,
            &mod_check.type_annotations,
            &mod_check.extension_calls,
            &mod_check.extension_members,
            &mod_check.extension_set_members,
        ) else {
            continue;
        };

        precompiled.insert(module_path, Arc::new(module_proto));
    }

    precompiled
}

pub fn resolve_import_specifier(specifier: &str, module_dir: &Path) -> Option<String> {
    if tsn_checker::module_resolver::is_known_stdlib(specifier) {
        return None;
    }

    let joined = module_dir.join(specifier);
    let candidates = if joined.extension().is_some() {
        vec![joined]
    } else {
        vec![joined.with_extension("tsn"), joined]
    };

    for candidate in candidates {
        if candidate.exists() {
            if let Ok(canonical) = std::fs::canonicalize(&candidate) {
                return Some(canonical.to_string_lossy().into_owned());
            }
            return Some(candidate.to_string_lossy().into_owned());
        }
    }

    None
}
