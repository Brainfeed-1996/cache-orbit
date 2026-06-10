use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};

use axum::{
    Json, Router,
    extract::{Query, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    http::StatusCode,
};
use moka::sync::Cache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::io::Write;

pub mod hotkey;
pub mod invalidation;
pub mod metrics;
pub mod partition;
pub mod benchmark;

// -- Constants --
pub const PARTITION_COUNT: usize = 1024;
pub const DEFAULT_TTL_SECS: u64 = 300;

// -- Error types --
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("node unavailable: {0}")]
    NodeUnavailable(String),
    #[error("key not found: {0}")]
    KeyNotFound(String),
    #[error("capacity exceeded: {0}")]
    CapacityExceeded(String),
    #[error("invalid command: {0}")]
    InvalidCommand(String),
}

impl IntoResponse for CacheError {
    fn into_response(self) -> Response {
        let status = match self {
            CacheError::KeyNotFound(_) => StatusCode::NOT_FOUND,
            CacheError::CapacityExceeded(_) => StatusCode::INSUFFICIENT_STORAGE,
            CacheError::NodeUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::BAD_REQUEST,
        };
        let body = Json(serde_json::json!({"error": self.to_string()}));
        (status, body).into_response()
    }
}

// -- Data types --
#[derive(Debug, Clone, Serialize)]
pub struct CacheEntry {
    pub value: Vec<u8>,
    pub ttl_ms: u64,
    pub created_at_ms: u64,
    pub version: u64,
    pub tags: Vec<String>,
    pub source_node: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheNodeConfig {
    pub node_id: String,
    pub listen_addr: String,
    pub datacenter: String,
    pub rack: String,
    pub partition_ids: Vec<u64>,
    pub replicas: Vec<ReplicaNode>,
    pub max_capacity_mb: u64,
    pub consistency: ConsistencyLevel,
    pub hotkey_replicas: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaNode {
    pub node_id: String,
    pub addr: String,
    pub weight: u32,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsistencyLevel {
    Weak,
    BoundedStaleness { millis: u64 },
    Strong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topology {
    pub version: u64,
    pub created_at_ms: u64,
    pub nodes: HashMap<String, CacheNodeConfig>,
    pub partition_map: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl Topology {
    pub fn new() -> Self {
        Self {
            version: 1,
            created_at_ms: now_ms(),
            nodes: HashMap::new(),
            partition_map: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

// -- Engine state --
#[derive(Debug)]
pub struct EngineStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
    pub read_bytes: AtomicU64,
    pub write_bytes: AtomicU64,
    pub p50_latency_ms: AtomicU64,
    pub p99_latency_ms: AtomicU64,
    pub active_keys: AtomicUsize,
    pub total_keys: AtomicUsize,
}

impl EngineStats {
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            read_bytes: AtomicU64::new(0),
            write_bytes: AtomicU64::new(0),
            p50_latency_ms: AtomicU64::new(0),
            p99_latency_ms: AtomicU64::new(0),
            active_keys: AtomicUsize::new(0),
            total_keys: AtomicUsize::new(0),
        }
    }

    pub fn record_latency(&self, latency_ms: u64) {
        let current = self.p50_latency_ms.load(Ordering::Relaxed);
        let new = if current == 0 { latency_ms } else { (current * 7 + latency_ms) / 8 };
        self.p50_latency_ms.store(new, Ordering::Relaxed);

        let current_p99 = self.p99_latency_ms.load(Ordering::Relaxed);
        if latency_ms > current_p99 {
            self.p99_latency_ms.store(latency_ms, Ordering::Relaxed);
        }
    }

    pub fn hit_ratio(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        if total == 0.0 { 0.0 } else { (hits / total) * 100.0 }
    }
}

#[derive(Debug, Serialize)]
pub struct StatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub p50_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub active_keys: u64,
    pub total_keys: u64,
    pub hit_ratio: f64,
    pub node_id: String,
    pub timestamp_ms: u64,
}

// -- Cache Engine --
pub struct CacheEngine {
    primary: Arc<Cache<String, Arc<CacheEntry>>>,
    l1: Arc<Cache<String, Arc<CacheEntry>>>,
    topology: RwLock<Topology>,
    stats: Arc<EngineStats>,
    node_id: String,
    start_time: Instant,
}

impl CacheEngine {
    pub async fn new(config: CacheNodeConfig) -> anyhow::Result<Self> {
        info!("initializing cache engine: node_id={}", config.node_id);

        let stats = Arc::new(EngineStats::new());
        
        let primary = Arc::new(
            Cache::builder()
                .max_capacity(config.max_capacity_mb)
                .time_to_live(Duration::from_secs(DEFAULT_TTL_SECS))
                .eviction_listener(move |_k, _v, _cause| {
                    stats.evictions.fetch_add(1, Ordering::Relaxed);
                })
                .build(),
        );

        let l1 = Arc::new(
            Cache::builder()
                .max_capacity(1_000_000)
                .time_to_live(Duration::from_secs(30))
                .build(),
        );

        let engine = Self {
            primary,
            l1,
            topology: RwLock::new(Topology::new()),
            stats,
            node_id: config.node_id.clone(),
            start_time: Instant::now(),
        };

        // Initialize topology
        let mut topo = engine.topology.write();
        topo.nodes.insert(config.node_id.clone(), config);
        topo.partition_map = vec![engine.node_id.clone(); PARTITION_COUNT];
        drop(topo);

        metrics::register();
        Ok(engine)
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub async fn get(&self, key: &str) -> Result<Option<Arc<CacheEntry>>, CacheError> {
        let t0 = Instant::now();
        
        let entry = self.l1.get(key).or_else(|| self.primary.get(key));
        
        let elapsed = t0.elapsed().as_millis() as u64;
        self.stats.record_latency(elapsed);
        histogram!("cache.read.latency_ms", elapsed as f64, "node" => self.node_id.clone());

        match entry {
            Some(e) => {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                counter!("cache.hits", 1, "node" => self.node_id.clone());
                self.stats.read_bytes.fetch_add(e.value.len() as u64, Ordering::Relaxed);
                gauge!("cache.keys", self.primary.entry_count() as f64, "node" => self.node_id.clone());
                debug!("HIT key={} latency={}ms", key, elapsed);
                Ok(Some(e))
            }
            None => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                counter!("cache.misses", 1, "node" => self.node_id.clone());
                debug!("MISS key={}", key);
                Ok(None)
            }
        }
    }

    pub async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl_secs: u64,
        tags: Vec<String>,
    ) -> Result<(), CacheError> {
        let entry = std::sync::Arc::new(CacheEntry {
            value,
            ttl_ms: ttl_secs * 1000,
            created_at_ms: now_ms(),
            version: now_ms(),
            tags,
            source_node: self.node_id.clone(),
        });

        self.primary.insert(key.to_string(), std::sync::Arc::clone(&entry));
        self.l1.insert(key.to_string(), entry);
        
        let count = self.primary.entry_count();
        self.stats.active_keys.store(count, Ordering::Relaxed);
        counter!("cache.writes", 1, "node" => self.node_id.clone());
        
        debug!("SET key={}", key);
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let removed = self.primary.remove(key).is_some();
        self.l1.remove(key);
        if removed {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            warn!("deleted key={}", key);
        }
        Ok(())
    }

    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<u64, CacheError> {
        let mut count = 0u64;
        if let Ok(p) = glob_match::Glob::new(pattern) {
            let keys: Vec<_> = self.primary.keys().cloned().collect();
            for k in keys {
                if p.matches(&k) {
                    self.primary.remove(&k);
                    self.l1.remove(&k);
                    count += 1;
                }
            }
        }
        info!("invalidated {} keys matching pattern {}", count, pattern);
        Ok(count)
    }

    pub fn detect_hotkey(&self, key: &str) -> Option<hotkey::HotKeySummary> {
        hotkey::analyze(key).cloned()
    }

    pub fn stats(&self) -> StatsSnapshot {
        StatsSnapshot {
            hits: self.stats.hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            read_bytes: self.stats.read_bytes.load(Ordering::Relaxed),
            write_bytes: self.stats.write_bytes.load(Ordering::Relaxed),
            p50_latency_ms: self.stats.p50_latency_ms.load(Ordering::Relaxed),
            p99_latency_ms: self.stats.p99_latency_ms.load(Ordering::Relaxed),
            active_keys: self.stats.active_keys.load(Ordering::Relaxed) as u64,
            total_keys: self.stats.total_keys.load(Ordering::Relaxed) as u64,
            hit_ratio: self.stats.hit_ratio(),
            node_id: self.node_id.clone(),
            timestamp_ms: now_ms(),
        }
    }

    pub fn topology(&self) -> Topology {
        self.topology.read().clone()
    }
}

// -- HTTP Handlers --
#[derive(Debug, Deserialize)]
pub struct GetRequest {
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct SetRequest {
    pub key: String,
    pub value: String,
    pub ttl_secs: Option<u64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct InvalidatePatternRequest {
    pub pattern: String,
}

#[derive(Debug, Deserialize)]
pub struct BenchmarkRequest {
    pub scenario: String,
    pub request_count: Option<usize>,
    pub concurrency: Option<usize>,
    pub write_ratio: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct CacheResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub stats: StatsSnapshot,
    pub health: HealthStatus,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkResponse {
    pub bench_id: String,
    pub status: String,
}

// -- Server --
pub struct CacheServer {
    engine: CacheEngine,
}

impl CacheServer {
    pub fn new(engine: CacheEngine) -> Self {
        Self { engine }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/get", post(Self::get_handler))
            .route("/set", post(Self::set_handler))
            .route("/del", post(Self::delete_handler))
            .route("/stats", get(Self::stats_handler))
            .route("/health", get(Self::health_handler))
            .route("/topology", get(Self::topology_handler))
            .route("/invalidate/pattern", post(Self::invalidate_pattern_handler))
            .route("/metrics", get(Self::metrics_handler))
            .route("/benchmark", post(Self::benchmark_handler))
            .with_state(Arc::new(self))
    }

    async fn get_handler(
        State(state): State<Arc<Self>>,
        Json(req): Json<GetRequest>,
    ) -> Result<Json<CacheResponse<String>>, CacheError> {
        let t0 = Instant::now();
        match state.engine.get(&req.key).await {
            Ok(Some(entry)) => Ok(Json(CacheResponse {
                success: true,
                data: Some(String::from_utf8_lossy(&entry.value).to_string()),
                error: None,
                latency_ms: t0.elapsed().as_millis() as u64,
            })),
            Ok(None) => Ok(Json(CacheResponse {
                success: true,
                data: None,
                error: Some("key not found".to_string()),
                latency_ms: t0.elapsed().as_millis() as u64,
            })),
            Err(e) => Err(e),
        }
    }

    async fn set_handler(
        State(state): State<Arc<Self>>,
        Json(req): Json<SetRequest>,
    ) -> Result<Json<CacheResponse<()>>, CacheError> {
        let t0 = Instant::now();
        state.engine
            .set(
                &req.key,
                req.value.into_bytes(),
                req.ttl_secs.unwrap_or(DEFAULT_TTL_SECS),
                req.tags.unwrap_or_default(),
            )
            .await?;
        Ok(Json(CacheResponse {
            success: true,
            data: Some(()),
            error: None,
            latency_ms: t0.elapsed().as_millis() as u64,
        }))
    }

    async fn delete_handler(
        State(state): State<Arc<Self>>,
        Json(req): Json<DeleteRequest>,
    ) -> Result<Json<CacheResponse<()>>, CacheError> {
        let t0 = Instant::now();
        state.engine.delete(&req.key).await?;
        Ok(Json(CacheResponse {
            success: true,
            data: Some(()),
            error: None,
            latency_ms: t0.elapsed().as_millis() as u64,
        }))
    }

    async fn stats_handler(
        State(state): State<Arc<Self>>,
    ) -> Result<Json<StatsResponse>, CacheError> {
        Ok(Json(StatsResponse {
            stats: state.engine.stats(),
            health: HealthStatus {
                status: "healthy".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                uptime_seconds: state.engine.start_time.elapsed().as_secs(),
            },
        }))
    }

    async fn health_handler(
        State(state): State<Arc<Self>>,
    ) -> Result<Json<HealthStatus>, CacheError> {
        Ok(Json(HealthStatus {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: state.engine.start_time.elapsed().as_secs(),
        }))
    }

    async fn topology_handler(
        State(state): State<Arc<Self>>,
    ) -> Result<Json<Topology>, CacheError> {
        Ok(Json(state.engine.topology()))
    }

    async fn invalidate_pattern_handler(
        State(state): State<Arc<Self>>,
        Json(req): Json<InvalidatePatternRequest>,
    ) -> Result<Json<CacheResponse<u64>>, CacheError> {
        let t0 = Instant::now();
        let count = state.engine.invalidate_pattern(&req.pattern).await?;
        Ok(Json(CacheResponse {
            success: true,
            data: Some(count),
            error: None,
            latency_ms: t0.elapsed().as_millis() as u64,
        }))
    }

    async fn metrics_handler() -> Result<Response, CacheError> {
        let encoder = PrometheusBuilder::new().build_encoder()?;
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        let response = Response::builder()
            .header("Content-Type", encoder.content_type())
            .body(axum::body::Body::from(buffer))
            .unwrap();
        Ok(response)
    }

    async fn benchmark_handler(
        State(state): State<Arc<Self>>,
        Json(req): Json<BenchmarkRequest>,
    ) -> Result<Json<BenchmarkResponse>, CacheError> {
        let bench_id = format!("bench-{}", uuid::Uuid::new_v4());
        let request_count = req.request_count.unwrap_or(10_000);
        let concurrency = req.concurrency.unwrap_or(50);
        let write_ratio = req.write_ratio.unwrap_or(0.1);

        let runner = benchmark::BenchmarkRunner::new(
            &req.scenario,
            request_count,
            concurrency,
            write_ratio,
        );

        let engine_clone = state.engine.clone();
        tokio::spawn(async move {
            let result = runner.run(|_key| async move {
                let _ = engine_clone.get("bench_key").await;
            });
            info!("benchmark {} result: {:?}", bench_id, result);
        });

        Ok(Json(BenchmarkResponse {
            bench_id,
            status: "started".to_string(),
        }))
    }
}

#[inline]
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
