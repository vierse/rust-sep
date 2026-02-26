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
    pub categories: [AtomicUsize; Category::AuthenticateUser as usize + 1],
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum Category {
    Redirect,
    Shorten,
    RecentlyAdded,
    AuthenticateSession,
    AuthenticateUser,
}

impl TryFrom<u32> for Category {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Category::Redirect),
            1 => Ok(Category::Shorten),
            2 => Ok(Category::RecentlyAdded),
            3 => Ok(Category::AuthenticateSession),
            4 => Ok(Category::AuthenticateUser),
            _ => Err("value to large to be a category"),
        }
    }
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Redirect => "redirect",
            Category::Shorten => "shorten",
            Category::RecentlyAdded => "recently added",
            Category::AuthenticateSession => "authenticate session",
            Category::AuthenticateUser => "authenticate user",
        }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct CategorySet(u32);

impl CategorySet {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn set(&mut self, cat: Category) {
        self.0 |= 1 << cat as u32;
    }

    pub fn is_set(&self, cat: Category) -> bool {
        self.0 & 1u32.wrapping_shl(cat as u32) != 0
    }

    pub fn is_set_raw(&self, cat: u32) -> bool {
        self.0 & 1u32.wrapping_shl(cat) != 0
    }
    pub fn from_raw(cats: u32) -> Self {
        Self(cats)
    }
}

impl Default for CategorySet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
/// 24 hours => 24 bits
pub struct HourSet(u32);

impl HourSet {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn set(&mut self, hour: usize) -> Self {
        self.0 |= 1u32.wrapping_shl(hour as u32);
        *self
    }

    pub fn is_set(&self, hour: usize) -> bool {
        self.0 & 1u32.wrapping_shl(hour as u32) != 0
    }

    pub fn from_raw(cats: u32) -> Self {
        Self(cats)
    }
}

impl Default for HourSet {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn total_usage_daily(&self) -> impl Iterator<Item = usize> {
        self.week_days.iter().map(|d| d.total_usage())
    }

    pub fn total_usage_daily_in_hours(&self, hours: HourSet) -> impl Iterator<Item = usize> {
        self.week_days
            .iter()
            .map(move |d| d.total_usage_in_hours(hours))
    }

    pub fn total_usage_by_day_in_bitset(
        &self,
        cat: CategorySet,
    ) -> impl Iterator<Item = impl Iterator<Item = usize>> {
        self.week_days
            .iter()
            .map(move |day| day.total_usage_in_bitset(cat))
    }

    pub fn total_usage_by_day_in_hours_bitset(
        &self,
        cats: CategorySet,
        hours: HourSet,
    ) -> impl Iterator<Item = impl Iterator<Item = usize>> {
        self.week_days
            .iter()
            .map(move |day| day.total_usage_in_hours_bitset(cats, hours))
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

    pub fn total_usage_in_bitset(&self, cats: CategorySet) -> impl Iterator<Item = usize> {
        (0..=Category::AuthenticateUser as u32)
            .filter(move |idx| cats.is_set_raw(*idx))
            .map(|idx| {
                self.hours
                    .iter()
                    // TODO: turn the access into column major
                    .map(|h| h.categories[idx as usize].load(Ordering::Relaxed))
                    .sum()
            })
    }

    pub fn total_usage_in_hours_bitset(
        &self,
        cats: CategorySet,
        hours: HourSet,
    ) -> impl Iterator<Item = usize> {
        (0..=Category::AuthenticateUser as u32)
            .filter(move |idx| cats.is_set_raw(*idx))
            .map(move |idx| {
                self.hours
                    .iter()
                    .enumerate()
                    .filter_map(move |(hour_idx, cs)| {
                        if hours.is_set(hour_idx) {
                            Some(cs)
                        } else {
                            None
                        }
                    })
                    // TODO: turn the access into column major
                    .map(|h| h.categories[idx as usize].load(Ordering::Relaxed))
                    .sum()
            })
    }

    pub fn total_usage(&self) -> usize {
        self.hours.iter().map(|h| h.sum()).sum()
    }

    pub fn total_usage_in_hours(&self, hours: HourSet) -> usize {
        self.hours
            .iter()
            .enumerate()
            .filter(|&(hour_idx, _)| hours.is_set(hour_idx))
            .map(|(_, h)| h.sum())
            .sum()
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

    /// returns the usage a category in an hour / total_occurances
    pub fn usage_frequency_in(&self, hour: usize, cat: Category) -> anyhow::Result<f64> {
        anyhow::ensure!(hour < self.hours.len(), "given hour doesn't fit in a day");

        let total_usage = self.total_usage_in(cat) as f64;

        anyhow::ensure!(
            total_usage > 0.,
            "there haven't been any hits of the given category logged yet"
        );

        Ok(self.hours[hour].categories[cat as usize].load(Ordering::Relaxed) as f64 / total_usage)
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
