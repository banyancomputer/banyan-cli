#![feature(io_error_more)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]

use clap::Parser;
use dataprep_lib::do_pipeline_and_write_metadata::{
    pack_pipeline::pack_pipeline, unpack_pipeline::unpack_pipeline,
};

mod cli;

#[tokio::main]
async fn main() {
    // Parse command line arguments. see args.rs
    let cli = cli::Args::parse();
    // Determine the command being executed
    match cli.command {
        // Execute the packing command
        cli::Commands::Pack {
            input_dir,
            output_dir,
            manifest_file,
            chunk_size,
            follow_links,
        } => {
            pack_pipeline(
                &input_dir,
                &output_dir,
                &manifest_file,
                chunk_size,
                follow_links,
            )
            .await
            .unwrap();
        }
        // Execute the unpacking command
        cli::Commands::Unpack {
            manifest_file,
            output_dir,
        } => {
            unpack_pipeline(&output_dir, &manifest_file).await.unwrap();
        }
    }
}
