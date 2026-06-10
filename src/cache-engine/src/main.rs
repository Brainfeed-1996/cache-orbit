use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    Json, Router, Server,
    extract::{Query, State, WebSocketUpgrade, ws::WebSocket, ws::Message as WsMessage},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use dashmap::DashMap;
use moka::sync::Cache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};
use metrics::{counter, histogram, gauge};
use std::time::SystemTime;

pub mod hotkey;
pub mod invalidation;
pub mod metrics;
pub mod partition;

// -- Constants --
pub const PARTITION_COUNT: usize = 1024;
pub const DEFAULT_TTL_SECS: u64 = 300;
pub const HOTKEY_THRESHOLD_QPS: u64 = 50_000;
pub const HOTKEY_THRESHOLD_P99_MS: u64 = 15;
pub const HOTKEY_SLIDING_WINDOW_SECS: u64 = 60;

// -- Error types --
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("node unavailable: {0}")]
    NodeUnavailable(String),
    #[error("replication timeout for key: {0}")]
    ReplicationTimeout(String),
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    #[error("capacity exceeded: current {current} max {max}")]
    CapacityExceeded { current: u64, max: u64 },
    #[error("key not found: {0}")]
    KeyNotFound(String),
    #[error("topology mismatch: expected version {expected} got {actual}")]
    TopologyMismatch { expected: u64, actual: u64 },
}

