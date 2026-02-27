use axum::{
    extract::{Path, State},
    response::Redirect,
};

use crate::{
    api::{
        AppState, error::ApiError, extract::MaybeToken, state::CachedLinkType, token::UnlockToken,
    },
    domain::LinkAlias,
    services,
};

pub async fn redirect(
    MaybeToken(token): MaybeToken<UnlockToken>,
    State(app): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Redirect, ApiError> {
    let alias: LinkAlias = alias.try_into()?;
    redirect_impl(&app, alias, None, token).await
}

pub async fn redirect_indexed(
    MaybeToken(token): MaybeToken<UnlockToken>,
    State(app): State<AppState>,
    Path((alias, idx)): Path<(String, usize)>,
) -> Result<Redirect, ApiError> {
    let alias: LinkAlias = alias.try_into()?;
    redirect_impl(&app, alias, Some(idx), token).await
}

async fn redirect_impl(
    app: &AppState,
    alias: LinkAlias,
    idx_opt: Option<usize>,
    token: Option<UnlockToken>,
) -> Result<Redirect, ApiError> {
    let link = super::fetch_link(&alias, app).await?;

    // redirect if link is locked and user has no matching token
    let unlocked = super::is_token_active(token.as_ref(), &link);
    if link.password_hash.is_some() && !unlocked {
        return Ok(Redirect::temporary(&format!("/unlock/{}", alias.as_str())));
    }

    match link.kind {
        CachedLinkType::Redirect => {
            if idx_opt.is_some() {
                return Err(ApiError::not_found());
            }
            app.metrics.record_hit(link.id);
            Ok(Redirect::temporary(&link.url))
        }
        CachedLinkType::Collection => match idx_opt {
            Some(idx) => {
                let url =
                    services::query_collection_url_by_pos(link.id, idx as i32, &app.pool).await?;
                app.metrics.record_hit(link.id);
                Ok(Redirect::temporary(&url))
            }
            None => Ok(Redirect::temporary(&format!(
                "/collection/{}",
                alias.as_str()
            ))),
        },
    }
}
