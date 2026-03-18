use fastly::{Request, Response};
use fastly_async::http::PendingResponse;
use tracing::instrument;

#[fastly_async::main]
async fn main(req: Request) -> Response {
    handle(req).await
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
