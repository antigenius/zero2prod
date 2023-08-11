use tokio::task::JoinHandle;
use tracing::subscriber::{Subscriber, set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber<Sink>(
    name: String,
    env: String,
    sink: Sink
) -> impl Subscriber + Send + Sync
     where Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static
{
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(env));
    let formatting_layer = BunyanFormattingLayer::new(
        name,
        sink
    );
    Registry::default()
        .with(filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger.");
    set_global_default(subscriber).expect("Failed to set subscriber.");
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}