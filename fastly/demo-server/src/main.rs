use fastly::{Request, Response};
use fastly_async::http::PendingResponse;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::instrument;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

fn init_tracer() -> SdkTracerProvider {
    let exporter = SpanExporter::builder()
        .with_http()
        .with_http_client(fastly_opentelemetry::FastlyHttpClient)
        .with_endpoint("http://otel-collector.local:4318/v1/traces")
        .build()
        .expect("failed to build span exporter");

    SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .build()
}

#[fastly_async::main]
async fn main(req: Request) -> Response {
    let provider = init_tracer();
    let tracer = provider.tracer("fastly-demo-server");

    tracing_subscriber::registry()
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    let response = handle(req).await;

    provider
        .shutdown()
        .expect("failed to shut down tracer provider");

    response
}

#[instrument]
async fn handle(_req: Request) -> Response {
    let req1 = PendingResponse::from(
        Request::get("https://httpbin.org/get")
            .send_async("httpbin")
            .expect("send failed"),
    );
    let req2 = PendingResponse::from(
        Request::get("https://httpbin.org/status/418")
            .send_async("httpbin")
            .expect("send failed"),
    );
    let req3 = PendingResponse::from(
        Request::get("https://httpbin.org/delay/1")
            .send_async("httpbin")
            .expect("send failed"),
    );

    let results = futures::future::try_join_all([req1, req2, req3])
        .await
        .expect("requests failed");
    let statuses = results
        .iter()
        .map(|res| res.get_status())
        .collect::<Vec<_>>();
    let statuses_str = statuses
        .iter()
        .map(|status| status.as_u16().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Response::from_status(200).with_body_text_plain(&format!("Backend responses: {statuses_str}"))
}
