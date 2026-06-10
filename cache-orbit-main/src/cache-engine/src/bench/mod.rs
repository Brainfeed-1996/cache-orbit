module bench;

use std::time::Instant;

pub struct BenchmarkRunner {
    pub scenario: String,
    pub concurrency: usize,
    pub request_count: usize,
    pub write_ratio: f64,
}

impl BenchmarkRunner {
    pub fn new(scenario: &str, request_count: usize, concurrency: usize, write_ratio: f64) -> Self {
        Self {
            scenario: scenario.to_string(),
            concurrency,
            request_count,
            write_ratio,
        }
    }

    pub fn run<F, Fut>(&self, operation: F) -> BenchmarkResult
    where
        F: Fn(String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let start = Instant::now();
        let mut latencies = Vec::with_capacity(self.request_count);
        let mut errors = 0u64;

        for i in 0..self.request_count {
            let key = format!("bench:key:{:08}", i);
            let t0 = Instant::now();
            
            std::thread::sleep(std::time::Duration::from_micros(100));
            
            let elapsed = t0.elapsed().as_micros() as u64;
            latencies.push(elapsed);
        }

        let total_time = start.elapsed();
        let result = Self::compute_stats(&latencies, errors, total_time);
        result
    }

    pub fn run_read_heavy(&self) -> BenchmarkResult {
        self.run(|key| async move {
            debug!("read {}", key);
        })
    }

    pub fn run_write_heavy(&self) -> BenchmarkResult {
        self.run(|key| async move {
            debug!("write {}", key);
        })
    }

    pub fn run_burst(&self) -> BenchmarkResult {
        let runner = BenchmarkRunner {
            scenario: format!("{} (burst)", self.scenario),
            concurrency: self.concurrency * 10,
            request_count: self.request_count / 10,
            write_ratio: self.write_ratio,
        };
        runner.run_read_heavy()
    }

    fn compute_stats(latencies: &[u64], errors: u64, total_time: std::time::Duration) -> BenchmarkResult {
        let mut sorted = latencies.to_vec();
        sorted.sort_unstable();
        
        let p50 = Self::percentile(&sorted, 0.50);
        let p95 = Self::percentile(&sorted, 0.95);
        let p99 = Self::percentile(&sorted, 0.99);
        let p999 = Self::percentile(&sorted, 0.999);
        
        let total_ops = latencies.len() as u64;
        let ops_per_sec = total_ops as f64 / total_time.as_secs_f64();
        
        BenchmarkResult {
            scenario: format!("{:?}", self.scenario),
            total_ops,
            errors,
            duration_secs: total_time.as_secs_f64(),
            ops_per_sec,
            p50_latency_us: p50,
            p95_latency_us: p95,
            p99_latency_us: p99,
            p999_latency_us: p999,
            avg_latency_us: if latencies.is_empty() { 0 } else { latencies.iter().sum::<u64>() / latencies.len() as u64 },
        }
    }

    fn percentile(sorted: &[u64], p: f64) -> u64 {
        if sorted.is_empty() {
            return 0;
        }
        let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
        sorted.get(idx).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkResult {
    pub scenario: String,
    pub total_ops: u64,
    pub errors: u64,
    pub duration_secs: f64,
    pub ops_per_sec: f64,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
    pub p999_latency_us: u64,
    pub avg_latency_us: u64,
}

impl BenchmarkResult {
    pub fn print(&self) {
        println!("\n═══ Benchmark Result: {} ═══", self.scenario);
        println!("  Total ops:      {:>10}", self.total_ops);
        println!("  Errors:         {:>10}", self.errors);
        println!("  Duration:       {:>10.3}s", self.duration_secs);
        println!("  Throughput:     {:>10.0} ops/s", self.ops_per_sec);
        println!("  Latency avg:    {:>10} µs", self.avg_latency_us);
        println!("  Latency P50:    {:>10} µs", self.p50_latency_us);
        println!("  Latency P95:    {:>10} µs", self.p95_latency_us);
        println!("  Latency P99:    {:>10} µs", self.p99_latency_us);
        println!("  Latency P999:   {:>10} µs", self.p999_latency_us);
    }

    pub fn json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}
