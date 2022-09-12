mod deal_maker;
mod args;

use clap::Parser;
use args::BanyanArgs;
use client::BanyanClient;
// use deal_maker::DealMaker;

fn main() {
    // Parse the CLI arguments
    let args: BanyanArgs = BanyanArgs::parse();
    // Initialize the Client

    let client = BanyanClient::builder()
        .banyan_contract_address(args.banyan_contract_address)
        .eth_api_url(args.eth_api_url)
        .eth_api_key(args.eth_api_key)
        .eth_private_key(args.eth_private_key)
        .estuary_api_hostname(args.estuary_api_hostname)
        .estuary_api_key(args.estuary_api_key)
        .build()
        .unwrap();
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
