use zero2prod::configuration::get_configuration;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    // App
    let config = get_configuration().expect("Failed to read config.");
    let app = Application::build(config).await?;
    app.run_until_stopped().await?;
    Ok(())
}
