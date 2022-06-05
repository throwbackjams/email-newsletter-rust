use actix_web::{ web, App, HttpRequest, HttpResponse, HttpServer, Responder, dev::Server};
use std::net::TcpListener;

// async fn greet(request: HttpRequest) -> impl Responder {
//     let name = request.match_info().get("name").unwrap_or("World");
//     format!("Hello, {}!", name)
// }

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[derive(serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}

async fn subscribe(_form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {

    let server = HttpServer::new(|| {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscribe", web::post().to(subscribe))
    })
    .listen(listener)?
    .run();

    Ok(server)
    
}