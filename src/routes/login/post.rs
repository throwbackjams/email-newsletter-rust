use crate::authentication::{validate_credentials, Credentials, AuthError};
use actix_web::{web, HttpResponse, ResponseError};
use actix_web::http::header::LOCATION;
use actix_web::http::StatusCode;
use actix_web_flash_messages::FlashMessage;
use secrecy::Secret;
use sqlx::PgPool;
use crate::routes::error_chain_fmt;
use actix_web::error::InternalError;
use crate::session_state::TypedSession;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, connection_pool, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty) //TODO! Understand what this does
)]
pub async fn login(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {

    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));
    
    match validate_credentials(credentials, &connection_pool).await 
        {
            Ok(user_id) => {
                tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
                session.renew(); // Rorate the session token when a user logs in
                session
                    .insert_user_id(user_id) // Create session
                    .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into())))?;
                
                Ok(HttpResponse::SeeOther()
                    .insert_header((LOCATION, "/admin/dashboard"))
                    .finish())

            },
            Err(e) => {
                let e = match e {
                    AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                    AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into())
                };

                FlashMessage::error(e.to_string()).send(); // Creates the cookie, signs it

                let response = HttpResponse::SeeOther()
                    .insert_header((
                        LOCATION,"/login")
                    )
                    .finish();
                
                Err(InternalError::from_response(e, response))
            }
        }
}

// Redirect to login page with an error message
fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();
    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();
    InternalError::from_response(e, response)
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header((
                LOCATION,
                "/login"))
            .finish()
    }

    
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }
}
