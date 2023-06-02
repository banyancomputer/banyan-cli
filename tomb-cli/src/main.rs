#![feature(io_error_more)]
#![feature(let_chains)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
#![deny(private_in_public)]
#![deny(unreachable_pub)]

//! this crate is the binary for the tomb project. It contains the main function and the command line interface.

use anyhow::Result;
use clap::Parser;
use std::{
    fs::{create_dir_all, remove_dir_all},
    io::Write,
};
use tomb::pipelines::{add, pack, pull, push, unpack};
use tomb_common::utils::{get_network_blockstore, set_remote, tomb_config};
mod cli;
///
pub mod tests;

#[tokio::main]
async fn main() -> Result<()> {
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
            match output_dir {
                Some(output_dir) => {
                    pack::pipeline(&input_dir, Some(&output_dir), chunk_size, follow_links).await?;
                },
                None => {
                    pack::pipeline(&input_dir, None, chunk_size, follow_links).await?;
                },
            }
        }
        // Execute the unpacking command
        cli::Commands::Unpack {
            input_dir,
            output_dir,
        } => {
            unpack::pipeline(&input_dir, &output_dir).await?;
        }
        cli::Commands::Init => {
            let tomb_path = &tomb_config()?.join(".tomb");
            // Remove existing metadata
            remove_dir_all(tomb_path)?;
            // Create new metadata folder
            create_dir_all(tomb_path)?;
        },
        cli::Commands::Login => unimplemented!("todo... a little script where you log in to the remote and enter your api key. just ends if you're authenticated. always does an auth check. little green checkmark :D."),
        cli::Commands::Register { bucket_name: _ } =>
            unimplemented!("todo... register a bucket on the remote. should create a database entry on the remote. let alex know we need one more api call for this."),
        cli::Commands::Configure { subcommand } => {
            match subcommand {
                cli::ConfigSubCommands::ContentScratchPath { path: _ } => {
                    unimplemented!("todo... change where we stage content locally");
                }
                cli::ConfigSubCommands::SetRemote { url, port } => { set_remote(url, port)?; }
            }
        },
        cli::Commands::Daemon => unimplemented!("todo... omg fun... cronjob"),
        cli::Commands::Pull {
            dir
        } => {
            // Start the Pull pipeline
            pull::pipeline(&dir, &get_network_blockstore()?).await?;
        },
        cli::Commands::Push {
            input_dir,
        } => {
            // Start the Push pipeline
            push::pipeline(&input_dir, &get_network_blockstore()?).await?;
        },
        cli::Commands::Add {input_file, tomb_path, wnfs_path } => {
            add::pipeline(&input_file, &tomb_path, &wnfs_path).await?;
        },
        cli::Commands::Remove { tomb_path: _, wnfs_path: _ } => todo!("remove")
    }

    Ok(())
}
