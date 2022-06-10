use sqlx::postgres::PgPoolOptions;
use zero2prod::email_client::EmailClient;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber(
        "zero2prod".to_string(),
        "info".to_string(),
        std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration file");
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
