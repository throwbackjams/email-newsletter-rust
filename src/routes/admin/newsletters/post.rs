use crate::authentication::UserId;
use crate::routes::error_chain_fmt;
use crate::{domain::SubscriberEmail, utils::see_other};
use actix_web::web::ReqData;
use actix_web::{web, HttpResponse, ResponseError};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use reqwest::StatusCode;
use sqlx::PgPool;
use crate::idempotency::{IdempotencyKey, get_saved_response, save_response, try_processing, NextAction};
use crate::utils::{e400, e500};

use crate::email_client::EmailClient;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
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
    user_id: ReqData<UserId>,
) -> Result<HttpResponse, PublishError> {
    let FormData { title, html_content, text_content, idempotency_key } = form.0;
    let user_id = user_id.into_inner();
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let transaction = match try_processing(&connection_pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };

    // if let Some(saved_response) = get_saved_response(&connection_pool, &idempotency_key, *user_id)
    //     .await
    //     .map_err(e500)? 
    // {
    //     return Ok(saved_response)
    // }

    // Get confirmed subscribers
    let confirmed_subscribers = get_confirmed_subscribers(&connection_pool).await?;
    // Send the newsletter to each confirmed subscriber
    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &title, &html_content, &text_content)
                    .await
                    .with_context(|| {
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
    success_message().send();
    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
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
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(confirmed_subscribers)
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error(transparent)]
    IdempotencyKeyError(#[from] actix_web::Error),
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
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            },
            PublishError::IdempotencyKeyError(_) => {
                HttpResponse::new(StatusCode::BAD_REQUEST)
            }
        }
    }
}

fn success_message() -> FlashMessage {
    FlashMessage::info("Your newsletter has been successfully sent.")
}
