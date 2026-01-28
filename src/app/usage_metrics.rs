use std::sync::atomic::{AtomicUsize, Ordering::Acquire};
use time::OffsetDateTime;

#[derive(Default)]
pub struct MetricsDay {
    hours: [Hour; 24],
}

#[derive(Default)]
pub struct Metrics {
    week_days: [MetricsDay; 7],
    // woul be 31 * 12 * 24 * 16 bytes
    // months: Box<[[Day; 31]; 12]>,
}

macro_rules! tracked_categories {
    ($($cat:ident, $cat_camel:ident);*) => {
        #[derive(Default)]
        pub struct Hour {
            $($cat: AtomicUsize,)*
        }

        #[derive(Clone, Copy)]
        pub enum Category {
            $($cat_camel,)*
        }

        impl Metrics {
            pub async fn log(&self, cat: Category) {
                let date_time = OffsetDateTime::now_utc();
                let date = date_time.date();
                let time = date_time.time();

                let week_day = date.weekday().number_from_monday() as usize;
                // let month_day = date.day() as usize;
                // let month = date.month() as usize;

                let hour = time.hour() as usize;

                match cat {
                    $(
                        Category::$cat_camel =>
                            self.week_days[week_day].hours[hour]
                                .$cat
                                .fetch_add(1, std::sync::atomic::Ordering::AcqRel),
                    )*
                };
            }

            pub fn most_frequented_day_cat(&self, cat: Category) -> usize {
                match cat {
                    $(
                        Category::$cat_camel =>
                            {
                                let (idx, _reds) =
                                                self
                                                .week_days
                                                .iter()
                                                .map(|day| day.total_usage_in(cat))
                                                .enumerate()
                                                .max_by_key(|(_, reds)| *reds)
                                                // this should never panick as self.week_days is a fixed size array
                                                .unwrap();
                            idx
                        },
                    )*
                }
            }

            pub fn total_usage_in(&self, cat: Category) -> usize {
                use Category::*;
                match cat {
                    $(
                        $cat_camel => self.week_days.iter().map(|day| day.total_usage_in(cat)).sum(),
                    )*
                }
            }
        }

        impl MetricsDay {
            pub fn avg_hourly_redirects(&self, cat: Category) -> f64 {
                self.total_usage_in(cat) as f64 / self.hours.len() as f64
            }

            pub fn total_usage_in(&self, cat: Category) -> usize {
                match cat {
                    $(
                        Category::$cat_camel => {
                            self.hours.iter().map(|h| h.$cat.load(Acquire)).sum()
                        }
                    )*
                }
            }

            /// returns the hour that has seen the most redirects
            pub fn most_usage(&self, cat: Category) -> usize {
                let (idx, _reds) = match cat {
                    $(
                        Category::$cat_camel =>
                             self
                                .hours
                                .iter()
                                .map(|h| h.redirect.load(Acquire))
                                .into_iter()
                                .enumerate()
                                .max_by_key(|(_, reds)| *reds)
                                // this should never panick as self.hours is a fixed size array
                                .unwrap(),
                    )*
                };
                idx
            }

            /// computes the fraction the given hour has seen of all redirects
            pub fn usage_fraction(&self, hour: usize, cat: Category) -> anyhow::Result<f64> {
                anyhow::ensure!(hour < self.hours.len(), "given hour doesn't fit in a day");

                Ok(self.hours[hour].redirect.load(Acquire) as f64 / self.total_usage_in(cat) as f64)
            }

            pub fn usage(&self, hour: usize, cat: Category) -> anyhow::Result<usize> {
                anyhow::ensure!(hour < self.hours.len(), "given hour doesn't fit in a day");

                match cat {
                    $(
                        Category::$cat_camel =>
                            Ok(self.hours[hour].$cat.load(Acquire)),
                    )*
                }
            }
        }
    };

}

tracked_categories!(redirect, Redirect; recent, Recent; shorten, Shorten);
