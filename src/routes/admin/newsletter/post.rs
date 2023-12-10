use actix_web::{HttpResponse, ResponseError, web};
use actix_web::http::StatusCode;
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::authentication::UserId;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;
use crate::utils::{e500, see_other};

#[derive(serde::Deserialize)]
pub struct FormData {
    html_content: String,
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
    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client.send_email(
                    &subscriber.email,
                    &form.title,
                    &form.html_content,
                    &form.text_content,
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
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid.",
                )
            }
        }
    }

    FlashMessage::info("The newsletter issue has been published.").send();
    Ok(see_other("/admin/newsletter"))
}
