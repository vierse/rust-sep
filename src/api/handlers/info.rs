use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use time::OffsetDateTime;

use crate::{
    api::{
        error::ApiError,
        extract::MaybeToken,
        handlers::{OwnerToken, fetch_link},
    },
    app::AppState,
    domain::LinkAlias,
    services::{self, LinkMetricsQuery},
};

#[derive(Serialize, Default)]
pub struct LinkInfoResponse {
    owned: bool,
    protected: bool,
    data: Option<LinkMetricsQuery>,
}

pub async fn link_info(
    MaybeToken(token): MaybeToken<OwnerToken>,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let alias: LinkAlias = alias.try_into()?;

    let link = fetch_link(&alias, &app).await?;

    let mut response = LinkInfoResponse::default();

    if let Some(token) = token {
        let now_s = OffsetDateTime::now_utc().unix_timestamp();
        if token.is_owner(link.id, now_s, link.created_at.unix_timestamp()) {
            let metrics = services::list_link_metrics(link.id, &app.pool).await?;
            response.owned = true;
            response.data = Some(metrics);
        }
    }

    if link.password_hash.is_some() {
        response.protected = true;
    }

    Ok((StatusCode::OK, Json(response)).into_response())
}
