use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use arc_swap::ArcSwap;
use dashmap::DashMap;

pub type MetricsMap = DashMap<i64, AtomicI64>;

pub struct Metrics {
    current: ArcSwap<MetricsMap>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            current: ArcSwap::from_pointee(DashMap::new()),
        }
    }

    pub fn record_hit(&self, link_id: i64) {
        let map = self.current.load();

        let val = map.entry(link_id).or_insert(AtomicI64::new(1));

        val.fetch_add(1, Ordering::Relaxed);
    }

    pub fn swap_map(&self) -> Arc<MetricsMap> {
        self.current.swap(Arc::new(DashMap::new()))
    }
}
