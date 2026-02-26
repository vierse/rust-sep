use std::{collections::HashMap, str::FromStr};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    api::error::ApiError,
    app::{
        AppState,
        usage_metrics::{Category, CategorySet, HourSet},
    },
};

#[derive(Clone, Copy, Debug)]
enum MetricsCat {
    Cat(Category),
    Total,
}

const CAT_NUM: u32 = Category::AuthenticateUser as u32 + 1;

impl MetricsCat {
    fn to_bit_idx(self) -> u32 {
        match self {
            MetricsCat::Cat(c) => c as u32,
            MetricsCat::Total => CAT_NUM,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct MetricsCatSet(u32);

struct MetricsCatSetIter(u32, usize);

impl Iterator for MetricsCatSetIter {
    type Item = MetricsCat;

    fn next(&mut self) -> Option<Self::Item> {
        let next_idx = self.0.trailing_zeros() + 1;
        if next_idx == 33 {
            return None;
        }
        self.1 += next_idx as usize;
        self.0 >>= next_idx;

        MetricsCat::try_from(self.1 as u32 - 1).ok()
    }
}

impl MetricsCatSet {
    pub fn iter(&self) -> MetricsCatSetIter {
        MetricsCatSetIter(self.0, 0)
    }
}

impl MetricsCatSet {
    fn set(&mut self, cat: MetricsCat) -> Self {
        self.0 |= 1u32.wrapping_shl(cat.to_bit_idx());
        *self
    }

    fn is_set(&self, cat: MetricsCat) -> bool {
        (self.0 & 1u32.wrapping_shl(cat.to_bit_idx())) != 0
    }
}

impl FromStr for MetricsCat {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num: u32 = s.parse().map_err(|_| {
            ApiError::public(StatusCode::BAD_REQUEST, "given parameter isn't a number")
        })?;

        num.try_into()
    }
}

impl TryFrom<u32> for MetricsCat {
    type Error = ApiError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            c if c < CAT_NUM => {
                Ok(MetricsCat::Cat(c.try_into().map_err(|e| {
                    ApiError::public(StatusCode::BAD_REQUEST, e)
                })?))
            }
            CAT_NUM => Ok(MetricsCat::Total),
            _ => Err(ApiError::public(
                StatusCode::BAD_REQUEST,
                "given ordinal is too high",
            )),
        }
    }
}

impl MetricsCat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cat(c) => c.as_str(),
            Self::Total => "total",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Params {
    weekdays: String,
    hours: Option<String>,
}

pub async fn metrics(
    State(app): State<AppState>,
    Query(params): Query<Params>,
) -> Result<Response, ApiError> {
    let cats = params
        .weekdays
        .split(',')
        .map(MetricsCat::from_str)
        .try_fold(MetricsCatSet(0), |mut set, cat| -> Result<_, ApiError> {
            Ok(set.set(cat?))
        })?;

    let mut resp: HashMap<&'static str, _> = HashMap::new();

    if let Some(hours) = params.hours {
        let mut hours = hours.split(',').map(|h| h.parse());

        let Ok(hours) = hours.try_fold(
            HourSet::new(),
            |mut set, hour| -> Result<_, <usize as FromStr>::Err> { Ok(set.set(hour?)) },
        ) else {
            return Err(ApiError::public(
                StatusCode::BAD_REQUEST,
                "hour wasn't given in number",
            ));
        };

        let it = app
            .usage_metrics
            .total_usage_by_day_in_hours_bitset(CategorySet::from_raw(cats.0), hours);

        for (day_idx, day_it) in it.enumerate() {
            for (cat_hits, cat) in day_it.zip(cats.iter()) {
                let cat_name = cat.as_str();
                resp.entry(cat_name).or_insert(vec![0; 7])[day_idx] = cat_hits;
            }
        }

        if cats.is_set(MetricsCat::Total) {
            let hits: Vec<_> = app
                .usage_metrics
                .total_usage_daily_in_hours(hours)
                .collect();

            resp.insert("total", hits);
        }
    } else {
        let it = app
            .usage_metrics
            .total_usage_by_day_in_bitset(CategorySet::from_raw(cats.0));

        for (day_idx, day_it) in it.enumerate() {
            for (cat_hits, cat) in day_it.zip(cats.iter()) {
                let cat_name = cat.as_str();
                resp.entry(cat_name).or_insert(vec![0; 7])[day_idx] = cat_hits;
            }
        }
        if cats.is_set(MetricsCat::Total) {
            let hits: Vec<_> = app.usage_metrics.total_usage_daily().collect();

            resp.insert("total", hits);
        }
    };

    Ok((StatusCode::OK, Json(resp)).into_response())
}
