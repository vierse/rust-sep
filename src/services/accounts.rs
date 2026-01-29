use anyhow::{Context, anyhow};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand_core::OsRng;
use sqlx::PgPool;

use crate::services::ServiceError;

#[tracing::instrument(name = "services::create_user_account", skip_all)]
pub async fn create_user_account(
    username: &str,
    password: &str,
    hasher: &Argon2<'_>,
    pool: &PgPool,
) -> Result<i64, ServiceError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = hasher
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| anyhow!("failed to hash"))?;

    let rec = sqlx::query!(
        r#"
        INSERT INTO users_main (username, password_hash)
        VALUES ($1, $2)
        ON CONFLICT (username) DO NOTHING
        RETURNING id
        "#,
        username,
        hash.to_string()
    )
    .fetch_optional(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    Ok(rec.unwrap().id)
}

#[tracing::instrument(name = "services::verify_user_password", skip_all)]
pub async fn verify_user_password(
    username: &str,
    password: &str,
    hasher: &Argon2<'_>,
    pool: &PgPool,
) -> Result<Option<i64>, ServiceError> {
    let rec = sqlx::query!(
        r#"
        SELECT id, password_hash
        FROM users_main
        WHERE username = $1
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("failed to fetch user password hash")?;

    let Some(rec) = rec else {
        return Ok(None);
    };

    let parsed_hash = PasswordHash::new(&rec.password_hash)
        .map_err(|e| anyhow::anyhow!("invalid password hash: {e}"))?;

    if hasher
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
    {
        Ok(Some(rec.id))
    } else {
        Ok(None)
    }
}