impl IntoResponse for CacheError {
    fn into_response(self) -> Response {
        let status = match self {
            CacheError::KeyNotFound(_) => axum::http::StatusCode::NOT_FOUND,
            CacheError::CapacityExceeded { .. } => axum::http::StatusCode::INSUFFICIENT_STORAGE,
            CacheError::NodeUnavailable(_) => axum::http::StatusCode::SERVICE_UNAVAILABLE,
            CacheError::TopologyMismatch { .. } => axum::http::StatusCode::CONFLICT,
            _ => axum::http::StatusCode::BAD_REQUEST,
        };
        let body = Json(serde_json::json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}

// -- Data types --
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn primary_for(&self, key: &str) -> Option<&CacheNodeConfig> {
        let p = partition::hash(key) % PARTITION_COUNT as u64;
        self.nodes.get(self.partition_map.get(p as usize)?)
    }

    pub fn update_version(&mut self) {
        self.version += 1;
        self.created_at_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.rebuild_partition_map();
    }

    fn rebuild_partition_map(&mut self) {
        let nodes: Vec<_> = self.nodes.keys().cloned().collect();
        if nodes.is_empty() {
            return;
        }
        self.partition_map = vec![nodes[0].clone(); PARTITION_COUNT];
        for (i, node) in nodes.iter().cycle().take(PARTITION_COUNT).enumerate() {
            self.partition_map[i] = node.clone();
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
        // Exponential moving average for P50
        let current_p50 = self.p50_latency_ms.load(Ordering::Relaxed);
        let new_p50 = if current_p50 == 0 { latency_ms } else { (current_p50 * 7 + latency_ms) / 8 };
        self.p50_latency_ms.store(new_p50, Ordering::Relaxed);

        // Keep max P99 (simplified; production uses sorted reservoir)
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

// -- Cache Engine --
#[derive(Clone)]
pub struct CacheEngine {
    primary: Arc<Cache<String, Arc<CacheEntry>>>,
    l1: Arc<Cache<String, Arc<CacheEntry>>>,
    topology: Arc<RwLock<Topology>>,
    stats: Arc<EngineStats>,
    hotkey_detector: Arc<hotkey::SlidingWindowDetector>,
    invalidation_tx: mpsc::Sender<InvalidationEvent>,
    node_id: String,
    max_capacity: u64,
}

#[derive(Debug, Clone)]
pub struct InvalidationEvent {
    pub key: String,
    pub pattern: Option<String>,
    pub scope: invalidation::InvalidationScope,
    pub timestamp_ms: u64,
    pub source_node: String,
}

impl CacheEngine {
    pub async fn new(
        config: CacheNodeConfig,
        hotkey_window_secs: u64,
        hotkey_p99_threshold_ms: u64,
    ) -> Result<Self, CacheError> {
        info!("initializing cache engine: node_id={}", config.node_id);

        let (tx, rx) = mpsc::channel(65536);
        tokio::spawn(async move {
            invalidation::process_invalidation_stream(rx).await;
        });

        let stats = Arc::new(EngineStats::new());
        let hotkey_detector = Arc::new(hotkey::SlidingWindowDetector::new(
            Duration::from_secs(hotkey_window_secs),
            hotkey_p99_threshold_ms,
        ));

        let engine = Self {
            primary: Arc::new(
                Cache::builder()
                    .max_capacity(config.max_capacity_mb)
                    .time_to_live(Duration::from_secs(DEFAULT_TTL_SECS))
                    .eviction_listener(|_k, _v, _cause| {
                        stats.evictions.fetch_add(1, Ordering::Relaxed);
                    })
                    .build(),
            ),
            l1: Arc::new(
                Cache::builder()
                    .max_capacity(1_000_000)
                    .time_to_live(Duration::from_secs(30))
                    .build(),
            ),
            topology: Arc::new(RwLock::new(Topology {
                version: 1,
                created_at_ms: 0,
                nodes: HashMap::new(),
                partition_map: vec![],
                metadata: HashMap::new(),
            })),
            stats,
            hotkey_detector,
            invalidation_tx: tx,
            node_id: config.node_id.clone(),
            max_capacity: config.max_capacity_mb,
        };

        // Initialize metrics
        metrics::register();

        Ok(engine)
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub async fn get(&self, key: &str) -> Result<Option<Arc<CacheEntry>>, CacheError> {
        let t0 = Instant::now();

        // Try L1 then primary
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
        let entry = Arc::new(CacheEntry {
            value,
            ttl_ms: ttl_secs * 1000,
            created_at_ms: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            version: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            tags,
            source_node: self.node_id.clone(),
        });

        self.primary.insert(key.to_string(), Arc::clone(&entry));
        self.l1.insert(key.to_string(), entry);
        
        let count = self.primary.entry_count();
        self.stats.active_keys.store(count, Ordering::Relaxed);
        self.stats.write_bytes.fetch_add(self.primary.entry_count() as u64, Ordering::Relaxed);
        counter!("cache.writes", 1);

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
        self.hotkey_detector.analyze(key).cloned()
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
            timestamp_ms: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn topology(&self) -> Topology {
        self.topology.read().clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

// -- Request/Response types --
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

// -- Main server logic --
pub struct CacheServer {
    engine: CacheEngine,
    start_time: Instant,
}

impl CacheServer {
    pub fn new(engine: CacheEngine) -> Self {
        Self {
            engine,
            start_time: Instant::now(),
        }
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
            .route("/ws", get(Self::ws_handler))
            .layer(CorsLayer::permissive())
            .layer(tower_http::trace::TraceLayer::new_for_http())
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
                uptime_seconds: state.start_time.elapsed().as_secs(),
            },
        }))
    }

    async fn health_handler(
        State(state): State<Arc<Self>>,
    ) -> Result<Json<HealthStatus>, CacheError> {
        Ok(Json(HealthStatus {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: state.start_time.elapsed().as_secs(),
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
        use metrics_exporter_prometheus::PrometheusBuilder;
        let encoder = PrometheusBuilder::new()
            .add_global_label("service", "cache-orbit")
            .build_encoder()?;
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        let response = Response::builder()
            .header("Content-Type", encoder.content_type())
            .body(axum::body::Body::from(buffer))
            .unwrap();
        Ok(response)
    }

    async fn ws_handler(
        State(state): State<Arc<Self>>,
        ws: WebSocketUpgrade,
    ) -> Result<impl IntoResponse, CacheError> {
        Ok(ws.on_upgrade(move |socket| handle_ws(socket, state)))
    }
}

async fn handle_ws(socket: WebSocket, state: Arc<CacheServer>) {
    let (mut sender, mut receiver) = socket.split();
    let mut ticker = tokio::time::interval(Duration::from_secs(2));
    
    let mut send_task = tokio::spawn(async move {
        loop {
            ticker.tick().await;
            let stats = state.engine.stats();
            if let Ok(json) = serde_json::to_string(&stats) {
                if sender.send(WsMessage::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}

// -- Main entry point --
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("cache_orbit=debug,tower_http=info,info")
        .json()
        .init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "6379".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    info!("starting Cache Orbit node on {}", addr);

    // Demo topology
    let mut nodes = HashMap::new();
    nodes.insert(
        "local".to_string(),
        CacheNodeConfig {
            node_id: "local".to_string(),
            listen_addr: format!("0.0.0.0:{}", port),
            datacenter: "local".to_string(),
            rack: "local".to_string(),
            partition_ids: (0..PARTITION_COUNT as u64).collect(),
            replicas: vec![],
            max_capacity_mb: 1_000_000,
            consistency: ConsistencyLevel::Weak,
            hotkey_replicas: 1,
        },
    );

    let topology = Topology {
        version: 1,
        created_at_ms: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        nodes,
        partition_map: vec!["local".to_string(); PARTITION_COUNT],
        metadata: HashMap::new(),
    };

    let config = topology.nodes.values().next().unwrap().clone();
    let engine = CacheEngine::new(config, HOTKEY_SLIDING_WINDOW_SECS, HOTKEY_THRESHOLD_P99_MS).await?;
    let server = CacheServer::new(engine);

    info!("🚀 Cache Orbit ready at http://{}", addr);
    Server::bind(&addr)
        .serve(server.router().into_make_service())
        .await?;

    Ok(())
}
