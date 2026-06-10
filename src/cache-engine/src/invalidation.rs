use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationEvent {
    pub key: String,
    pub pattern: Option<String>,
    pub scope: InvalidationScope,
    pub timestamp_ms: u64,
    pub source_node: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvalidationScope {
    Local,
    Cluster,
    Datacenter { dc: String },
}

pub struct InvalidationProcessor {
    pending: DashMap<u64, InvalidationEvent>,
    processed: AtomicU64,
}

impl InvalidationProcessor {
    pub fn new() -> Self {
        Self {
            pending: DashMap::new(),
            processed: AtomicU64::new(0),
        }
    }

    pub fn enqueue(&self, event: InvalidationEvent) {
        let id = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.pending.insert(id, event);
        self.processed.fetch_add(1, Ordering::Relaxed);
        debug!("invalidated event queued: id={}", id);
    }

    pub fn process_batch(&self, batch_size: usize) -> Vec<InvalidationEvent> {
        let mut events = Vec::with_capacity(batch_size);
        for entry in self.pending.iter().take(batch_size) {
            events.push(entry.value().clone());
        }
        events
    }

    pub fn stats(&self) -> InvalidationStats {
        InvalidationStats {
            pending: self.pending.len(),
            processed: self.processed.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InvalidationStats {
    pub pending: usize,
    pub processed: u64,
}

pub async fn process_invalidation_stream(
    mut rx: tokio::sync::mpsc::Receiver<InvalidationEvent>,
) {
    let processor = Arc::new(InvalidationProcessor::new());
    
    while let Some(event) = rx.recv().await {
        processor.enqueue(event);
    }
}
