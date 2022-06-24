use actix_web::{web, HttpResponse, http::header::{ContentType, LOCATION}};
use uuid::Uuid;
use anyhow::Context;
use sqlx::PgPool;
use crate::session_state::TypedSession;

// Return an opaque 500 while preserving an error's root cause for logging
fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static
{
    actix_web::error::ErrorInternalServerError(e)
}

pub async fn admin_dashboard(
    session: TypedSession,
    connection_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session
        .get_user_id() // Deserialize into a Uuid type
        .map_err(e500)?
    {
        get_username(user_id, &connection_pool).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta http-equiv="content-type" content="text/html; charset=utf-8">
<title>Admin dashboard</title>
</head>
<body>
<p>Welcome {username}!</p>
</body>
</html>"#
    )))
}

#[tracing::instrument(name = "Get username", skip(connection_pool))]
async fn get_username(
    user_id: Uuid,
    connection_pool: &PgPool,
) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(connection_pool)
    .await
    .context("Failed to perform a query to retrievew a username.")?;
    
    Ok(row.username)
}