use zero2prod::startup::run;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use sqlx::{ PgPool, Executor};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    
    let configuration = get_configuration().expect("Failed to read configuration file");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connet to Postgres");

    let listener = TcpListener::bind(address)?;

    run(listener, connection_pool)?.await
}

