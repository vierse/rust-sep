use std::sync::atomic::{AtomicUsize, Ordering::Acquire};
use time::OffsetDateTime;

#[derive(Default)]
pub struct Day {
    hours: [Hour; 24],
}

#[derive(Default)]
pub struct Hour {
    redirect: AtomicUsize,
    // webpage: usize,
}

#[derive(Default)]
pub struct Metrics {
    week_days: [Day; 7],
    // woul be 31 * 12 * 24 * 16 bytes
    // months: Box<[[Day; 31]; 12]>,
}

impl Metrics {
    pub async fn log_redirect(&self) {
        let date_time = OffsetDateTime::now_utc();
        let date = date_time.date();
        let time = date_time.time();

        let week_day = date.weekday().number_from_monday() as usize;
        // let month_day = date.day() as usize;
        // let month = date.month() as usize;

        let hour = time.hour() as usize;

        self.week_days[week_day].hours[hour]
            .redirect
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        // self.months[month][month_day].hours[hour].redirect += 1;
    }
}

impl Day {
    pub fn avg_hourly_redirects(&self) -> f64 {
        self.total_redirects() as f64 / self.hours.len() as f64
    }

    pub fn total_redirects(&self) -> usize {
        self.hours.iter().map(|h| h.redirect.load(Acquire)).sum()
    }

    /// returns the hour that has seen the most redirects
    pub fn most_redirects(&self) -> usize {
        let (idx, _reds) = self
            .hours
            .iter()
            .map(|h| h.redirect.load(Acquire))
            .into_iter()
            .enumerate()
            .max_by_key(|(_, reds)| *reds)
            // this should never panick as self.hours is a fixed size array
            .unwrap();
        idx
    }

    /// computes the fraction the given hour has seen of all redirects
    pub fn redirects_fraction(&self, hour: usize) -> anyhow::Result<f64> {
        anyhow::ensure!(hour < self.hours.len(), "given hour doesn't fit in a day");

        Ok(self.hours[hour].redirect.load(Acquire) as f64 / self.total_redirects() as f64)
    }

    pub fn redirects(&self, hour: usize) -> anyhow::Result<usize> {
        anyhow::ensure!(hour < self.hours.len(), "given hour doesn't fit in a day");

        Ok(self.hours[hour].redirect.load(Acquire))
    }
}
