use actix_web::{HttpResponse, web};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::authentication::{AuthError, Credentials, UserId, validate_credentials};
use crate::routes::admin::dashboard::get_username;
use crate::utils::{e500, see_other};


#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error("Password fields must match.").send();
        return Ok(see_other("/admin/password"));
    }

    let password_length = form.new_password
        .expose_secret()
        .chars()
        .count();

    if password_length < 13 || password_length > 127 {
        FlashMessage::error("New password must be between 12 and 128 characters.").send();
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(*user_id, &pool)
        .await
        .map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            },
            AuthError::UnexpectedError(_) => Err(e500(e).into()),
        }
    }
    
    crate::authentication::change_password(
        *user_id,
        form.0.new_password,
        &pool
    )
    .await
    .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
