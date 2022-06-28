use actix_web::{HttpResponse, web};
use actix_web_flash_messages::FlashMessage;
use secrecy::{Secret, ExposeSecret};

use crate::{
    utils::{e500, see_other},
    authentication::{Credentials, validate_credentials, AuthError, UserId}};
use crate::routes::admin::dashboard::get_username;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner(); // Consumes ReqData and returns the inner value
    
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        
        return Ok(see_other("/admin/password"));
    };

    let username = get_username(*user_id, &connection_pool).await.map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.0.current_password.clone(),
    };

    if let Err(e) = validate_credentials(credentials, &connection_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e).into()),
        }
    }

    let new_password_length = form.new_password.expose_secret().len();

    if new_password_length <= 12 || new_password_length >= 256 {
        FlashMessage::error(
            "The new password must be longer than 12 characters and shorter than 128 characters.",
        )
        .send();

        return Ok(see_other("/admin/password"));
    } 
    
    crate::authentication::change_password(*user_id, form.0.new_password, &connection_pool)
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}