use std::time::Duration;
use tracing::field::display;
use tracing::Span;

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    configuration::Settings, domain::SubscriberEmail, email_client::EmailClient,
    startup::get_connection_pool,
};

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty
    ),
    err
)]
pub async fn try_execute_task(
    connection_pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    let task = dequeue_task(connection_pool).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }

    let (transaction, issue_id, email) = task.unwrap();

    Span::current()
        .record("newsletter_issue_id", &display(issue_id))
        .record("subscriber_email", &display(&email));

    match SubscriberEmail::parse(email.clone()) {
        Ok(email) => {
            let issue = get_issue(connection_pool, issue_id).await?;
            match email_client
                .send_email(
                    &email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
            {
                Ok(_) => {
                    // Only delete the task if send_email was successful
                    delete_task(transaction, issue_id, &email.to_string()).await?
                }
                Err(e) => {
                    tracing::error!(
                        error.cause_chain = ?e,
                        error.message = %e,
                        "Failed to deliver issue to a confirmed subscriber. Skipping"
                    );
                }
            }
        }
        Err(e) => {
            tracing::error!(
            error.cause_chain = ?e,
            error.message = %3,
            "Skipping a confirmed subscriber. The stored email address is invalid"
            );
        }
    }
    Ok(ExecutionOutcome::TaskCompleted)
}

type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    connection_pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error> {
    let mut transaction = connection_pool.begin().await?;

    // Lock the row with FOR UPDATE and skip locked rows with SKIP LOCKED
    let r = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#
    )
    .fetch_optional(&mut transaction)
    .await?;
    if let Some(r) = r {
        Ok(Some((
            transaction,
            r.newsletter_issue_id,
            r.subscriber_email,
        )))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1 AND
            subscriber_email = $2
        "#,
        issue_id,
        email
    )
    .execute(&mut transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(
    connection_pool: &PgPool,
    issue_id: Uuid,
) -> Result<NewsletterIssue, anyhow::Error> {
    let r = sqlx::query!(
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE newsletter_issue_id = $1
        "#,
        issue_id
    )
    .fetch_one(connection_pool)
    .await?;

    let issue = NewsletterIssue {
        title: r.title,
        text_content: r.text_content,
        html_content: r.html_content,
    };
    Ok(issue)
}

async fn worker_loop(
    connection_pool: PgPool,
    email_client: EmailClient,
) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(&connection_pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    let email_client = configuration.email_client.client();
    worker_loop(connection_pool, email_client).await
}
