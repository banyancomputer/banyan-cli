use axum::{body::Bytes, http::StatusCode};
use axum::{routing::post, Router};
use banyan_guts::cli2::commands::{BanyanServiceApiCommand, RunnableCommand};
use banyan_guts::native::NativeError;

use banyan_guts::shared::{PID_FILE, STDERR, STDOUT};
use daemonize::Daemonize;
use std::fs::File;

pub fn daemonize_self() -> Result<String, NativeError> {
    let stdout_file = File::create(STDOUT).expect("Failed to create stdout file");
    let stderr_file = File::create(STDERR).expect("Failed to create stderr file");
    let daemonize = Daemonize::new()
        .pid_file(PID_FILE)
        .working_directory("/tmp")
        .user("nobody")
        .group("daemon")
        .stdout(stdout_file)
        .stderr(stderr_file)
        .privileged_action(|| start_service());
    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }
    Ok("Started daemon".to_string())
}

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
