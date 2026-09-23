use std::sync::Arc;

use crate::{LiveViewError, LiveViewPool, LiveViewSocket, LiveviewRouter, interpreter_glue};
use axum::{
    Router,
    extract::{
        Path, Request, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::Html,
    routing::*,
};
use futures_util::{SinkExt, StreamExt};

/// Convert an Axum WebSocket into a `LiveViewSocket`.
///
/// This is required to launch a LiveView app using the Axum web framework.
/// Applications that construct their own router should also mount [`axum_file_upload`] at the
/// websocket path with `/upload/{token}` appended so file inputs can use HTTP uploads.
pub fn axum_socket(ws: WebSocket) -> impl LiveViewSocket {
    ws.map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, axum::Error>) -> Result<Vec<u8>, LiveViewError> {
    message
        .map_err(|_| LiveViewError::SendingFailed)?
        .into_text()
        .map(|text| text.as_str().into())
        .map_err(|_| LiveViewError::SendingFailed)
}

async fn transform_tx(message: Vec<u8>) -> Result<Message, axum::Error> {
    Ok(Message::Binary(message.into()))
}

/// Create the HTTP route that receives files for a [`LiveViewPool`].
///
/// Mount this at the websocket route with `/upload/{token}` appended. For example, a websocket
/// mounted at `/liveview` should mount this route at `/liveview/upload/{token}`.
pub fn axum_file_upload(view: LiveViewPool) -> MethodRouter {
    put(move |Path(token): Path<String>, request: Request| {
        let view = view.clone();
        async move {
            let size = request
                .headers()
                .get("X-Content-Size")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let mut upload = view
                .uploads
                .begin(&token, size)
                .await
                .map_err(upload_response_error)?;
            let mut body = request.into_body().into_data_stream();
            loop {
                let chunk = tokio::select! {
                    biased;
                    _ = upload.cancelled() => {
                        return Err(upload_response_error(crate::upload::UploadError::Canceled));
                    }
                    chunk = body.next() => chunk,
                };
                let Some(chunk) = chunk else { break };
                let chunk = chunk.map_err(|_| {
                    upload_response_error(crate::upload::UploadError::BodyReadFailed)
                })?;
                upload.write(chunk).await.map_err(upload_response_error)?;
            }
            upload.finish().map_err(upload_response_error)?;
            Ok::<_, (StatusCode, String)>(StatusCode::NO_CONTENT)
        }
    })
}

fn upload_response_error(error: crate::upload::UploadError) -> (StatusCode, String) {
    let status = match error {
        crate::upload::UploadError::UnknownOrExpired => StatusCode::NOT_FOUND,
        crate::upload::UploadError::AlreadyStarted => StatusCode::CONFLICT,
        crate::upload::UploadError::Canceled => StatusCode::GONE,
        crate::upload::UploadError::StorageFailed(_) => StatusCode::INSUFFICIENT_STORAGE,
        crate::upload::UploadError::LimitExceeded => StatusCode::PAYLOAD_TOO_LARGE,
        crate::upload::UploadError::FileCountLimitExceeded => StatusCode::PAYLOAD_TOO_LARGE,
        crate::upload::UploadError::BodyReadFailed => StatusCode::BAD_REQUEST,
        crate::upload::UploadError::SizeMismatch => StatusCode::BAD_REQUEST,
        crate::upload::UploadError::Incomplete => StatusCode::CONFLICT,
    };
    (status, error.to_string())
}

impl LiveviewRouter for Router {
    fn create_default_liveview_router() -> Self {
        Router::new()
    }

    fn with_virtual_dom(
        self,
        route: &str,
        app: impl Fn() -> dioxus_core::VirtualDom + Send + Sync + 'static,
    ) -> Self {
        let view = crate::LiveViewPool::new();

        let ws_path = format!("{}/ws", route.trim_start_matches('/'));
        let upload_path = format!("{ws_path}/upload/{{token}}");
        let title = crate::app_title();

        let index_page_with_glue = move |glue: &str| {
            Html(format!(
                r#"
        <!DOCTYPE html>
        <html>
            <head><title>{title}</title></head>
            <body><div id="main"></div></body>
            {glue}
        </html>
        "#,
            ))
        };

        let app = Arc::new(app);
        // Add an extra catch all segment to the route
        let mut route = route.trim_matches('/').to_string();
        if route.is_empty() {
            route = "/{*route}".to_string();
        } else {
            route = format!("/{route}/{{*route}}");
        }

        let websocket_view = view.clone();
        self.route(
            &ws_path,
            get(move |ws: WebSocketUpgrade| async move {
                let app = app.clone();
                let view = websocket_view.clone();
                ws.on_upgrade(move |socket| async move {
                    _ = view
                        .launch_virtualdom(axum_socket(socket), move || app())
                        .await;
                })
            }),
        )
        .route(&upload_path, axum_file_upload(view))
        .route(
            &route,
            get(move || async move { index_page_with_glue(&interpreter_glue(&ws_path)) }),
        )
    }

    async fn start(self, address: impl Into<std::net::SocketAddr>) {
        let listener = tokio::net::TcpListener::bind(address.into()).await.unwrap();
        if let Err(err) = axum::serve(listener, self.into_make_service()).await {
            eprintln!("Failed to start axum server: {}", err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use std::{convert::Infallible, time::Duration};
    use tower::ServiceExt;

    #[tokio::test]
    async fn cancellation_releases_a_stalled_http_upload() {
        let view = LiveViewPool::new().with_upload_storage_limit(3);
        let session = view.uploads.new_session();
        let reservation = view.uploads.reserve(&session, 3).unwrap();
        let token = view.uploads.register_reserved(reservation);
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let body = futures_util::stream::once(async move {
            let _ = started_tx.send(());
            Ok::<_, Infallible>(vec![1_u8])
        })
        .chain(futures_util::stream::pending());
        let request = Request::builder()
            .method("PUT")
            .uri(format!("/upload/{}", token))
            .header("X-Content-Size", "3")
            .body(Body::from_stream(body))
            .unwrap();
        let router = Router::new().route("/upload/{token}", axum_file_upload(view.clone()));
        let response = tokio::spawn(router.oneshot(request));
        started_rx.await.unwrap();
        assert_eq!(
            view.uploads.reserve(&session, 1).err(),
            Some(crate::upload::UploadError::LimitExceeded)
        );
        view.uploads.cancel(&token);

        let response = tokio::time::timeout(Duration::from_secs(1), response)
            .await
            .expect("cancellation should stop the request without another body chunk")
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::GONE);
        assert!(view.uploads.reserve(&session, 3).is_ok());
    }
}
