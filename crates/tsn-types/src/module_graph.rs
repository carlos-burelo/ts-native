use std::collections::HashMap;

use crate::chunk::FunctionProto;

/// Serialized artifact for a full compiled module graph.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ModuleGraphArtifact {
    /// Artifact format version.
    pub format_version: u32,
    /// Canonical module path for the entrypoint module.
    pub entry_path: String,
    /// Stable hash derived from source hashes and binary fingerprint.
    pub graph_hash: u64,
    /// Source hash by canonical module path (includes entry and transitives).
    pub source_hashes: HashMap<String, u64>,
    /// Compiled bytecode proto by canonical module path.
    pub modules: HashMap<String, FunctionProto>,
}

impl ModuleGraphArtifact {
    /// Returns the compiled entrypoint proto if present.
    pub fn entry_proto(&self) -> Option<&FunctionProto> {
        self.modules.get(&self.entry_path)
    }
}
