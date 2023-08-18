use actix_web::{HttpRequest, HttpResponse, ResponseError, web};
use actix_web::http::{header, StatusCode};
use actix_web::http::header::{HeaderMap, HeaderValue};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use anyhow::Context;
use base64::Engine;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;
use crate::telemetry::spawn_blocking_with_tracing;


#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(serde::Deserialize)]
pub struct BodyData {
    content: Content,
    title: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentications failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#)
                    .unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);

                response
            },
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            },
        }
    }
}

struct Credentials {
    password: Secret<String>,
    username: String,
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header is missing.")?
        .to_str()
        .context("The 'Authorization' header is not a valid UTF8 string.")?;
    dbg!(header_value);
    let b64_credentials = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme is no 'Basic'")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(b64_credentials)
        .context("Failed ot base64-decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credentials string is not valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ":");
    let username = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!("A username must be provided in 'Basic' auth.")
        })?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!("A password must be provided in 'Basic' auth.")
        })?
        .to_string();

    Ok(Credentials {
        password: Secret::new(password),
        username,
    })
}

async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let mut user_id = None;
    let mut expected_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string()
    );

    if let Some((stored_user_id, stored_hash)) = get_stored_credentials(
        &credentials.username,
        &pool
    )
    .await
    .map_err(PublishError::UnexpectedError)?
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
    .context("Failed to spawn blocking task to verify password.")
    .map_err(PublishError::UnexpectedError)??;
    
    user_id.ok_or_else(||
        PublishError::AuthError(anyhow::anyhow!("Unkown username."))
    )
}

#[tracing::instrument(
    name = "Get conrirmed subscribers",
    skip(pool)
)]
async fn get_confirmed_subscribers(
    pool: &PgPool
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#,)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(e) => Err(anyhow::anyhow!(e)),
    })
    .collect();

    Ok(confirmed_subscribers)
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client, request)
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers())
        .map_err(PublishError::AuthError)?;
    tracing::Span::current().record(
        "username",
        &tracing::field::display(&credentials.username)
    );
    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("userid", &tracing::field::display(&user_id));
    let subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client.send_email(
                    &subscriber.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                )
                .await
                .with_context(|| {
                    format!("Failed to send newsletter to {}", subscriber.email)
                })?;
            },
            Err(e) => {
                tracing::warn!(
                    error.cause_chain = ?e,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid.",
                )
            }
        }

    }

    Ok(HttpResponse::Ok().finish())
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

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_hash, password_candidate)
)]
fn verify_password_hash(
    expected_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), PublishError> {
    let expected_hash = PasswordHash::new(&expected_hash.expose_secret())
        .context("Failed to parse PHC format hash string.").
        map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_hash
        )
        .context("Invalid password.")
        .map_err(PublishError::AuthError)
}