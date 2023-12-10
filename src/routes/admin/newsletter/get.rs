use std::fmt::Write;

use actix_web::{HttpResponse, web};
use actix_web::http::header::ContentType;
use actix_web_flash_messages::IncomingFlashMessages;

use crate::authentication::UserId;


pub async fn publish_newsletter_form(
    flash_messages: IncomingFlashMessages,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let _ = user_id.into_inner();
    let mut error_html = String::new();

    for m in flash_messages.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }
    
    Ok(HttpResponse::Ok()
    .content_type(ContentType::html())
    .body(format!(r#"<!DOCTYPE html>
        <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Publish Newsletter</title>
            </head>
            <body>
                {error_html}
                <form action="/admin/newsletter" method="post">
                    <label>Title
                        <input
                            type="text"
                            placeholder="Newsletter Title"
                            name="title"
                        >
                    </label>
                    <br />
                    <label>HTML Content
                        <input
                            type="textarea"
                            placeholder="<p>Content</p>"
                            name="html_content"
                        >
                    </label>
                    <br />
                    <label>Text Content
                        <input
                            type="textarea"
                            placeholder="Content"
                            name="text_content"
                        >
                    </label>
                    <button type="submit">Publish</button>
                </form>
                <p><a href="/admin/dashboard">&lt;- Back</a></p>
            </body>
        </html>"#,
    )))
}
