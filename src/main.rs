mod args;
mod banyan;
mod estuary;

use args::BanyanArgs;
use banyan::BanyanClient;
use clap::Parser;
use banyan_shared::{types::*, deals::*};

#[tokio::main]
async fn main() {
    // Parse the CLI arguments
    let args: BanyanArgs = BanyanArgs::parse();
    // Route the CLI arguments to the appropriate function
    match args.command_type {
        args::CommandType::Deal(deal_command) => match deal_command.command {
            args::DealSubcommand::Submit(submit_deal) => {
                println!("Preparing to submit a deal for {}", &submit_deal.file);
                // Read the file to submit a deal for as a std::fs::File and tokio::fs::File
                let file = std::fs::File::open(&submit_deal.file).unwrap();
                let tokio_file = tokio::fs::File::open(&submit_deal.file).await.unwrap();
                // Create a Banyan Client
                let banyan_client = BanyanClient::default();
                // Generate the Deal Proposal
                let deal_proposal = DealProposalBuilder::default()
                    .build(&file)
                    .unwrap();
                println!("Generated a deal proposal: {:?}", deal_proposal);
                // Submit the deal
                let b3_hash_str = &deal_proposal.blake3_checksum.to_hex();
                let deal_id_str = banyan_client.eth_client.propose_deal(
                    deal_proposal, None, None
                ).await.unwrap().0.to_string();
                println!("Submitted a deal with ID {}", deal_id_str);
                println!("Staging file with Blake3 Hash {}", b3_hash_str);
                banyan_client.estuary_client.stage_file(
                    submit_deal.file,
                    deal_id_str,
                    b3_hash_str.to_string(),
                ).await.unwrap();
            }
            args::DealSubcommand::Show => {
                println!("Showings all deals");
            }
        },
        args::CommandType::Config(config_command) => {
            println!("Configuring the Banyan CLI");
        }
    }
    return;
}
