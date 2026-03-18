use bytes::Bytes;
use opentelemetry_http::{HttpClient, HttpError};

#[derive(Debug, Clone)]
pub struct FastlyHttpClient;

#[async_trait::async_trait]
impl HttpClient for FastlyHttpClient {
    async fn send_bytes(
        &self,
        _req: http::Request<Bytes>,
    ) -> Result<http::Response<Bytes>, HttpError> {
        // Implement the logic to send the request using Fastly's HTTP client
        // unimplemented!()
        Ok(http::Response::builder()
            .status(200)
            .body(Bytes::new())
            .unwrap())
    }
}
