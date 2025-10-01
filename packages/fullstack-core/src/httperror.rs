use axum_core::response::IntoResponse;
use http::StatusCode;
use std::fmt;

/// An error type that wraps an HTTP status code and optional message.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpError {
    pub status: StatusCode,
    pub message: Option<String>,
}

impl HttpError {
    pub fn new<M: Into<String>>(status: StatusCode, message: M) -> Self {
        HttpError {
            status,
            message: Some(message.into()),
        }
    }

    pub fn err<T>(status: StatusCode, message: impl Into<String>) -> Result<T, Self> {
        Err(HttpError::new(status, message))
    }

    // --- 4xx Client Errors ---
    pub fn bad_request<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::BAD_REQUEST, message)
    }
    pub fn unauthorized<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::UNAUTHORIZED, message)
    }
    pub fn payment_required<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::PAYMENT_REQUIRED, message)
    }
    pub fn forbidden<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::FORBIDDEN, message)
    }
    pub fn not_found<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::NOT_FOUND, message)
    }
    pub fn method_not_allowed<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::METHOD_NOT_ALLOWED, message)
    }
    pub fn not_acceptable<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::NOT_ACCEPTABLE, message)
    }
    pub fn proxy_auth_required<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::PROXY_AUTHENTICATION_REQUIRED, message)
    }
    pub fn request_timeout<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::REQUEST_TIMEOUT, message)
    }
    pub fn conflict<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::CONFLICT, message)
    }
    pub fn gone<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::GONE, message)
    }
    pub fn length_required<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::LENGTH_REQUIRED, message)
    }
    pub fn precondition_failed<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::PRECONDITION_FAILED, message)
    }
    pub fn payload_too_large<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::PAYLOAD_TOO_LARGE, message)
    }
    pub fn uri_too_long<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::URI_TOO_LONG, message)
    }
    pub fn unsupported_media_type<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::UNSUPPORTED_MEDIA_TYPE, message)
    }
    pub fn im_a_teapot<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::IM_A_TEAPOT, message)
    }
    pub fn too_many_requests<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::TOO_MANY_REQUESTS, message)
    }

    // --- 5xx Server Errors ---
    pub fn internal_server_error<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
    pub fn not_implemented<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::NOT_IMPLEMENTED, message)
    }
    pub fn bad_gateway<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::BAD_GATEWAY, message)
    }
    pub fn service_unavailable<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::SERVICE_UNAVAILABLE, message)
    }
    pub fn gateway_timeout<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::GATEWAY_TIMEOUT, message)
    }
    pub fn http_version_not_supported<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::HTTP_VERSION_NOT_SUPPORTED, message)
    }

    // --- 2xx/3xx (rare, but for completeness) ---
    pub fn ok<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::OK, message)
    }
    pub fn created<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::CREATED, message)
    }
    pub fn accepted<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::ACCEPTED, message)
    }
    pub fn moved_permanently<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::MOVED_PERMANENTLY, message)
    }
    pub fn found<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::FOUND, message)
    }
    pub fn see_other<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::SEE_OTHER, message)
    }
    pub fn not_modified<T>(message: impl Into<String>) -> Result<T, Self> {
        Self::err(StatusCode::NOT_MODIFIED, message)
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(msg) => write!(f, "{}: {}", self.status, msg),
            None => write!(f, "{}", self.status),
        }
    }
}

impl std::error::Error for HttpError {}

/// Trait to convert errors into HttpError with a given status code.
pub trait OrHttpError<T, M>: Sized {
    fn or_http_error(self, status: StatusCode, message: impl Into<String>) -> Result<T, HttpError>;

    // --- Most common user-facing status codes ---
    fn or_bad_request(self, message: impl Into<String>) -> Result<T, HttpError> {
        self.or_http_error(StatusCode::BAD_REQUEST, message)
    }
    fn or_unauthorized(self, message: impl Into<String>) -> Result<T, HttpError> {
        self.or_http_error(StatusCode::UNAUTHORIZED, message)
    }
    fn or_forbidden(self, message: impl Into<String>) -> Result<T, HttpError> {
        self.or_http_error(StatusCode::FORBIDDEN, message)
    }
    fn or_not_found(self, message: impl Into<String>) -> Result<T, HttpError> {
        self.or_http_error(StatusCode::NOT_FOUND, message)
    }
    fn or_internal_server_error(self, message: impl Into<String>) -> Result<T, HttpError> {
        self.or_http_error(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

impl<T, E> OrHttpError<T, ()> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn or_http_error(self, status: StatusCode, message: impl Into<String>) -> Result<T, HttpError> {
        self.map_err(|_| HttpError {
            status,
            message: Some(message.into()),
        })
    }
}

impl<T> OrHttpError<T, ()> for Option<T> {
    fn or_http_error(self, status: StatusCode, message: impl Into<String>) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError {
            status,
            message: Some(message.into()),
        })
    }
}

impl OrHttpError<(), ()> for bool {
    fn or_http_error(
        self,
        status: StatusCode,
        message: impl Into<String>,
    ) -> Result<(), HttpError> {
        if self {
            Ok(())
        } else {
            Err(HttpError {
                status,
                message: Some(message.into()),
            })
        }
    }
}

pub struct AnyhowMarker;
impl<T> OrHttpError<T, AnyhowMarker> for Result<T, anyhow::Error> {
    fn or_http_error(self, status: StatusCode, message: impl Into<String>) -> Result<T, HttpError> {
        self.map_err(|_| HttpError {
            status,
            message: Some(message.into()),
        })
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum_core::response::Response {
        let body = match &self.message {
            Some(msg) => msg.clone(),
            None => self
                .status
                .canonical_reason()
                .unwrap_or("Unknown error")
                .to_string(),
        };
        (self.status, body).into_response()
    }
}
