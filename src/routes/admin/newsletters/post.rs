use actix_web::web::ReqData;
use actix_web::{web, HttpResponse, HttpRequest, ResponseError};
use actix_web_flash_messages::FlashMessage;
use reqwest::{StatusCode, header};
use actix_web::http::header::{HeaderMap, HeaderValue};
use sqlx::PgPool;
use crate::authentication::UserId;
use crate::{domain::SubscriberEmail, utils::see_other};
use crate::routes::error_chain_fmt;
use anyhow::Context;

use crate::email_client::{self, EmailClient};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
}

#[tracing::instrument(
    name="Send newsletter",
    skip(form, connection_pool, email_client)
    fields(user_id=%*user_id)
)]
pub async fn send_newsletter(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: ReqData<UserId>
) -> Result<HttpResponse, PublishError>{
    // Get confirmed subscribers and newsletter components from FormData
    let confirmed_subscribers = get_confirmed_subscribers(&connection_pool).await?;
    let title = form.0.title;
    let html_content = form.0.html_content;
    let text_content = form.0.text_content;

    // Send the newsletter to each confirmed subscriber
    for subscriber in confirmed_subscribers {
        
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &title,
                        &html_content,
                        &text_content,
                    )
                    .await
                    .with_context(||{
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored email address is invalid"
                );
            }
        }
    }

    // After successfully sending a newsletter, redirect to the send newsletter form with a successful message
    FlashMessage::info("Your newsletter has been successfully sent.").send();
    Ok(see_other("/admin/newsletters"))
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(connection_pool))]
async fn get_confirmed_subscribers(
    connection_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(connection_pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber {email}),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(confirmed_subscribers)
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
    // Implement error_response instead of status_code to insert a custom header
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}