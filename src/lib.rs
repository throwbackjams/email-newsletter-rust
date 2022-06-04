use actix_web::{ web, App, HttpsRequest, HttpServer, Responder};

async fn greet(request: HttpsRequest) -> impl Responder {
    let name = request.match_info().get("name").unwrap_or("World");
    format!("Hello, {}!", name)
}

async fn health_check() -> HttpResponse {
    HttpResponse::Ok()
}

pub async fn main() -> std::io::Result<()>{

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(greet))
            .route("/{name}", web::get().to(greet))
            .route("/health_check", web::get().to(health_check))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}