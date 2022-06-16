use crate::email_client::EmailClient;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use actix_web::ResponseError;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::startup::ApplicationBaseUrl;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, connection_pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, actix_web::Error> {
    let new_subscriber: NewSubscriber = match form.0.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return Ok(HttpResponse::BadRequest().finish()),
    };

    let check_existing = match sqlx::query!(
            r#"
            SELECT id, status
            FROM subscriptions 
            WHERE email = $1
            "#,
            new_subscriber.email.as_ref()
        )
        .fetch_optional(connection_pool.get_ref())
        .await
        {
            Ok(option_record) => option_record,
            Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
        };
    
    match check_existing {
        None => (),
        Some(record) => {

            // If the existing subscriber already has a confirmed status, then exit with a success code
            if record.status == "confirmed" {
                return Ok(HttpResponse::Ok().finish());
            }

            let subscription_token = generate_subscription_token();

            let mut transaction = match connection_pool.begin().await {
                Ok(transaction) => transaction,
                Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
            };

            store_token(&mut transaction, record.id, &subscription_token).await?;

            if transaction.commit().await.is_err() {
                return Ok(HttpResponse::InternalServerError().finish());
            }        

            if send_confirmation_email(
                &email_client,
                new_subscriber,
                &base_url.0,
                &subscription_token,
            )
            .await
            .is_err()
            {
                return Ok(HttpResponse::InternalServerError().finish());
            }
        return Ok(HttpResponse::Ok().finish())
        }
    }

    let mut transaction = match connection_pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };

    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };

    let subscription_token = generate_subscription_token();

    store_token(&mut transaction, subscriber_id, &subscription_token).await?;

    if transaction.commit().await.is_err() {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return Ok(HttpResponse::InternalServerError().finish());
    }
    Ok(HttpResponse::Ok().finish())
}

pub fn parse_subscriber(form: FormData) -> Result<NewSubscriber, String> {
    let name = SubscriberName::parse(form.name)?;
    let email = SubscriberEmail::parse(form.email)?;
    Ok(NewSubscriber { name, email })
}

#[tracing::instrument(
    name = "Saving new subscriber in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store a subscription token with subscriber ID in subscription_token table",
    skip(transaction, subscription_token)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        StoreTokenError(e)
    })?;

    Ok(())
}

pub struct StoreTokenError(sqlx::Error);

impl ResponseError for StoreTokenError {}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

// Goes through entire chain of errors and their sources, using the source method of the Error trait
fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscribe/confirm?subscription_token={}",
        base_url, subscription_token
    );

    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter! <br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(new_subscriber.email, "Welcome", &html_body, &plain_body)
        .await
}

/// Generate a random 25-character csae-sensitive subscription token for subscription confirmation
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
