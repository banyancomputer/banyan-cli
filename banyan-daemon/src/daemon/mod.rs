use axum::{body::Bytes, http::StatusCode};
use axum::{routing::post, Router};
use banyan_guts::native::NativeError;

pub async fn start_daemon() -> Result<(), NativeError> {
    // TODO check if already running

    let app = Router::new()
        .route("/", post(handler))
        .layer(tower_http::trace::TraceLayer::new_for_http());
    let listen = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listen, app.into_make_service()).await.unwrap();

    Ok(())

    // TODO daemonize yourself

    /* EXAMPLE FROM DAEMONIZE LIBRARY: TODO later
       let stdout = File::create("/tmp/daemon.out").unwrap();
       let stderr = File::create("/tmp/daemon.err").unwrap();

       let daemonize = Daemonize::new()
           .pid_file("/tmp/test.pid") // Every method except `new` and `start`
           .chown_pid_file(true)      // is optional, see `Daemonize` documentation
           .working_directory("/tmp") // for default behaviour.
           .user("nobody")
           .group("daemon") // Group name
           .group(2)        // or group id.
           .umask(0o777)    // Set umask, `0o027` by default.
           .stdout(stdout)  // Redirect stdout to `/tmp/daemon.out`.
           .stderr(stderr)  // Redirect stderr to `/tmp/daemon.err`.
           .privileged_action(|| "Executed before drop privileges");

       match daemonize.start() {
           Ok(_) => println!("Success, daemonized"),
           Err(e) => eprintln!("Error, {}", e),
       }
    */
}

async fn handler(body: Bytes) -> Result<String, StatusCode> {
    let parse_body = String::from_utf8(body.to_vec())
        .unwrap()
        .parse::<serde_json::Value>()
        .unwrap();
    Ok(format!("Hello, World! {parse_body:?}"))
}
