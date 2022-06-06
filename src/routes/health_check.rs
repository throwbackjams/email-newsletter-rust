use actix_web::{ web, App, HttpRequest, HttpResponse, HttpServer, Responder, dev::Server};
use std::net::TcpListener;

// async fn greet(request: HttpRequest) -> impl Responder {
//     let name = request.match_info().get("name").unwrap_or("World");
//     format!("Hello, {}!", name)
// }

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}