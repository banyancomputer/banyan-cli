use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct BanyanArgs {
    #[clap(subcommand)]
    pub cmd_type: CommandType,
}

/* Our Main Command Types */

#[derive(Debug, Subcommand)]
pub enum CommandType {
    /// Create, update, or show deals
    #[clap(about = "Submit a deal to the Estuary network")]
    Deal(DealCommand),

    // /// Configure the Banyan CLI
    // #[clap(about = "Configure the Banyan CLI")]
    // Config(ConfigCommand),
}

/* Deal Subcommands */

#[derive(Debug, Args)]
pub struct DealCommand {
    #[clap(subcommand)]
    pub cmd: DealSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum DealSubcommand {
    /// Create a new deal
    #[clap(about = "Submit a new deal")]
    Submit(SubmitDeal),

    /// Show information about a deal
    #[clap(about = "Show information about a deal")]
    Show,

    // /// Update an existing deal
    // #[clap(about = "Update an existing deal")]
    // Update(UpdateCommand),
}

/* Submit Deal Command */

#[derive(Debug, Args)]
pub struct SubmitDeal {
    /// The path to the file to submit a deal for
    pub file: String,

    // #[clap(short, long, about = "The address of the executor")]
    // pub executor_address: String,
    //
    // #[clap(short, long, about = "How long the deal should last")]
    // pub deal_length_in_blocks: u32,
    //
    // #[clap(short, long, about = "How often the executor should submit proofs")]
    // pub proof_frequency_in_blocks: u32,
    //
    // #[clap(short, long, about = "How much to pay the executor per TiB")]
    // pub bounty_per_tib: f64,
    //
    // #[clap(short, long, about = "How much collateral to put up per TiB")]
    // pub collateral_per_tib: f64,
    //
    // #[clap(short, long, about = "The ERC20 token to use for collateral/bounty")]
    // pub erc20_token_denomination: String,
}


/* Config Subcommands */

// #[derive(Debug, Args)]
// pub struct ConfigCommand {
//     #[clap(subcommand)]
//     pub cmd: ConfigSubcommand,
// }