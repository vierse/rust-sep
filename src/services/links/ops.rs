use anyhow::{Context, anyhow};
use argon2::Argon2;
use sqids::Sqids;
use sqlx::{PgPool, Postgres, Transaction};

use crate::{
    domain::{LinkAlias, LinkPassword, Url, UserId},
    services::{CollectionError, LinkError, MAX_COLLECTION_ITEMS, ServiceError, hash_password},
};

/// Create a new link for the provided URL
#[tracing::instrument(
    name = "services::create_link",
    skip(generator, pool, password, hasher)
)]
pub async fn create_link(
    url: &Url,
    generator: &Sqids,
    pool: &PgPool,
    user_id: Option<UserId>,
    password: Option<&str>,
    hasher: &Argon2<'_>,
) -> Result<(i64, String), ServiceError> {
    let mut tx = pool.begin().await?;
    let out = create_link_tx(generator, Some(url), &mut tx, user_id, password, hasher).await?;
    tx.commit().await?;
    Ok(out)
}

/// Create a link with user-defined alias for the provided URL
///
/// Returns Ok(false) if the alias is already taken
#[tracing::instrument(
    name = "services::create_link_with_alias",
    skip(pool, password, hasher)
)]
pub async fn create_link_with_alias(
    url: &Url,
    alias: &LinkAlias,
    pool: &PgPool,
    user_id: Option<UserId>,
    password: Option<&str>,
    hasher: &Argon2<'_>,
) -> Result<(i64, String), ServiceError> {
    let mut tx = pool.begin().await?;
    let out =
        create_link_with_alias_tx(alias, Some(url), &mut tx, user_id, password, hasher).await?;
    tx.commit().await?;
    Ok(out)
}

