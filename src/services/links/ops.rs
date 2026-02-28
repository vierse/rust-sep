use anyhow::{Context, anyhow};
use argon2::Argon2;
use sqids::Sqids;
use sqlx::{PgPool, Postgres, Transaction};

use crate::{
    domain::{LinkId, UserId},
    services::{CollectionError, LinkError, MAX_COLLECTION_ITEMS, ServiceError, hash_password},
};

pub async fn create_link(
    url: &str,
    generator: &Sqids,
    pool: &PgPool,
    user_id: Option<UserId>,
    password: Option<&str>,
    hasher: &Argon2<'_>,
) -> Result<(LinkId, String), ServiceError> {
    let mut tx = pool.begin().await?;
    let out = create_link_tx(generator, Some(url), &mut tx, user_id, password, hasher).await?;
    tx.commit().await?;
    Ok(out)
}

pub async fn create_link_with_alias(
    url: &str,
    alias: &str,
    pool: &PgPool,
    user_id: Option<UserId>,
    password: Option<&str>,
    hasher: &Argon2<'_>,
) -> Result<(LinkId, String), ServiceError> {
    let mut tx = pool.begin().await?;
    let out =
        create_link_with_alias_tx(alias, Some(url), &mut tx, user_id, password, hasher).await?;
    tx.commit().await?;
    Ok(out)
}

pub async fn create_collection(
    urls: Vec<String>,
    alias: Option<&str>,
    password: Option<&str>,
    generator: &Sqids,
    hasher: &Argon2<'_>,
    pool: &PgPool,
) -> Result<(LinkId, String), ServiceError> {
    if urls.is_empty() {
        return Err(CollectionError::Empty.into());
    }

    if urls.len() as i32 > MAX_COLLECTION_ITEMS {
        return Err(CollectionError::LimitReached.into());
    }

    let mut tx = pool.begin().await?;

    let (link_id, alias) = match alias {
        Some(link_alias) => {
            create_link_with_alias_tx(link_alias, None, &mut tx, None, password, hasher).await?
        }
        None => create_link_tx(generator, None, &mut tx, None, password, hasher).await?,
    };

    let positions: Vec<i32> = (0..urls.len() as i32).collect();
    let target_urls: Vec<String> = urls.iter().map(|u| u.to_string()).collect();

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
    alias: &str,
    url: &str,
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
        alias,
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
                url,
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
        _ => {
            return Err(anyhow!("DB contained unexpected value for record kind").into());
        }
    }

    tx.commit().await?;

    Ok(())
}

pub async fn delete_link_for_user(
    user_id: UserId,
    alias: &str,
    pool: &PgPool,
) -> Result<(), ServiceError> {
    let rec = sqlx::query!(
        r#"
        DELETE FROM links
        WHERE user_id = $1
          AND alias = $2
        "#,
        user_id,
        alias
    )
    .execute(pool)
    .await?;

    if rec.rows_affected() == 0 {
        return Err(LinkError::NotFound.into());
    }

    Ok(())
}

pub async fn convert_to_collection(alias: &str, pool: &PgPool) -> Result<(), ServiceError> {
    let mut tx = pool.begin().await?;

    let parent = sqlx::query!(
        r#"
        SELECT id, kind, target_url
        FROM links
        WHERE alias = $1
        FOR UPDATE
        "#,
        alias,
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
            return Err(CollectionError::AlreadyCollection.into());
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
) -> Result<(LinkId, String), ServiceError> {
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
        .map_err(|_| LinkError::GeneratorError)?;

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
) -> Result<(LinkId, String), ServiceError> {
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
