use std::sync::atomic::{AtomicUsize, Ordering};
use time::OffsetDateTime;

#[derive(Default)]
pub struct MetricsDay {
    pub hours: [Hour; 24],
}

#[derive(Default)]
pub struct Metrics {
    pub week_days: [MetricsDay; 7],
}

#[derive(Default)]
pub struct Hour {
    pub categories: [AtomicUsize; 6],
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Category {
    Redirect,
    Recent,
    Shorten,
    RecentlyAdded,
    AuthenticateSession,
    AuthenticateUser,
}

impl Metrics {
    pub fn log(&self, cat: Category) {
        let date_time = OffsetDateTime::now_utc();
        let date = date_time.date();
        let time = date_time.time();
        let week_day = date.weekday().number_from_monday() as usize;
        let hour = time.hour() as usize;

        self.week_days[week_day].hours[hour].categories[cat as usize]
            .fetch_add(1, Ordering::Relaxed);
    }

    /// computes the day which saw the most hits in a given category
    pub fn most_frequented_weekday_in(&self, cat: Category) -> usize {
        let (idx, _) = self
            .week_days
            .iter()
            .map(|day| day.total_usage_in(cat))
            .enumerate()
            .max_by_key(|(_, reds)| *reds)
            .unwrap();
        idx
    }
    pub fn total_usage_in(&self, cat: Category) -> usize {
        self.week_days
            .iter()
            .map(|day| day.total_usage_in(cat))
            .sum()
    }
}

impl MetricsDay {
    pub fn avg_hourly_hits_in(&self, cat: Category) -> f64 {
        self.total_usage_in(cat) as f64 / self.hours.len() as f64
    }

    pub fn total_usage_in(&self, cat: Category) -> usize {
        self.hours
            .iter()
            .map(|h| h.categories[cat as usize].load(Ordering::Relaxed))
            .sum()
    }

    pub fn total_usage(&self) -> usize {
        self.hours.iter().map(|h| h.sum()).sum()
    }

    /// returns the hour that has seen the most hits in a category
    pub fn most_hit_hour(&self, cat: Category) -> usize {
        let (idx, _reds) = self
            .hours
            .iter()
            .map(|h| h.categories[cat as usize].load(Ordering::Relaxed))
            .enumerate()
            .max_by_key(|(_, reds)| *reds)
            .unwrap();

        idx
    }

    pub fn most_hits_total(&self) -> usize {
        let (idx, _reds) = self
            .hours
            .iter()
            .map(|h| h.sum())
            .enumerate()
            .max_by_key(|(_, reds)| *reds)
            .unwrap();

        idx
    }

    /// returns the usage a category in an hour /  total_occurances
    pub fn usage_frequency_in(&self, hour: usize, cat: Category) -> anyhow::Result<f64> {
        anyhow::ensure!(hour < self.hours.len(), "given hour doesn't fit in a day");

        Ok(
            self.hours[hour].categories[cat as usize].load(Ordering::Relaxed) as f64
                / self.total_usage_in(cat) as f64,
        )
    }

    pub fn usage(&self, hour: usize, cat: Category) -> anyhow::Result<usize> {
        anyhow::ensure!(hour < self.hours.len(), "given hour doesn't fit in a day");

        Ok(self.hours[hour].categories[cat as usize].load(Ordering::Relaxed))
    }
}

impl Hour {
    pub fn sum(&self) -> usize {
        self.categories
            .iter()
            .fold(0, |acc, e| acc + e.load(Ordering::Relaxed))
    }
}