pub async fn create_collection(
    urls: Vec<Url>,
    alias: Option<&LinkAlias>,
    password: Option<&LinkPassword>,
    generator: &Sqids,
    hasher: &Argon2<'_>,
    pool: &PgPool,
) -> Result<(i64, String), ServiceError> {
    if urls.is_empty() {
        return Err(CollectionError::Empty.into());
    }

    if urls.len() as i32 > MAX_COLLECTION_ITEMS {
        return Err(CollectionError::LimitReached.into());
    }

    let mut tx = pool.begin().await?;

    let password = password.map(|v| v.as_str());
    let (link_id, alias) = match alias {
        Some(link_alias) => {
            create_link_with_alias_tx(link_alias, None, &mut tx, None, password, hasher).await?
        }
        None => create_link_tx(generator, None, &mut tx, None, password, hasher).await?,
    };

    let positions: Vec<i32> = (0..urls.len() as i32).collect();
    let target_urls: Vec<String> = urls.iter().map(|u| u.clone().into_string()).collect();

    sqlx::query!(
        r#"
        INSERT INTO collection_items (link_id, position, target_url, title)
        SELECT $1, t.position, t.target_url, NULL
        FROM UNNEST($2::int[], $3::text[]) AS t(position, target_url)
        "#,
        link_id,
        &positions,
        &target_urls
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((link_id, alias))
}

pub async fn add_url_to_collection(
    alias: &LinkAlias,
    url: &Url,
    title: Option<&str>,
    pool: &PgPool,
) -> Result<(), ServiceError> {
    let mut tx = pool.begin().await?;

    let parent = sqlx::query!(
        r#"
        SELECT id, kind, target_url
        FROM links
        WHERE alias = $1
        FOR UPDATE
        "#,
        alias.as_str(),
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(LinkError::NotFound)?;

    match parent.kind.as_str() {
        "collection" => {
            let count = sqlx::query_scalar!(
                r#"
                SELECT COUNT(*)::INT AS "count!"
                FROM collection_items
                WHERE link_id = $1
                "#,
                parent.id,
            )
            .fetch_one(&mut *tx)
            .await?;

            if count >= MAX_COLLECTION_ITEMS {
                return Err(CollectionError::LimitReached.into());
            }

            let next_position = sqlx::query_scalar!(
                r#"
                SELECT COALESCE(MAX(position) + 1, 0)::INT AS "next_position!"
                FROM collection_items
                WHERE link_id = $1
                "#,
                parent.id,
            )
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO collection_items (link_id, position, target_url, title)
                VALUES ($1, $2, $3, $4)
                "#,
                parent.id,
                next_position,
                url.as_str(),
                title,
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                r#"
                UPDATE links
                SET updated_at = now()
                WHERE id = $1
                "#,
                parent.id,
            )
            .execute(&mut *tx)
            .await?;
        }
        "redirect" => return Err(anyhow!("this is not a collection").into()),
        _ => {
            return Err(anyhow!("DB contained unexpected value for record kind").into());
        }
    }

    tx.commit().await?;

    Ok(())
}

/// Remove user's link
#[tracing::instrument(name = "services::remove_user_link", skip(pool))]
pub async fn delete_link_for_user(
    user_id: &UserId,
    alias: &LinkAlias,
    pool: &PgPool,
) -> Result<(), ServiceError> {
    sqlx::query!(
        r#"
        DELETE FROM links
        WHERE user_id = $1
          AND alias = $2
        "#,
        user_id,
        alias.as_str()
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn convert_to_collection(alias: &LinkAlias, pool: &PgPool) -> Result<(), ServiceError> {
    let mut tx = pool.begin().await?;

    let parent = sqlx::query!(
        r#"
        SELECT id, kind, target_url
        FROM links
        WHERE alias = $1
        FOR UPDATE
        "#,
        alias.as_str(),
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(LinkError::NotFound)?;

    match parent.kind.as_str() {
        "redirect" => {
            let target_url = parent
                .target_url
                .context("Expected redirect record to contain target URL")?;

            sqlx::query!(
                r#"
                INSERT INTO collection_items (link_id, position, target_url, title)
                VALUES ($1, 0, $2, NULL)
                "#,
                parent.id,
                target_url,
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                r#"
                UPDATE links
                SET kind = 'collection',
                    target_url = NULL,
                    updated_at = now()
                WHERE id = $1
                "#,
                parent.id,
            )
            .execute(&mut *tx)
            .await?;
        }

        "collection" => {
            return Err(anyhow!("Already a collection").into());
        }
        _ => {
            return Err(anyhow!("DB contained unexpected value for record kind").into());
        }
    }

    tx.commit().await?;

    Ok(())
}

async fn create_link_tx(
    generator: &Sqids,
    target_url: Option<&str>,
    tx: &mut Transaction<'_, Postgres>,
    user_id: Option<UserId>,
    password: Option<&str>,
    hasher: &Argon2<'_>,
) -> Result<(i64, String), ServiceError> {
    let password_hash = password
        .filter(|p| !p.is_empty())
        .map(|p| hash_password(p, hasher))
        .transpose()?;
    let password_hash_ref = password_hash.as_deref();

    // reserve unique ID
    let seq = sqlx::query!(
        r#"
        SELECT nextval(pg_get_serial_sequence('links', 'id')) AS "id!"
        "#
    )
    .fetch_one(&mut **tx)
    .await?;

    let id = seq.id;

    let alias = generator
        .encode(&[id as u64])
        .context("Sqids alphabet was exhausted")?;

    let rec = sqlx::query!(
        r#"
        INSERT INTO links (id, alias, target_url, user_id, password_hash)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, alias
        "#,
        id,
        alias,
        target_url,
        user_id,
        password_hash_ref,
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok((rec.id, rec.alias))
}

async fn create_link_with_alias_tx(
    alias: &str,
    target_url: Option<&str>,
    tx: &mut Transaction<'_, Postgres>,
    user_id: Option<UserId>,
    password: Option<&str>,
    hasher: &Argon2<'_>,
) -> Result<(i64, String), ServiceError> {
    let password_hash = password
        .filter(|p| !p.is_empty())
        .map(|p| hash_password(p, hasher))
        .transpose()?;
    let password_hash_ref = password_hash.as_deref();

    let rec_opt = sqlx::query!(
        r#"
        INSERT INTO links (alias, target_url, user_id, password_hash)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (alias) DO NOTHING
        RETURNING alias, id
        "#,
        alias,
        target_url,
        user_id,
        password_hash_ref,
    )
    .fetch_optional(&mut **tx)
    .await?;

    rec_opt
        .map(|rec| (rec.id, rec.alias))
        .ok_or(LinkError::AlreadyExists.into())
}
