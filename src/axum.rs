use crate::{JsonRpcService, RequestHeaders};
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::post,
};

pub fn router(service: JsonRpcService) -> Router {
    let endpoint = service.endpoint().to_owned();

    Router::new()
        .route(&endpoint, post(handle_rpc))
        .with_state(service)
}

async fn handle_rpc(
    State(service): State<JsonRpcService>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let body = String::from_utf8_lossy(&body);
    let headers = RequestHeaders::new(headers.iter().filter_map(|(name, value)| {
        value
            .to_str()
            .ok()
            .map(|value| (name.as_str(), value.to_owned()))
    }));

    match service.handle_json_with_headers(&body, headers).await {
        Some(response) => ([(CONTENT_TYPE, "application/json")], response).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}
