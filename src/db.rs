use anyhow::{Context, Result};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use sqlx::{Pool, Postgres, types::time::OffsetDateTime};

pub struct Database {
    pool: Pool<Postgres>,
}

impl Database {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn remove(&self, alias: &str) -> Result<bool> {
        let rec = sqlx::query(
            r#"
            DELETE FROM links
            WHERE alias = $1
            "#,
        )
        .bind(alias)
        .execute(&self.pool)
        .await
        .context("connection failed while removing alias")?;

        Ok(rec.rows_affected() > 0)
    }

    /// Creates a new user with a hashed password. Returns the new user's ID.
    pub async fn create_user(&self, username: &str, password: &str) -> Result<i64> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("failed to hash password: {e}"))?
            .to_string();

        let rec = sqlx::query!(
            r#"
            INSERT INTO users (username, password_hash)
            VALUES ($1, $2)
            RETURNING id
            "#,
            username,
            password_hash
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to insert user")?;

        Ok(rec.id)
    }

    /// Verifies login credentials. Returns Some(user_id) if valid, None if invalid.
    pub async fn verify_user_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<i64>> {
        let rec = sqlx::query!(
            r#"
            SELECT id, password_hash
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await
        .context("failed to fetch user password hash")?;

        let Some(rec) = rec else {
            return Ok(None);
        };

        let parsed_hash = PasswordHash::new(&rec.password_hash)
            .map_err(|e| anyhow::anyhow!("invalid password hash: {e}"))?;

        if Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            Ok(Some(rec.id))
        } else {
            Ok(None)
        }
    }

    /// Creates a login session for a user. Called after successful login.
    pub async fn create_session(
        &self,
        user_id: i64,
        session_token: &str,
        expires_at: OffsetDateTime,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO sessions (user_id, session_token, expires_at)
            VALUES ($1, $2, $3)
            "#,
            user_id,
            session_token,
            expires_at
        )
        .execute(&self.pool)
        .await
        .context("failed to insert session")?;

        Ok(())
    }

    /// Deletes a session (logout). Returns true if a session was deleted.
    pub async fn delete_session(&self, session_token: &str) -> Result<bool> {
        let rec = sqlx::query!(
            r#"
            DELETE FROM sessions
            WHERE session_token = $1
            "#,
            session_token
        )
        .execute(&self.pool)
        .await
        .context("failed to delete session")?;

        Ok(rec.rows_affected() > 0)
    }

    /// Gets user ID from a valid (non-expired) session token. Used to check if user is logged in.
    pub async fn get_user_id_by_session(&self, session_token: &str) -> Result<Option<i64>> {
        let rec = sqlx::query!(
            r#"
            SELECT user_id
            FROM sessions
            WHERE session_token = $1
              AND expires_at > now()
            "#,
            session_token
        )
        .fetch_optional(&self.pool)
        .await
        .context("failed to fetch session user_id")?;

        Ok(rec.map(|row| row.user_id))
    }
}
