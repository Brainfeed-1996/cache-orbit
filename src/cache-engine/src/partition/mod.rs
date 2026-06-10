use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use blake3;

pub const PARTITION_COUNT: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub id: u64,
    pub primary_node: String,
    pub replica_nodes: Vec<String>,
    pub key_range: (u64, u64),
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionMap {
    pub version: u64,
    pub partitions: Vec<Partition>,
    pub primary_for_key: Arc<DashMap<String, u64>>,
}

impl PartitionMap {
    pub fn new(node_count: usize) -> Self {
        let chunk = PARTITION_COUNT / node_count.max(1);
        let partitions = (0..PARTITION_COUNT as u64)
            .map(|id| Partition {
                id,
                primary_node: (id % node_count as u64).to_string(),
                replica_nodes: vec![
                    ((id + 1) % node_count as u64).to_string(),
                    ((id + 2) % node_count as u64).to_string(),
                ],
                key_range: (id * 1000, (id + 1) * 1000),
                size_bytes: 0,
            })
            .collect();

        Self {
            version: 1,
            partitions,
            primary_for_key: Arc::new(DashMap::new()),
        }
    }

    pub fn primary_for(&self, key: &str) -> Option<&Partition> {
        let hash = blake3::hash(key.as_bytes());
        let val = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap());
        let partition_id = val % PARTITION_COUNT as u64;
        self.partitions.get(partition_id as usize)
    }

    pub fn update(&mut self, new_partitions: Vec<Partition>) {
        self.partitions = new_partitions;
        self.version += 1;
        debug!("partition map updated to version {}", self.version);
    }

    pub fn rebalance(&self, target_utilization: f64) -> RebalancePlan {
        let mut plan = RebalancePlan {
            moves: Vec::new(),
            estimated_impact_ms: 0,
        };

        for p in &self.partitions {
            let utilization = p.size_bytes as f64 / (10u64 * 1024 * 1024 * 1024) as f64;
            if utilization > target_utilization {
                plan.moves.push(MovePartition {
                    partition_id: p.id,
                    from: p.primary_node.clone(),
                    to: ((p.id + 3) % self.partitions.len() as u64).to_string(),
                });
            }
        }

        plan.estimated_impact_ms = plan.moves.len() as u64 * 50;
        plan
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RebalancePlan {
    pub moves: Vec<MovePartition>,
    pub estimated_impact_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MovePartition {
    pub partition_id: u64,
    pub from: String,
    pub to: String,
}

pub fn hash(key: &str) -> u64 {
    let hash = blake3::hash(key.as_bytes());
    u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap())
}
