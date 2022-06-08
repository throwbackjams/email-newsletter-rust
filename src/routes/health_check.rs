use actix_web::HttpResponse;
use tracing::info;

pub async fn health_check() -> HttpResponse {
    info!("Health check request received");
    HttpResponse::Ok().finish()
}
