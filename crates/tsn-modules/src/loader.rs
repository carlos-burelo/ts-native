use std::path::{Path, PathBuf};

use super::registry::{MODULE_REGISTRY, is_known, spec_for};
use super::spec::ModuleKind;

/// Path-resolution authority. Does NOT handle ExportMap or BindResult
/// — avoids circular deps with the checker. Only resolves paths and specs.
pub struct ModuleLoader {
    stdlib_root: PathBuf,
}

impl ModuleLoader {
    pub fn new(stdlib_root: PathBuf) -> Self {
        Self { stdlib_root }
    }

    /// Build from environment: TSN_STDLIB env var, or fallback to repo-relative path.
    pub fn from_env() -> Self {
        let candidates = stdlib_candidates();
        for c in &candidates {
            if c.is_dir() {
                return Self::new(c.clone());
            }
        }
        // Fallback: use the first candidate even if it doesn't exist
        Self::new(candidates.into_iter().next().unwrap_or_default())
    }

    pub fn stdlib_root(&self) -> &Path {
        &self.stdlib_root
    }

    pub fn is_known(&self, specifier: &str) -> bool {
        is_known(specifier)
    }

    pub fn spec_for(&self, specifier: &str) -> Option<&'static super::spec::ModuleSpec> {
        spec_for(specifier)
    }

    pub fn is_builtin(&self, specifier: &str) -> bool {
        spec_for(specifier).is_some_and(|s| s.kind == ModuleKind::Builtin)
    }

    pub fn is_stdlib(&self, specifier: &str) -> bool {
        spec_for(specifier).is_some_and(|s| s.kind == ModuleKind::Stdlib)
    }

    /// Returns the absolute path to the TSN source file for a known module.
    pub fn tsn_source_path(&self, specifier: &str) -> Option<PathBuf> {
        let spec = spec_for(specifier)?;
        let path = self.stdlib_root.join(spec.tsn_source);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn builtins(&self) -> impl Iterator<Item = &'static super::spec::ModuleSpec> {
        MODULE_REGISTRY.iter().filter(|m| m.kind == ModuleKind::Builtin)
    }

    pub fn stdlib_modules(&self) -> impl Iterator<Item = &'static super::spec::ModuleSpec> {
        MODULE_REGISTRY.iter().filter(|m| m.kind == ModuleKind::Stdlib)
    }

    /// Resolve a relative specifier (e.g. "./foo") relative to a base directory.
    pub fn resolve_relative(&self, base_dir: &Path, specifier: &str) -> Option<String> {
        let base = if base_dir.is_file() {
            base_dir.parent().unwrap_or(base_dir)
        } else {
            base_dir
        };
        let resolved = base.join(specifier);
        let normalized = resolved
            .components()
            .fold(PathBuf::new(), |mut acc, c| {
                use std::path::Component::*;
                match c {
                    ParentDir => { acc.pop(); }
                    CurDir => {}
                    c => acc.push(c),
                }
                acc
            });
        Some(normalized.to_string_lossy().to_string())
    }
}

fn stdlib_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();

    // 1. TSN_STDLIB env var
    if let Ok(v) = std::env::var("TSN_STDLIB") {
        out.push(PathBuf::from(v));
    }

    // 2. TSN_HOME/stdlib
    if let Ok(home) = std::env::var("TSN_HOME") {
        out.push(PathBuf::from(home).join("stdlib"));
    }

    // 3. Repo-relative tsn-stdlib
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe.parent().and_then(|p| p.parent()) {
            out.push(root.join("tsn-stdlib"));
        }
    }

    // 4. CWD-relative
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("tsn-stdlib"));
    }

    out
}
