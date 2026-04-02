pub mod revision;

use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use std::sync::Arc;

use crate::document::DocumentState;
use crate::index::ProjectIndex;
use crate::pipeline::run_pipeline;

pub use revision::{Cached, Revision};

pub struct Workspace {
    files: Arc<DashMap<String, DocumentState>>,
    pub index: std::sync::RwLock<ProjectIndex>,
    revision: std::sync::Mutex<Revision>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            files: Arc::new(DashMap::new()),
            index: std::sync::RwLock::new(ProjectIndex::new()),
            revision: std::sync::Mutex::new(Revision::new()),
        }
    }

    pub fn update_file(&self, uri: String, source: String) {
        let state = run_pipeline(source, uri.clone());
        if let Ok(mut idx) = self.index.write() {
            idx.update_file(&uri, &state);
        }
        self.files.insert(uri, state);
        if let Ok(mut rev) = self.revision.lock() {
            rev.bump();
        }
    }

    pub fn remove_file(&self, uri: &str) {
        self.files.remove(uri);
        if let Ok(mut idx) = self.index.write() {
            idx.remove_file(uri);
        }
    }

    pub fn get(&self, uri: &str) -> Option<Ref<'_, String, DocumentState>> {
        self.files.get(uri)
    }

    pub fn iter(&self) -> dashmap::iter::Iter<'_, String, DocumentState> {
        self.files.iter()
    }

    pub fn revision(&self) -> u32 {
        self.revision.lock().map(|r| r.current()).unwrap_or(0)
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}
