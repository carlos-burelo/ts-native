/// Compile cache logic: path generation, load, store, and validation
use std::path::Path;
use tsn_types::ModuleGraphArtifact;

use super::hash::fnv1a64;
use crate::error::CliError;

type PipelineResult<T> = Result<T, CliError>;

const CACHE_HEADER_LEN: usize = 4 + 8 + 8;

pub fn compile_cache_path(source_path: &str) -> std::path::PathBuf {
    let abs = std::fs::canonicalize(source_path)
        .unwrap_or_else(|_| std::path::PathBuf::from(source_path));
    let key = fnv1a64(abs.to_string_lossy().as_bytes());
    tsn_core::paths::tsn_cache_dir()
        .join("compile")
        .join(format!("{key:016x}.bin"))
}

pub fn load_cached_graph(
    cache_path: &Path,
    binary_fingerprint: u64,
    cache_version: u32,
) -> PipelineResult<Option<ModuleGraphArtifact>> {
    let bytes = match std::fs::read(cache_path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(CliError::fatal(format!(
                "{}{}error[cache]{}: cannot read '{}': {}",
                super::colors::BOLD,
                super::colors::C_ERRORS,
                super::colors::R,
                cache_path.display(),
                e
            )));
        }
    };

    if bytes.len() < CACHE_HEADER_LEN {
        return Ok(None);
    }

    let version = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let header_graph_hash = u64::from_le_bytes([
        bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11],
    ]);
    if version != cache_version {
        return Ok(None);
    }

    let payload = &bytes[CACHE_HEADER_LEN..];
    let graph: ModuleGraphArtifact = match bincode::deserialize(payload) {
        Ok(g) => g,
        Err(_) => return Ok(None),
    };

    if graph.format_version != cache_version {
        return Ok(None);
    }
    if graph.graph_hash != header_graph_hash {
        return Ok(None);
    }
    if graph.entry_proto().is_none() {
        return Ok(None);
    }
    if !is_graph_cache_valid(&graph, binary_fingerprint) {
        return Ok(None);
    }

    Ok(Some(graph))
}

pub fn store_cached_graph(cache_path: &Path, graph: &ModuleGraphArtifact) -> PipelineResult<()> {
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            CliError::fatal(format!(
                "{}{}error[cache]{}: cannot create '{}': {}",
                super::colors::BOLD,
                super::colors::C_ERRORS,
                super::colors::R,
                parent.display(),
                e
            ))
        })?;
    }

    let payload = bincode::serialize(graph).map_err(|e| {
        CliError::fatal(format!(
            "{}{}error[cache]{}: serialize failed: {}",
            super::colors::BOLD,
            super::colors::C_ERRORS,
            super::colors::R,
            e
        ))
    })?;

    let mut bytes = Vec::with_capacity(CACHE_HEADER_LEN + payload.len());
    bytes.extend_from_slice(&graph.format_version.to_le_bytes());
    bytes.extend_from_slice(&graph.graph_hash.to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    bytes.extend_from_slice(&payload);

    std::fs::write(cache_path, bytes).map_err(|e| {
        CliError::fatal(format!(
            "{}{}error[cache]{}: cannot write '{}': {}",
            super::colors::BOLD,
            super::colors::C_ERRORS,
            super::colors::R,
            cache_path.display(),
            e
        ))
    })
}

pub fn compile_output_from_graph(
    graph_artifact: ModuleGraphArtifact,
) -> PipelineResult<super::compile::CompileOutput> {
    let entry_proto = graph_artifact.entry_proto().cloned().ok_or_else(|| {
        CliError::fatal("cache error: graph artifact is missing entry proto".to_owned())
    })?;
    let precompiled = super::compile::precompiled_from_graph(&graph_artifact);
    Ok(super::compile::CompileOutput {
        entry_proto,
        precompiled,
        graph_artifact,
    })
}

fn is_graph_cache_valid(graph_artifact: &ModuleGraphArtifact, binary_fingerprint: u64) -> bool {
    let expected_graph_hash =
        super::graph_hash_from_sources(&graph_artifact.source_hashes, binary_fingerprint);
    if graph_artifact.graph_hash != expected_graph_hash {
        return false;
    }

    for (path, expected_hash) in &graph_artifact.source_hashes {
        let Ok(source) = std::fs::read_to_string(path) else {
            return false;
        };
        if fnv1a64(source.as_bytes()) != *expected_hash {
            return false;
        }
    }

    true
}
