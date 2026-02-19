use argon2::{Argon2, PasswordHash, PasswordVerifier};
use sqlx::PgPool;

use crate::{
    domain::{User, UserName, UserPassword},
    services::ServiceError,
};

use super::hash_password;

#[tracing::instrument(name = "services::create_user_account", skip_all)]
pub async fn create_user(
    username: UserName,
    password: UserPassword,
    hasher: &Argon2<'_>,
    pool: &PgPool,
) -> Result<Option<User>, ServiceError> {
    let hash = hash_password(password.as_str(), hasher)?;

    let rec_opt = sqlx::query!(
        r#"
        INSERT INTO users_main (username, password_hash)
        VALUES ($1, $2)
        ON CONFLICT (username) DO NOTHING
        RETURNING id
        "#,
        username.as_str(),
        hash
    )
    .fetch_optional(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    Ok(rec_opt.map(|rec| User::new(rec.id, username)))
}

#[tracing::instrument(name = "services::verify_user_password", skip_all)]
pub async fn authenticate_user(
    username: UserName,
    password: UserPassword,
    hasher: &Argon2<'_>,
    pool: &PgPool,
) -> Result<User, ServiceError> {
    let rec = sqlx::query!(
        r#"
        SELECT id, password_hash
        FROM users_main
        WHERE username = $1
        "#,
        username.as_str()
    )
    .fetch_optional(pool)
    .await
    .map_err(ServiceError::DatabaseError)?;

    let Some(rec) = rec else {
        return Err(ServiceError::AuthError);
    };

    let hash = PasswordHash::new(&rec.password_hash)
        .map_err(|e| anyhow::anyhow!("invalid password hash: {e}"))
        .map_err(ServiceError::Other)?;

    let password_str = password.as_str();
    if hasher
        .verify_password(password_str.as_bytes(), &hash)
        .is_err()
    {
        return Err(ServiceError::AuthError);
    }

    Ok(User::new(rec.id, username))
}
