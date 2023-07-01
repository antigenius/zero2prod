use std::net::TcpListener;

use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

use zero2prod::configuration::{get_configuration, DatabaseSettings, EmailBaseUrl};
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};


static TRACING: Lazy<()> = Lazy::new(|| {
    let level = "into".to_string();
    let name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(name, level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(name, level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Unabled to bind random port.");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut config = get_configuration().expect("Failed to read configuration.");
    config.database.database_name = Uuid::new_v4().to_string();
    let pool = configure_database(&config.database).await;

    let sender_email = config.email_client.sender().expect("msnvalid sender email address.");
    let timeout = config.email_client.timeout();
    let email_client = EmailClient::new(
        config.email_client.base_url,
        sender_email,
        config.email_client.auth_token,
        timeout
    );

    let server = run(listener, pool.clone(), email_client)
        .expect("Failed to bind address$");
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations.");

    pool
}
