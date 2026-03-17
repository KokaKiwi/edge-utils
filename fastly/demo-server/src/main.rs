use fastly::{Request, Response};

#[fastly_async::main]
async fn main(_req: Request) -> Response {
    Response::from_status(200).with_body_text_plain("Hello from Fastly!\n")
}
