use crate::{routes::{health_check, subscribe}, email_client::EmailClient};
use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;
use crate::configuration::{Settings, DatabaseSettings};

/// A new type to hold the server that is and its port
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
    
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address in settings");
    
        let response_timeout = configuration.email_client.timeout();
        
        let email_client = EmailClient::new(
            configuration.clone().email_client.base_url,
            sender_email,
            configuration.clone().email_client.authorization_token,
            response_timeout,
        );
    
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
    
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, email_client)?;
        Ok(Self { port, server})
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

pub fn run(
    listener: TcpListener, 
    connection_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let connection_pool = web::Data::new(connection_pool);
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscribe", web::post().to(subscribe))
            .app_data(connection_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
