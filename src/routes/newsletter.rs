use actix_web::{HttpResponse, ResponseError, web};
use actix_web::http::StatusCode;
use anyhow::Context;
use sqlx::PgPool;

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;


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
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
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

pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
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