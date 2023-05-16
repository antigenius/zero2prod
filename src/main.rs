use std::net::TcpListener;

use secrecy::ExposeSecret;
use sqlx::PgPool;

use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Logging
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    
    // App Config
    let config = get_configuration().expect("Failed to read config.");
    let connection = PgPool::connect(&config.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection)?.await
}
