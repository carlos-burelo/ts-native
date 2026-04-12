use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;

use tsn_compiler::FunctionProto;
use tsn_core::ast::Program;
use tsn_types::ModuleGraphArtifact;

/// Internal result used while building a full module graph.
pub struct ModuleGraphBuild {
    pub entry_path: String,
    pub source_hashes: HashMap<String, u64>,
    pub modules: HashMap<String, FunctionProto>,
}

/// Build a transitive compiled module graph from an entry program.
pub fn build_module_graph(
    entry_program: &Program,
    entry_source: &str,
    entry_path: &str,
    entry_proto: &FunctionProto,
) -> Result<ModuleGraphBuild, String> {
    let canonical_entry = canonical_or_original(Path::new(entry_path));
    let mut source_hashes = HashMap::new();
    let mut modules = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    let mut queue = VecDeque::new();

    source_hashes.insert(canonical_entry.clone(), fnv1a64(entry_source.as_bytes()));
    modules.insert(canonical_entry.clone(), entry_proto.clone());
    visited.insert(canonical_entry.clone());

    let entry_dir = Path::new(&canonical_entry)
        .parent()
        .unwrap_or_else(|| Path::new("."));

    for import_spec in crate::import_collector::collect_imports(entry_program) {
        if let Some(resolved_path) = resolve_import_specifier(&import_spec, entry_dir) {
            if !visited.contains(&resolved_path) {
                queue.push_back(resolved_path);
            }
        }
    }

    while let Some(module_path) = queue.pop_front() {
        if visited.contains(&module_path) {
            continue;
        }
        visited.insert(module_path.clone());

        let source = std::fs::read_to_string(&module_path)
            .map_err(|e| format!("cannot read module '{}': {}", module_path, e))?;
        source_hashes.insert(module_path.clone(), fnv1a64(source.as_bytes()));

        let tokens = tsn_lexer::scan(&source, &module_path);
        let mod_program = tsn_parser::parse(tokens, &module_path)
            .map_err(|errs| format!("parse error in '{}': {}", module_path, errs[0].message))?;

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

        let module_proto = tsn_compiler::compile_with_check_result(
            &mod_program,
            &mod_check.type_annotations,
            &mod_check.extension_calls,
            &mod_check.extension_members,
            &mod_check.extension_set_members,
        )
        .map_err(|e| format!("compile error in '{}': {}", module_path, e))?;

        modules.insert(module_path, module_proto);
    }

    Ok(ModuleGraphBuild {
        entry_path: canonical_entry,
        source_hashes,
        modules,
    })
}

/// Build a serializable graph artifact from an entry program and graph build.
pub fn build_graph_artifact(
    format_version: u32,
    graph_hash: u64,
    graph: ModuleGraphBuild,
) -> ModuleGraphArtifact {
    ModuleGraphArtifact {
        format_version,
        entry_path: graph.entry_path,
        graph_hash,
        source_hashes: graph.source_hashes,
        modules: graph.modules,
    }
}

/// Resolve an import specifier against a module directory.
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

fn canonical_or_original(path: &Path) -> String {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical.to_string_lossy().into_owned();
    }
    path.to_string_lossy().into_owned()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
