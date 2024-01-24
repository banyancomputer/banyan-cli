// /// Prompt the user for a y/n answer
// pub fn prompt_for_bool(msg: &str) -> bool {
//     info!("{msg} y/n");
//     loop {
//         let mut input = [0];
//         let _ = std::io::stdin().read(&mut input);
//         match input[0] as char {
//             'y' | 'Y' => return true,
//             'n' | 'N' => return false,
//             _ => info!("y/n only please."),
//         }
//     }
// }

// #[tokio::main]
// async fn main() {
//     // Parse command line arguments. see args.rs
//     let cli = Args::parse();

//     let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stderr());
//     let env_filter = EnvFilter::builder()
//         .with_default_directive(Level::INFO.into())
//         .from_env_lossy();

//     let stderr_layer = tracing_subscriber::fmt::layer()
//         .pretty()
//         .with_target(false)
//         .with_file(false)
//         .with_line_number(false)
//         .with_writer(non_blocking_writer)
//         .with_filter(env_filter);

//     tracing_subscriber::registry().with(stderr_layer).init();

//     // Determine the command being executed run appropriate subcommand
//     let _ = cli.command.run().await;
// }

pub fn main() {
    unimplemented!("HEY!!! WHAT'S GOING ON HERE?");
}
