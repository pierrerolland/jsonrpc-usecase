use crate::JsonRpcService;
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::post,
};

pub fn router(service: JsonRpcService) -> Router {
    let endpoint = service.endpoint().to_owned();

    Router::new()
        .route(&endpoint, post(handle_rpc))
        .with_state(service)
}

async fn handle_rpc(State(service): State<JsonRpcService>, body: Bytes) -> Response {
    let body = String::from_utf8_lossy(&body);

    match service.handle_json(&body).await {
        Some(response) => ([(CONTENT_TYPE, "application/json")], response).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}
