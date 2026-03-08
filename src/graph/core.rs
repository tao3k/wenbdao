use crate::entity::{Entity, Relation};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub(super) fn read_lock<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn write_lock<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

/// Knowledge graph storage.
#[derive(Debug, Clone)]
pub struct KnowledgeGraph {
    /// Entities by ID
    pub(crate) entities: Arc<RwLock<HashMap<String, Entity>>>,
    /// Entities by name (for quick lookup)
    pub(crate) entities_by_name: Arc<RwLock<HashMap<String, String>>>,
    /// Relations by ID
    pub(crate) relations: Arc<RwLock<HashMap<String, Relation>>>,
    /// Outgoing relations (entity name -> set of relation IDs)
    pub(crate) outgoing_relations: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// Incoming relations (entity name -> set of relation IDs)
    pub(crate) incoming_relations: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// Entities by type
    pub(crate) entities_by_type: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeGraph {
    /// Create a new knowledge graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            entities_by_name: Arc::new(RwLock::new(HashMap::new())),
            relations: Arc::new(RwLock::new(HashMap::new())),
            outgoing_relations: Arc::new(RwLock::new(HashMap::new())),
            incoming_relations: Arc::new(RwLock::new(HashMap::new())),
            entities_by_type: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
