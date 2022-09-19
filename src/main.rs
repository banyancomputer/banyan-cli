#![deny(unused_crate_dependencies)]

mod args;
mod banyan;

use args::BanyanArgs;
use banyan::BanyanClient;
use banyan_shared::{
    deals::*,
    types::*,
};
use clap::Parser;
use spinners::{Spinner, Spinners};
use cli_table::{format::Justify, print_stdout, Cell, Style, Table};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the CLI arguments
    let args: BanyanArgs = BanyanArgs::parse();
    // Create a BanyanClient
    let client = BanyanClient::default();
    // Route the CLI arguments to the appropriate function
    match args.command_type {
        args::CommandType::Deal(deal_command) => match deal_command.command {
            args::DealSubcommand::Submit(submit_deal) => {
                println!("Submitting a Deal for {}...", &submit_deal.file);
                // Open the file
                let file = std::fs::File::open(&submit_deal.file).unwrap_or_else(|_| {
                    panic!("Could not open file: {}", &submit_deal.file);
                });
                // TODO: Get initial builder from config / From CLI
                let deal_proposal = DealProposalBuilder::default()
                    .with_file(file)
                    .build()?;
                // Save the b3hash for later
                let b3_hash_str = &deal_proposal.blake3_checksum.to_hex();
                // Display the deal proposal and ask for confirmation
                println!("Deal Proposal:\n{}", deal_proposal);
                println!("Are you sure you want to submit this deal? [y/N]");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim().to_lowercase() == "y" {
                    // Submit the deal
                    let mut sp = Spinner::new(Spinners::Dots9, "Submitting Deal...".into());
                    // TODO - Configurable Gas Price
                    let deal_id = client.propose_deal(
                        deal_proposal, None, None
                    ).await?;
                    sp.stop();
                    println!("Deal Submitted: {}", deal_id);
                    // If we should stage the file, do so
                    if submit_deal.stage {
                        let mut sp = Spinner::new(Spinners::Dots9, "Staging File...".into());
                        client.stage_file(
                            submit_deal.file,
                            Some(deal_id.id().to_string()),
                            Some(b3_hash_str.to_string()),
                        ).await?;
                        sp.stop();
                        println!("File Staged");
                    } else {
                        println!("File not staged. To stage the file, run:");
                        println!(
                            "banyan-cli content stage {} -d {} -b {}",
                            submit_deal.file, deal_id.id(), b3_hash_str
                        );
                    }
                } else {
                    println!("Deal not submitted");
                }
            }
            args::DealSubcommand::Show(show_deal) => {
                let mut sp = Spinner::new(Spinners::Dots9, "Fetching Deal...".into());
                // read the string as a u64, then convert to a DealId
                let deal_id = DealID(show_deal.deal_id.parse::<u64>()?);
                let deal = client.get_deal(deal_id).await?;
                sp.stop();
                println!("Deal:\n{}", deal);
            }
        },
        args::CommandType::Content(content_command) => match content_command.command {
            args::ContentSubcommand::Stage(stage_content) => {
                let mut sp = Spinner::new(Spinners::Dots9, "Staging File...".into());
                client.stage_file(
                    stage_content.file,
                    stage_content.deal_id,
                    stage_content.b3hash,
                ).await?;
                sp.stop();
                println!("File Staged");
            }
            args::ContentSubcommand::Ls => {
                let mut rows = vec![];

                let mut sp = Spinner::new(Spinners::Dots9, "Retrieving Content...".into());
                let content = client.get_content().await?;
                sp.stop();
                // If there is no content, return
                if content.is_empty() {
                    println!("No content found");
                    return Ok(());
                }
                println!("Content:");
                for c in content {
                    // Push the content to the table
                    rows.push(vec![
                        c.id.to_string().cell(),
                        c.cid_str.cell(),
                        c.deal_id.to_string().cell().justify(Justify::Right),
                    ]);
                }
                let table = rows.table()
                    .title(vec![
                        "Estuary ID".cell().bold(true),
                        "IPFS CID".cell().bold(true),
                        "Deal ID".cell().bold(true),
                    ])
                    .bold(true);
                print_stdout(table)?;
            }
        },
    }
    Ok(())
}
