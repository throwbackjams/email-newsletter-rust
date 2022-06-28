use actix_web::{web, HttpResponse, http::header::ContentType};
use uuid::Uuid;
use anyhow::Context;
use sqlx::PgPool;
use crate::utils::e500;
use crate::authentication::UserId;

pub async fn admin_dashboard(
    user_id: web::ReqData<UserId>,
    connection_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &connection_pool).await.map_err(e500)?;

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
<p>Available actions:</p>
<ol>
    <li><a href="/admin/password">Change password</a></li>
    <li>
        <form name="logoutForm" action="/admin/logout" method="post">
            <input type="submit" value="Logout">
        </form>
    </li>
</ol>
</body>
</html>"#
    )))
}

#[tracing::instrument(name = "Get username", skip(connection_pool))]
pub async fn get_username(
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