use fastly::{Request, Response};
use fastly_async::http::PendingResponse;

#[fastly_async::main]
async fn main(_req: Request) -> Response {
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
        Request::get("https://httpbin.org/delay/5")
            .send_async("httpbin")
            .expect("send failed"),
    );

    let (res1, res2, res3) = futures::try_join!(req1, req2, req3).expect("request failed");

    let status1 = res1.get_status();
    let status2 = res2.get_status();
    let status3 = res3.get_status();

    Response::from_status(200).with_body_text_plain(&format!(
        "Backend responses: {status1}, {status2}, {status3}\n"
    ))
}
