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
use std::{env::current_dir, io::Write};
use tomb::pipelines::{add, configure, pack, pull, push, unpack};
/// Command Line Interface and tests
pub mod cli;

use assert_cmd as _;
use async_recursion as _;
use chrono as _;
use criterion as _;
use dir_assert as _;
use fake_file as _;
use fclones as _;
use fs_extra as _;
use indicatif as _;
use jwalk as _;
use lazy_static as _;
use rand as _;
use serde as _;
use serial_test as _;
use thiserror as _;
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
            follow_links,
        } => {
            if let Some(input_dir) = input_dir {
                pack::pipeline(&input_dir, follow_links).await?;
            }
            else {
                pack::pipeline(&current_dir()?, follow_links).await?;
            }
        }
        // Execute the unpacking command
        cli::Commands::Unpack {
            input_dir,
            output_dir,
        } => {
            unpack::pipeline(&input_dir, &output_dir).await?;
        }
        cli::Commands::Init {
            dir
        } => {
            // Initialize here
            if let Some(dir) = dir {
                configure::init(&dir)?;
            }
            else {
                configure::init(&current_dir()?)?;
            }
        },
        cli::Commands::Deinit {
            dir
        } => {
            // Initialize here
            if let Some(dir) = dir {
                configure::deinit(&dir)?;
            }
            else {
                configure::deinit(&current_dir()?)?;
            }
        },
        cli::Commands::Login => unimplemented!("todo... a little script where you log in to the remote and enter your api key. just ends if you're authenticated. always does an auth check. little green checkmark :D."),
        cli::Commands::Register { bucket_name: _ } =>
            unimplemented!("todo... register a bucket on the remote. should create a database entry on the remote. let alex know we need one more api call for this."),
        cli::Commands::Configure { subcommand } => {
            match subcommand {
                cli::ConfigSubCommands::ContentScratchPath { path: _ } => {
                    unimplemented!("todo... change where we stage content locally");
                }
                cli::ConfigSubCommands::SetRemote { address } => {
                    configure::remote(&address)?;
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
        cli::Commands::Add { input_file, tomb_path, wnfs_path } => {
            add::pipeline(&input_file, &tomb_path, &wnfs_path).await?;
        },
        cli::Commands::Remove { tomb_path: _, wnfs_path: _ } => todo!("remove")
    }

    Ok(())
}
