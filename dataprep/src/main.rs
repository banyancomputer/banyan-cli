#![feature(io_error_more)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
#![deny(private_in_public)]
#![deny(unreachable_pub)]

//! this crate is the binary for the dataprep project. It contains the main function and the command line interface.

use clap::Parser;
use dataprep_lib::do_pipeline_and_write_metadata::{
    pack_pipeline::pack_pipeline, unpack_pipeline::unpack_pipeline,
};
use std::io::Write;

mod cli;

#[tokio::main]
async fn main() {
    // Parse command line arguments. see args.rs
    let cli = cli::Args::parse();

    // TODO eventually make options to format it differently?
    env_logger::Builder::new()
        .filter_level(cli.verbose.into())
        .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
        .format_timestamp(None)
        .format_level(true)
        .format_module_path(false)
        .init();

    // Determine the command being executed
    match cli.command {
        // Execute the packing command
        cli::Commands::Pack {
            input_dir,
            output_dir,
            chunk_size,
            follow_links,
        } => {
            pack_pipeline(
                &input_dir,
                &output_dir,
                chunk_size,
                follow_links,
            )
            .await
            .unwrap();
        }
        // Execute the unpacking command
        cli::Commands::Unpack {
            input_dir,
            output_dir,
        } => {
            unpack_pipeline(&input_dir, &output_dir)
                .await
                .unwrap();
        }
    }
}
