mod client;
mod args;
mod hasher;

use clap::Parser;
use args::BanyanArgs;
use client::BanyanClient;

fn main() {
    // Parse the CLI arguments
    let args: BanyanArgs = BanyanArgs::parse();
    dbg!(args);
    // Route the CLI arguments to the appropriate function
    match args.command_type {
        args::CommandType::Deal(deal_command) => {
            match deal_command.command {
                args::DealSubcommand::Submit(submit_deal) => {
                    println!("Submitting a new deal for file: {}", submit_deal.file);
                },
                args::DealSubcommand::Show => {
                    println!("Showings all deals");
                },
            }
        },
        args::CommandType::Config(config_command) => {
            println!("Configuring the Banyan CLI");
        },
    }
    return;
}
