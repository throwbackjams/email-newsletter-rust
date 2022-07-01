use crate::{utils::e500, idempotency};
use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn submit_newsletter_to_send_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let idempotency_key = uuid::Uuid::new_v4();
    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).map_err(e500)?;
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Send a Newsletter</title>
    </head>
    <body>
        {msg_html}
        <form action="/admin/newsletters" method="post">
            <label>Title:<br>
                <input
                    type="text"
                    placeholder="Enter newsletter title"
                    name="title"
                >
            </label>
            <br>
            <label>HTML Content: <br>
                <textarea
                    placeholder="Enter content in HTML format"
                    name="html_content"
                    rows="20"
                    cols="50"
                ></textarea>
            </label>
            <br>
            <label>Text Content: <br>
                <textarea
                    placeholder="Enter content in plain text"
                    name="text_content"
                    rows="20"
                    cols="50"
                ></textarea>
            </label>
            <br>
            <input hidden type="text" name="idempotency_key" value="{idempotency_key}">
            <button type="submit">Send Newsletter</button>
        </form>
        <p><a href="/admin/dashboard">&lt;- Back</a></p>
    </body>
</html>"#,
        )))
}
