use std::pin::Pin;
use std::task;

use axum::body::Body as AxumBody;
use http::{Request as HttpRequest, Response as HttpResponse};
use hyper014::{
    Body as HyperBody,
    http::{Request as HyperRequest, Response as HyperResponse},
};
use viceroy_lib::body::Body as ViceroyBody;

pub fn axum_request_to_hyper014<B>(req: HttpRequest<B>) -> HyperRequest<B> {
    let (parts, body) = req.into_parts();

    let mut builder = HyperRequest::builder()
        .method(parts.method.as_str())
        .uri(parts.uri.to_string());

    for (key, value) in parts.headers.iter() {
        builder = builder.header(key.as_str(), value.as_bytes());
    }

    builder.body(body).unwrap()
}

pub fn hyper014_response_to_axum<B>(resp: HyperResponse<B>) -> HttpResponse<B> {
    let (parts, body) = resp.into_parts();

    let mut builder = HttpResponse::builder().status(parts.status.as_u16());

    for (key, value) in parts.headers.iter() {
        builder = builder.header(key.as_str(), value.as_bytes());
    }

    builder.body(body).unwrap()
}

pub fn axum_body_to_hyper014(body: AxumBody) -> HyperBody {
    HyperBody::wrap_stream(body.into_data_stream())
}

pub fn viceroy_body_to_axum(body: ViceroyBody) -> AxumBody {
    AxumBody::new(ViceroyBodyWrapper::new(body))
}

enum ViceroyBodyWrapperState {
    ReadingData,
    ReadingTrailers,
    Done,
}

#[pin_project::pin_project]
struct ViceroyBodyWrapper {
    #[pin]
    body: ViceroyBody,
    state: ViceroyBodyWrapperState,
}

impl ViceroyBodyWrapper {
    fn new(body: ViceroyBody) -> Self {
        Self {
            body,
            state: ViceroyBodyWrapperState::ReadingData,
        }
    }
}

impl http_body::Body for ViceroyBodyWrapper {
    type Data = bytes::Bytes;
    type Error = viceroy_lib::error::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.project();
        let mut body = this.body;

        loop {
            match this.state {
                ViceroyBodyWrapperState::ReadingData => {
                    let chunk = match task::ready!(http_body_04::Body::poll_data(body.as_mut(), cx))
                    {
                        Some(Ok(chunk)) => chunk,
                        Some(Err(e)) => return task::Poll::Ready(Some(Err(e))),
                        None => {
                            *this.state = ViceroyBodyWrapperState::ReadingTrailers;
                            continue;
                        }
                    };

                    return task::Poll::Ready(Some(Ok(http_body::Frame::data(chunk))));
                }
                ViceroyBodyWrapperState::ReadingTrailers => {
                    let trailers =
                        match task::ready!(http_body_04::Body::poll_trailers(body.as_mut(), cx)) {
                            Ok(Some(trailers)) => {
                                let mut headers = http::HeaderMap::new();
                                for (key, value) in trailers.iter() {
                                    let name = match http::HeaderName::from_bytes(key.as_ref()) {
                                        Ok(n) => n,
                                        Err(_) => continue,
                                    };
                                    let value = match http::HeaderValue::from_bytes(value.as_ref())
                                    {
                                        Ok(v) => v,
                                        Err(_) => continue,
                                    };

                                    headers.insert(name, value);
                                }
                                headers
                            }
                            Ok(None) => {
                                *this.state = ViceroyBodyWrapperState::Done;
                                return task::Poll::Ready(None);
                            }
                            Err(e) => return task::Poll::Ready(Some(Err(e))),
                        };

                    return task::Poll::Ready(Some(Ok(http_body::Frame::trailers(trailers))));
                }
                ViceroyBodyWrapperState::Done => {
                    return task::Poll::Ready(None);
                }
            }
        }
    }

    fn size_hint(&self) -> http_body::SizeHint {
        let size_hint = http_body_04::Body::size_hint(&self.body);

        let mut hint = http_body::SizeHint::new();
        hint.set_lower(size_hint.lower());
        if let Some(upper) = size_hint.upper() {
            hint.set_upper(upper);
        }

        hint
    }
}
