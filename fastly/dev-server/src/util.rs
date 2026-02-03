use std::fmt;
use std::time::Duration;

use opentelemetry_semantic_conventions::attribute as otel;
use serde::{de::DeserializeOwned, ser::Serialize};
use tower_http::trace;
use tracing::field;

pub async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[derive(Debug, Clone, Copy)]
pub struct OtelTrace;

impl<B> trace::MakeSpan<B> for OtelTrace {
    fn make_span(&mut self, request: &http::Request<B>) -> tracing::Span {
        use axum::extract::MatchedPath;

        let path = if let Some(matched_path) = request.extensions().get::<MatchedPath>() {
            matched_path.as_str()
        } else {
            request.uri().path()
        };

        tracing::debug_span!(
            "request",
            otel.name = format!("{} {path}", request.method()),
            otel.kind = "server",
            { otel::HTTP_REQUEST_METHOD } = %request.method(),
            { otel::HTTP_ROUTE } = request.uri().path(),
            { otel::URL_FULL } = %request.uri(),
            { otel::NETWORK_PROTOCOL_NAME } = "http",
            { otel::NETWORK_PROTOCOL_VERSION } = ?request.version(),
            { otel::OTEL_STATUS_CODE } = field::Empty,
            { otel::HTTP_RESPONSE_STATUS_CODE } = field::Empty,
        )
    }
}

impl<B> trace::OnResponse<B> for OtelTrace {
    fn on_response(self, response: &http::Response<B>, _latency: Duration, span: &tracing::Span) {
        let code = if response.status().is_success() {
            "OK"
        } else {
            "ERROR"
        };

        span.record(otel::OTEL_STATUS_CODE, code);
        span.record(otel::HTTP_RESPONSE_STATUS_CODE, response.status().as_u16());
    }
}

impl<B> trace::OnFailure<B> for OtelTrace {
    fn on_failure(&mut self, _failure_classification: B, _latency: Duration, span: &tracing::Span) {
        span.record(otel::OTEL_STATUS_CODE, "ERROR");
    }
}

pub struct JsonRecord<T>(pub T);

impl<T> redb::Value for JsonRecord<T>
where
    T: fmt::Debug + Serialize + DeserializeOwned,
{
    type SelfType<'a>
        = JsonRecord<T>
    where
        T: 'a;

    type AsBytes<'a>
        = String
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let value = serde_json::from_slice(data).expect("failed to deserialize json value");
        JsonRecord(value)
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        serde_json::to_string(&value.0).expect("failed to serialize json value")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Json")
    }
}

impl<T: fmt::Debug> fmt::Debug for JsonRecord<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Json").field(&self.0).finish()
    }
}
