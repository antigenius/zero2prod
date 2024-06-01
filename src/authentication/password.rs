use argon2::{
    Algorithm,
    Argon2,
    Params,
    PasswordHash,
    PasswordHasher,
    PasswordVerifier,
    Version, password_hash::SaltString
};
use anyhow::Context;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::telemetry::spawn_blocking_with_tracing;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub password: Secret<String>,
    pub username: String,
}

#[tracing::instrument(
    name = "Get stored credentials.",
    skip(username, pool)
)]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
            SELECT
                id,
                password_hash

            FROM
                person

            WHERE
                username = $1
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to execute query to retreive stored credentials.")?
    .map(|row| (row.id, Secret::new(row.password_hash)));

    Ok(row)
}

pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string()
    );

    if let Some((stored_user_id, stored_hash)) = get_stored_credentials(
        &credentials.username,
        pool
    )
    .await?
    {
        user_id = Some(stored_user_id);
        expected_hash = stored_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(
            expected_hash,
            credentials.password
        )
    })
    .await
    .context("Failed to spawn blocking task to verify password.")??;
    
    user_id
        .ok_or_else(|| anyhow::anyhow!("Unkown username."))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_hash, password_candidate)
)]
fn verify_password_hash(
    expected_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_hash = PasswordHash::new(expected_hash.expose_secret())
        .context("Failed to parse PHC format hash string.")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_hash
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(
    name = "Change password",
    skip(password, pool)
)]
pub async fn change_password(
    user_id: uuid::Uuid,
    password: Secret<String>,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let hash = spawn_blocking_with_tracing(
        move || compute_password_hash(password)
    )
    .await?
    .context("Failed to hash password.")?;

    sqlx::query!(
        r#"
            UPDATE person
            SET password_hash = $1
            WHERE id = $2
        "#,
        hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to save password change to database.")?;

    Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();
    Ok(Secret::new(hash))
}
