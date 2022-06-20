use actix_web::http::StatusCode;
use actix_web::ResponseError;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Confirm a pending subscriber"
    skip(parameters, connection_pool)
)]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    connection_pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmError> {
    let id = get_subscriber_id_from_token(&connection_pool, &parameters.subscription_token)
        .await
        .context("Failed to retrieve from database subscriber id based on subscription token")?;

    match id {
        None => Err(ConfirmError::SubscriptionIdNotFound),
        Some(subscriber_id) => {
            confirm_subscriber(&connection_pool, subscriber_id)
                .await
                .context("Failed to change confirmation status to confirmed for the subscriber")?;

            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[tracing::instrument(
    name = "Get subscriber_id from token",
    skip(subscription_token, connection_pool)
)]
pub async fn get_subscriber_id_from_token(
    connection_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens where subscription_token= $1"#,
        subscription_token,
    )
    .fetch_optional(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to execute get subscriber id from token query: {:?}",
            e
        );
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(
    name = "Mark subscriber as confirmed"
    skip(subscriber_id, connection_pool)
)]
pub async fn confirm_subscriber(
    connection_pool: &PgPool,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions SET status = 'confirmed' WHERE id = $1
        "#,
        subscriber_id,
    )
    .execute(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute confirm subscriber query: {:?}", e);
        e
    })?;
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum ConfirmError {
    #[error(transparent)]
    DatabaseError(#[from] anyhow::Error),
    #[error("No subscriber found based on the given subscription token link")]
    SubscriptionIdNotFound,
}

impl ResponseError for ConfirmError {
    fn status_code(&self) -> StatusCode {
        match self {
            ConfirmError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ConfirmError::SubscriptionIdNotFound => StatusCode::UNAUTHORIZED,
        }
    }
}
