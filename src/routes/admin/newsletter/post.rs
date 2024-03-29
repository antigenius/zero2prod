use actix_web::{HttpResponse, ResponseError, web};
use actix_web::http::StatusCode;
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::authentication::UserId;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::idempotency::{get_saved_response, IdempotencyKey, NextAction, save_response, try_processing};
use crate::routes::error_chain_fmt;
use crate::utils::{e400, e500, see_other};

#[derive(serde::Deserialize)]
pub struct FormData {
    html_content: String,
    idempotency_key: String,
    text_content: String,
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
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            },
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

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(form, pool, email_client, user_id)
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let FormData {
        html_content,
        idempotency_key,
        text_content,
        title
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    let txn = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };

    if let Some(saved_response) = get_saved_response(
        &pool,
        &idempotency_key,
        *user_id
    )
    .await
    .map_err(e500)?
    {
        FlashMessage::info("The newsletter issue has been published.").send();
        return Ok(saved_response);
    }
    
    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client.send_email(
                    &subscriber.email,
                    &title,
                    &html_content,
                    &text_content,
                )
                .await
                .with_context(|| {
                    format!("Failed to send newsletter to {}", subscriber.email)
                })
                .map_err(e500)?;
            },
            Err(e) => {
                tracing::warn!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid.",
                )
            }
        }
    }

    success_message().send();
    let response = see_other("/admin/newsletter");
    let response = save_response(txn, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been published.")
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues
        (
            id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        id,
        title,
        text_content,
        html_content
    )
    .execute(transaction)
    .await?;

    Ok(id)
}
