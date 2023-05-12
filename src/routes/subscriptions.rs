use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let request_id = Uuid::new_v4();
    log::info(
        "request {} - Adding '{} '{}' as a new subscriber.",
        request_id,
        form.email,
        form.name,
    );
    log::info!(
        "request {} - Saving new subscriber details in database.",
        request_id
    );
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions
        (id, email, name, subscribed_at)
        VALUES
        ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.get_ref())
    .await
    {
        Ok(_) => {
            log::info!(
                "request {} - New subscriber details have been saved",
                request_id
            );
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            log::error!("request {} - Failed to execute quey: {:?}", request_id, e);
            HttpResponse::InsufficientStorage().finish()
        }
    }
}
