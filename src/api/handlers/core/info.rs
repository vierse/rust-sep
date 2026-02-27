use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::{
    api::{
        AppState,
        error::ApiError,
        extract::{MaybeToken, MaybeUser},
        token::OwnerToken,
    },
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
    MaybeUser(session_id): MaybeUser,
    MaybeToken(token): MaybeToken<OwnerToken>,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Response, ApiError> {
    let alias: LinkAlias = alias.try_into()?;

    let link = super::fetch_link(&alias, &app).await?;

    let owned = super::is_user_owned(session_id.as_ref(), &link, &app)
        || super::is_token_active(token.as_ref(), &link);

    let mut response = LinkInfoResponse {
        protected: link.password_hash.is_some(),
        ..Default::default()
    };

    if owned {
        response.owned = true;
        response.data = Some(services::list_link_metrics(link.id, &app.pool).await?);
    }

    Ok((StatusCode::OK, Json(response)).into_response())
}
