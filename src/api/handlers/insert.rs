use axum::{
Json,
extract::State,
http::StatusCode,
response::IntoResponse,
};

use crate::app::AppState;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct InsertRequest {
    alias: String,
    url: String,
}

#[derive(Serialize)]
pub struct InsertResponse {
    alias: String,
}

pub async fn insert(State(app): State<AppState>, Json(InsertRequest{alias, url}): Json<InsertRequest> ) -> impl IntoResponse {
   
   if let Err(_) = validate_alias(&alias){
    return (StatusCode::BAD_REQUEST).into_response();
   }
    match app.insert(&alias, &url).await {
    Ok(()) => (StatusCode::CREATED, Json(InsertResponse { alias })).into_response(),
    Err(e) => {
        tracing::error!(error = %e, "insert request err");
        (StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }


   }
    
    
}

enum AliasError {
    TooShort,
    TooLong,
    InvalidCharacters,
    
}

fn validate_alias(alias: &str) -> Result<(), AliasError>{
    const MIN_ALIAS_LENGTH: usize = 6;
    const MAX_ALIAS_LENGTH: usize = 20;
if alias.len() < MIN_ALIAS_LENGTH {
    return Err(AliasError::TooShort);
}
if alias.len() > MAX_ALIAS_LENGTH {
    return Err(AliasError::TooLong);
}
if alias.contains(|c: char| !c.is_alphanumeric()) {
    return Err(AliasError::InvalidCharacters);
}
Ok(())
}


