use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::RwLock as PLRwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

static SLIDING_WINDOW_SECS: u64 = 60;
static QPS_THRESHOLD: u64 = 50_000;
static P99_THRESHOLD_MS: u64 = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotKeySummary {
    pub key: String,
    pub qps: u64,
    pub p99_latency_ms: u64,
    pub detected_at_ms: u64,
    pub action: MitigationAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MitigationAction {
    None,
    LocalReplica { replicas: u32 },
    CircuitBreaker { until_ms: u64 },
    Throttle { qps_limit: u64 },
}

struct HotKeyStats {
    qps: AtomicU64,
    p99_ms: AtomicU64,
    events: VecDeque<(Instant, u64)>,
}

static GLOBAL_MAP: OnceCell<DashMap<String, HotKeyStats>> = OnceCell::new();

fn global_map() -> &'static DashMap<String, HotKeyStats> {
    GLOBAL_MAP.get_or_init(|| DashMap::new())
}

pub fn record(key: &str, latency_ms: u64) {
    let map = global_map();
    let now = Instant::now();
    let mut entry = map.entry(key.to_string()).or_insert_with(|| HotKeyStats {
        qps: AtomicU64::new(0),
        p99_ms: AtomicU64::new(0),
        events: VecDeque::new(),
    });

    entry.events.push_back((now, latency_ms));
    prune(entry, now);

    let count = entry.events.len() as u64;
    let window_secs = SLIDING_WINDOW_SECS.max(1);
    entry.qps.store(count / window_secs, Ordering::Relaxed);

    let current_p99 = entry.p99_ms.load(Ordering::Relaxed);
    entry.p99_ms.store(latency_ms.max(current_p99), Ordering::Relaxed);
}

pub fn analyze(key: &str) -> Option<HotKeySummary> {
    global_map().get(key).map(|e| {
        let qps = e.qps.load(Ordering::Relaxed);
        let p99 = e.p99_ms.load(Ordering::Relaxed);
        let action = if qps > QPS_THRESHOLD || p99 > P99_THRESHOLD_MS {
            MitigationAction::Throttle { qps_limit: QPS_THRESHOLD }
        } else {
            MitigationAction::None
        };
        HotKeySummary {
            key: key.to_string(),
            qps,
            p99_latency_ms: p99,
            detected_at_ms: now_ms(),
            action,
        }
    })
}

fn prune(stats: &mut HotKeyStats, now: Instant) {
    let cutoff = now - Duration::from_secs(SLIDING_WINDOW_SECS);
    while let Some(front) = stats.events.front() {
        if front.0 < cutoff {
            stats.events.pop_front();
        } else {
            break;
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

use std::sync::OnceCell;
