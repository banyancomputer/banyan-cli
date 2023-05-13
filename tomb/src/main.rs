#![feature(io_error_more)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
#![deny(private_in_public)]
#![deny(unreachable_pub)]

//! this crate is the binary for the tomb project. It contains the main function and the command line interface.

use clap::Parser;
use std::{io::Write, net::Ipv4Addr};
use tomb_lib::{
    pipelines::{
        pack_pipeline::pack_pipeline, push_pipeline::push_pipeline,
        unpack_pipeline::unpack_pipeline,
    },
    types::networkblockstore::NetworkBlockStore,
};

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
            pack_pipeline(&input_dir, &output_dir, chunk_size, follow_links)
                .await
                .unwrap();
        }
        // Execute the unpacking command
        cli::Commands::Unpack {
            input_dir,
            output_dir,
        } => {
            unpack_pipeline(&input_dir, &output_dir).await.unwrap();
        }
        cli::Commands::Init => unimplemented!("todo... create the tombolo file in the current directory"),
        cli::Commands::Login => unimplemented!("todo... a little script where you log in to the remote and enter your api key. just ends if you're authenticated. always does an auth check. little green checkmark :D."),
        cli::Commands::Register { bucket_name: _ } =>
            unimplemented!("todo... register a bucket on the remote. should create a database entry on the remote. let alex know we need one more api call for this."),
        cli::Commands::Configure { subcommand } => {
            match subcommand {
                cli::ConfigSubCommands::ContentScratchPath { path: _ } => {
                    unimplemented!("todo... change where we stage content locally");
                }
                cli::ConfigSubCommands::SetUrl { bucket_name: _ } => {
                    unimplemented!("todo... change bucket location... maybe maybe danger if unrelated version history");
                }
            }
        },
        cli::Commands::Pull => unimplemented!("todo... pull all the diffs? we might not support that yet."),
        cli::Commands::Push {
            input_dir,
            address,
            port
        } => {
            // Represent the string as an array of four numbers exactly
            let numbers: [u8; 4] = address
                .split('.')
                .map(|s| s.parse::<u8>().unwrap())
                .collect::<Vec<u8>>()
                .as_slice()
                .try_into()
                .unwrap();
            // Construct the IP Address from these numbers
            let ip = Ipv4Addr::from(numbers);
            // Construct the NetworkBlockStore from this IP and Port combination
            let store = NetworkBlockStore::new(ip, port);
            // Start the Push pipeline
            push_pipeline(&input_dir, &store).await.unwrap();
        },
        cli::Commands::Daemon => unimplemented!("todo... omg fun... cronjob"),
    }
}
