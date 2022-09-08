mod deal_maker;
mod args;

use clap::Parser;
use args::BanyanArgs;
// use deal_maker::DealMaker;

fn main() {
    // Parse the CLI arguments
    let args: BanyanArgs = BanyanArgs::parse();
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
