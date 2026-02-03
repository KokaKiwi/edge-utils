use opentelemetry_sdk::trace::SdkTracerProvider;

pub struct TraceGuard {
    tracer_provider: SdkTracerProvider,
}

impl Drop for TraceGuard {
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            eprintln!("Failed to shutdown tracer provider: {err}");
        }
    }
}

#[must_use]
pub fn setup_tracing() -> TraceGuard {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{EnvFilter, filter::LevelFilter, fmt};

    let fmt_layer = fmt::layer()
        .without_time()
        .with_target(false)
        .with_writer(std::io::stderr);

    let (otel_layer, tracer_provider) = {
        use opentelemetry::trace::TracerProvider;
        use tracing_subscriber::filter::{EnvFilter, LevelFilter};

        let tracer_provider = setup_otel_tracer_provider();

        let tracer = tracer_provider.tracer("fastly-dev-server");
        let layer = tracing_opentelemetry::layer().with_tracer(tracer);

        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .parse("debug,fastly_dev_server=trace")
            .unwrap();
        let layer = layer.with_filter(filter);

        (layer, tracer_provider)
    };

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let fmt_layer = fmt_layer.with_filter(filter);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    TraceGuard { tracer_provider }
}

fn setup_otel_tracer_provider() -> SdkTracerProvider {
    use opentelemetry::{KeyValue, global};
    use opentelemetry_otlp::SpanExporter;
    use opentelemetry_sdk::{Resource, propagation::TraceContextPropagator};
    use opentelemetry_semantic_conventions::{SCHEMA_URL, attribute::SERVICE_VERSION};

    global::set_text_map_propagator(TraceContextPropagator::new());

    let resource = Resource::builder()
        .with_service_name("fastly-dev-server")
        .with_schema_url(
            [KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))],
            SCHEMA_URL,
        )
        .build();

    let exporter = SpanExporter::builder()
        .with_http()
        .build()
        .expect("Failed to build OTLP span exporter");

    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build()
}
