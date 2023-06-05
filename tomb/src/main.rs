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
use std::{env, io::Write};
use tomb::pipelines::{add, configure, pack, pull, push, unpack};
/// Command Line Interface and tests
pub mod cli;

use assert_cmd as _;
use async_recursion as _;
use blake2 as _;
use chrono as _;
use criterion as _;
use dir_assert as _;
use fake_file as _;
use fclones as _;
use fs_extra as _;
use indicatif as _;
use jwalk as _;
use lazy_static as _;
use predicates as _;
use rand as _;
use serde as _;
use wnfs as _;
use zstd as _;

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
            match input_dir {
                Some(input_dir) => {
                    unpack::pipeline(Some(&input_dir), &output_dir).await?;
                },
                None => {
                    unpack::pipeline(None, &output_dir).await?;
                },
            }
        }
        cli::Commands::Init {
            dir
        } => {
            // If no dir was supplied, use the current working directory
            let dir = dir.unwrap_or(env::current_dir()?);
            // Initialize here
            configure::init(&dir)?;
        },
        cli::Commands::Login => unimplemented!("todo... a little script where you log in to the remote and enter your api key. just ends if you're authenticated. always does an auth check. little green checkmark :D."),
        cli::Commands::Register { bucket_name: _ } =>
            unimplemented!("todo... register a bucket on the remote. should create a database entry on the remote. let alex know we need one more api call for this."),
        cli::Commands::Configure { subcommand } => {
            match subcommand {
                cli::ConfigSubCommands::ContentScratchPath { path: _ } => {
                    unimplemented!("todo... change where we stage content locally");
                }
                cli::ConfigSubCommands::SetRemote { dir, url, port } => {
                    // If no dir was supplied, use the current working directory
                    let dir = dir.unwrap_or(env::current_dir()?);
                    configure::remote(&dir, &url, port)?;
                }
            }
        },
        cli::Commands::Daemon => unimplemented!("todo... omg fun... cronjob"),
        cli::Commands::Pull {
            dir
        } => {
            // Start the Pull pipeline
            pull::pipeline(&dir).await?;
        },
        cli::Commands::Push {
            dir,
        } => {
            // Start the Push pipeline
            push::pipeline(&dir).await?;
        },
        cli::Commands::Add { local, input_file, tomb_path, wnfs_path } => {
            add::pipeline(local, &input_file, &tomb_path, &wnfs_path).await?;
        },
        cli::Commands::Remove { tomb_path: _, wnfs_path: _ } => todo!("remove")
    }

    Ok(())
}
