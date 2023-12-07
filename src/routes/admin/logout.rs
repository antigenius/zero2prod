use actix_web::{HttpResponse, web};
use actix_web_flash_messages::FlashMessage;

use crate::authentication::UserId;
use crate::session_state::TypedSession;
use crate::utils::see_other;


pub async fn logout(
    session: TypedSession,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let _ = user_id.into_inner();
    session.logout();
    FlashMessage::info("You have successfully logged out.").send();
    Ok(see_other("/login"))
}
