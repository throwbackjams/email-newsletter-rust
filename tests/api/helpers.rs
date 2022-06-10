use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::configuration::get_configuration;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use zero2prod::{configuration::DatabaseSettings, startup::run};
use zero2prod::email_client::EmailClient;

//Ensurce that tracing stack is only initialized once
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "debug".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(
            subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(
            subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
    //TRACING only executed once
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration =
        get_configuration().expect("Failed to read configuration within spwan_app");
    configuration.database.database_name = Uuid::new_v4().to_string();

    let response_timeout = configuration.email_client.timeout();

    let email_client = EmailClient::new(
        configuration.email_client.base_url.clone(), 
        configuration.email_client.sender().expect("Failed to parse configuration email client"),
        configuration.email_client.authorization_token,
        response_timeout
    );

    let connection_pool = configure_database(&configuration.database).await;

    let server = run(listener, connection_pool.clone(), email_client).expect("Failed to run server");
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres in configure_database");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database in configure_database");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres in configure_database");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate database in configure_database");

    connection_pool
}
