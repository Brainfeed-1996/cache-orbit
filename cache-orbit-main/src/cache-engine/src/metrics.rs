use lazy_static::lazy_static;
use prometheus::{
    Counter, Encoder, Gauge, Histogram, HistogramOpts, Opts, Registry, TextEncoder,
    proto::MetricFamily,
};
use std::sync::Arc;

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref ENCODER: TextEncoder = TextEncoder::new();
}

pub fn register() {
    lazy_static::initialize(&REGISTRY);
}

pub fn counter(name: &str, help: &str) -> Option<Counter> {
    let opts = Opts::new(name, help);
    Counter::with_opts(opts).ok().map(|c| {
        let _ = REGISTRY.register(Box::new(c.clone()));
        c
    })
}

pub fn gauge(name: &str, help: &str) -> Option<Gauge> {
    let opts = Opts::new(name, help);
    Gauge::with_opts(opts).ok().map(|g| {
        let _ = REGISTRY.register(Box::new(g.clone()));
        g
    })
}

pub fn histogram(name: &str, help: &str, buckets: Vec<f64>) -> Option<Histogram> {
    let opts = HistogramOpts::new(name, help).buckets(buckets);
    Histogram::with_opts(opts).ok().map(|h| {
        let _ = REGISTRY.register(Box::new(h.clone()));
        h
    })
}

pub fn gather() -> Vec<MetricFamily> {
    REGISTRY.gather()
}

pub fn encode() -> Result<String, prometheus::Error> {
    let mut buffer = Vec::new();
    ENCODER.encode(&REGISTRY.gather(), &mut buffer)?;
    Ok(String::from_utf8(buffer).unwrap_or_default())
}
