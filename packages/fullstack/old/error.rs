/// A custom header that can be used to indicate a server function returned an error.
pub const SERVER_FN_ERROR_HEADER: &str = "serverfnerror";

/// Serializes and deserializes JSON with [`serde_json`].
pub struct ServerFnErrorEncoding;

impl ContentType for ServerFnErrorEncoding {
    const CONTENT_TYPE: &'static str = "text/plain";
}

impl FormatType for ServerFnErrorEncoding {
    const FORMAT_TYPE: Format = Format::Text;
}

impl Encodes<ServerFnError> for ServerFnErrorEncoding {
    type Error = std::fmt::Error;

    fn encode(output: &ServerFnError) -> Result<Bytes, Self::Error> {
        let mut buf = String::new();
        let result = match output {
            ServerFnError::Registration(e) => {
                write!(&mut buf, "Registration|{e}")
            }
            ServerFnError::Request(e) => write!(&mut buf, "Request|{e}"),
            ServerFnError::Response(e) => write!(&mut buf, "Response|{e}"),
            ServerFnError::ServerError(e) => {
                write!(&mut buf, "ServerError|{e}")
            }
            ServerFnError::MiddlewareError(e) => {
                write!(&mut buf, "MiddlewareError|{e}")
            }
            ServerFnError::Deserialization(e) => {
                write!(&mut buf, "Deserialization|{e}")
            }
            ServerFnError::Serialization(e) => {
                write!(&mut buf, "Serialization|{e}")
            }
            ServerFnError::Args(e) => write!(&mut buf, "Args|{e}"),
            ServerFnError::MissingArg(e) => {
                write!(&mut buf, "MissingArg|{e}")
            }
            ServerFnError::UnsupportedRequestMethod(e) => {
                write!(&mut buf, "UnsupportedRequestMethod|{e}")
            }
        };

        match result {
            Ok(()) => Ok(Bytes::from(buf)),
            Err(e) => Err(e),
        }
    }
}

impl Decodes<ServerFnError> for ServerFnErrorEncoding {
    type Error = String;

    fn decode(bytes: Bytes) -> Result<ServerFnError, Self::Error> {
        let data = String::from_utf8(bytes.to_vec())
            .map_err(|err| format!("UTF-8 conversion error: {err}"))?;

        data.split_once('|')
            .ok_or_else(|| format!("Invalid format: missing delimiter in {data:?}"))
            .and_then(|(ty, data)| match ty {
                "Registration" => Ok(ServerFnError::Registration(data.to_string())),
                "Request" => Ok(ServerFnError::Request(data.to_string())),
                "Response" => Ok(ServerFnError::Response(data.to_string())),
                "ServerError" => Ok(ServerFnError::ServerError(data.to_string())),
                "MiddlewareError" => Ok(ServerFnError::MiddlewareError(data.to_string())),
                "Deserialization" => Ok(ServerFnError::Deserialization(data.to_string())),
                "Serialization" => Ok(ServerFnError::Serialization(data.to_string())),
                "Args" => Ok(ServerFnError::Args(data.to_string())),
                "MissingArg" => Ok(ServerFnError::MissingArg(data.to_string())),
                _ => Err(format!("Unknown error type: {ty}")),
            })
    }
}

impl FromServerFnError for ServerFnError {
    type Encoder = ServerFnErrorEncoding;

    fn from_server_fn_error(value: ServerFnError) -> Self {
        match value {
            ServerFnError::Registration(value) => ServerFnError::Registration(value),
            ServerFnError::Request(value) => ServerFnError::Request(value),
            ServerFnError::ServerError(value) => ServerFnError::ServerError(value),
            ServerFnError::MiddlewareError(value) => ServerFnError::MiddlewareError(value),
            ServerFnError::Deserialization(value) => ServerFnError::Deserialization(value),
            ServerFnError::Serialization(value) => ServerFnError::Serialization(value),
            ServerFnError::Args(value) => ServerFnError::Args(value),
            ServerFnError::MissingArg(value) => ServerFnError::MissingArg(value),
            ServerFnError::Response(value) => ServerFnError::Response(value),
            ServerFnError::UnsupportedRequestMethod(value) => {
                ServerFnError::UnsupportedRequestMethod(value)
            }
        }
    }
}

/// A trait for types that can be returned from a server function.
pub trait FromServerFnError: Debug + Sized + 'static {
    /// The encoding strategy used to serialize and deserialize this error type. Must implement the [`Encodes`](server_fn::Encodes) trait for references to the error type.
    type Encoder: Encodes<Self> + Decodes<Self>;

    /// Converts a [`ServerFnError`] into the application-specific custom error type.
    fn from_server_fn_error(value: ServerFnError) -> Self;

    /// Converts the custom error type to a [`String`].
    fn ser(&self) -> Bytes {
        Self::Encoder::encode(self).unwrap_or_else(|e| {
            Self::Encoder::encode(&Self::from_server_fn_error(ServerFnError::Serialization(
                e.to_string(),
            )))
            .expect(
                "error serializing should success at least with the \
                 Serialization error",
            )
        })
    }

    /// Deserializes the custom error type from a [`&str`].
    fn de(data: Bytes) -> Self {
        Self::Encoder::decode(data)
            .unwrap_or_else(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())
    }
}

/// A helper trait for converting a [`ServerFnError`] into an application-specific custom error type that implements [`FromServerFnError`].
pub trait IntoAppError<E> {
    /// Converts a [`ServerFnError`] into the application-specific custom error type.
    fn into_app_error(self) -> E;
}

impl<E> IntoAppError<E> for ServerFnError
where
    E: FromServerFnError,
{
    fn into_app_error(self) -> E {
        E::from_server_fn_error(self)
    }
}

#[test]
fn assert_from_server_fn_error_impl() {
    fn assert_impl<T: FromServerFnError>() {}

    assert_impl::<ServerFnError>();
}
