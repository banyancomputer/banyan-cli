use banyan_guts::cli::args::Args;
use clap::Parser;
use tracing::Level;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

/*
claudia todo list
clean up all unwraps
clean up all comments
clean up all errors
bring things from banyan-guts into appropriate libraries as appropriate
cut up banyan-guts as appropriate
variables for localhost endpoint
configurable port?
config files in general
*/

#[tokio::main]
async fn main() {
    // Parse command line arguments. see args.rs
    let cli = Args::parse();

    // TODO: is there anything we need to do here
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stderr());
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_writer(non_blocking_writer)
        .with_filter(env_filter);

    tracing_subscriber::registry().with(stderr_layer).init();

    let serialized_command = serde_json::to_vec(&cli.command).unwrap();
    let client = reqwest::Client::new();

    // TODO: variable-ify this endpoint/port
    let res = client
        .post("http://127.0.0.1:3000/")
        .body(serialized_command)
        .send()
        .await
        .unwrap();

    // TODO: do something better than just printing this. lame
    // TODO: error handle
    println!("{}", res.text().await.unwrap());
}
