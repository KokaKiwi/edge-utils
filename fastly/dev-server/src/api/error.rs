use axum::{
    http,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, Clone, bon::Builder, Serialize)]
#[builder(on(String, into))]
pub struct Error {
    #[builder(default = http::StatusCode::INTERNAL_SERVER_ERROR)]
    #[serde(skip)]
    status_code: http::StatusCode,
    message: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        tracing::debug!(error.message = self.message, "API error");
        (self.status_code, axum::Json(self)).into_response()
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error,
{
    fn from(err: E) -> Self {
        tracing::error!(error.message = %err, "Internal error");
        Error::builder().message("Internal server error").build()
    }
}

const _: () = {
    use error_builder::*;

    impl<S: State> ErrorBuilder<S> {
        pub fn not_found(self) -> ErrorBuilder<SetStatusCode<S>>
        where
            S::StatusCode: IsUnset,
        {
            self.status_code(http::StatusCode::NOT_FOUND)
        }

        #[allow(unused)]
        pub fn not_implemented(self) -> ErrorBuilder<SetMessage<SetStatusCode<S>>>
        where
            S::StatusCode: IsUnset,
            S::Message: IsUnset,
        {
            self.status_code(http::StatusCode::NOT_IMPLEMENTED)
                .message("Not implemented")
        }
    }
};
