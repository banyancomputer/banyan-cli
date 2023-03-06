#![feature(io_error_more)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]

use clap::Parser;
use dataprep_lib::do_pipeline_and_write_metadata::{pack_pipeline::pack_pipeline, unpack_pipeline::unpack_pipeline};
use fclones::config::GroupConfig;

mod cli;

#[tokio::main]
async fn main() {
    // Parse command line arguments. see args.rs
    let cli = cli::Args::parse();

    match cli.command {
        cli::Commands::Pack {
            input_dir,
            output_dir,
            manifest_file,
            target_chunk_size,
            group_config,
        } => {
            pack_pipeline(
                input_dir,
                output_dir,
                manifest_file,
                target_chunk_size,
                GroupConfig::default(),
            )
            .await
            .unwrap();
        }
        cli::Commands::Unpack {
            input_dir,
            manifest_file,
            output_dir,
        } => {
            unpack_pipeline(input_dir, output_dir, manifest_file)
                .await
                .unwrap();
        }
    }
}
