use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use arc_swap::ArcSwap;
use dashmap::DashMap;
use time::OffsetDateTime;

pub type MetricsMap = DashMap<i64, MetricsValue>;

pub struct Metrics {
    current: ArcSwap<MetricsMap>,
}

pub struct MetricsValue {
    hits: AtomicI64,
    last_access_s: AtomicI64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            current: ArcSwap::from_pointee(DashMap::new()),
        }
    }

    pub fn record_hit(&self, link_id: i64) {
        let now_s = OffsetDateTime::now_utc().unix_timestamp();

        let map = self.current.load();
        let val = map.entry(link_id).or_insert(MetricsValue::new(now_s));

        // increment hitcount
        val.hits.fetch_add(1, Ordering::Relaxed);

        // update last access timestamp
        let mut last_access_s = val.last_access_s.load(Ordering::Relaxed);
        while now_s > last_access_s {
            match val.last_access_s.compare_exchange_weak(
                last_access_s,
                now_s,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(next) => last_access_s = next,
            }
        }
    }

    pub fn swap_map(&self) -> Arc<MetricsMap> {
        self.current.swap(Arc::new(DashMap::new()))
    }
}

impl MetricsValue {
    pub fn new(last_access_s: i64) -> Self {
        Self {
            hits: AtomicI64::new(1),
            last_access_s: AtomicI64::new(last_access_s),
        }
    }

    pub fn hits(&self) -> i64 {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn last_access_s(&self) -> i64 {
        self.last_access_s.load(Ordering::Relaxed)
    }
}
