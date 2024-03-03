use axum::{body::Bytes, http::StatusCode};
use axum::{routing::post, Router};
use banyan_guts::cli2::commands::{BanyanServiceApiCommand, RunnableCommand};
use banyan_guts::native::NativeError;

pub async fn start_service() -> Result<(), NativeError> {
    let app = Router::new()
        .route("/", post(handler))
        .layer(tower_http::trace::TraceLayer::new_for_http());
    let listen = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listen, app.into_make_service()).await.unwrap();

    Ok(())
}

#[axum::debug_handler]
async fn handler(body: Bytes) -> Result<String, StatusCode> {
    let parse_body = serde_json::from_slice::<BanyanServiceApiCommand>(&body)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    parse_body
        .clone()
        .run()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // TODO cleanup
    Ok(format!(
        "Hello, World! running {parse_body:?} worked! see remote logs to learn more"
    ))
}
