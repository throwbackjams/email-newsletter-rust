use sqlx::postgres::PgPoolOptions;
use zero2prod::email_client::{self, EmailClient};
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".to_string(), "info".to_string(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration file");
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );

    let email_client = EmailClient::new(
        configuration.email_client.base_url.clone(), 
        configuration.email_client.sender().expect("Failed to parse configuration email client"),
        configuration.email_client.authorization_token
    );

    let connection_pool = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());

    let listener = TcpListener::bind(address)?;

    run(listener, connection_pool, email_client)?.await
}
