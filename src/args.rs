use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct BanyanArgs {
    #[clap(subcommand)]
    pub command_type: CommandType,
}

/* Our Main Command Types */

#[derive(Debug, Subcommand)]
pub enum CommandType {
    /// Create, update, or show deals
    #[clap(about = "Submit a deal to the Estuary network")]
    Deal(DealCommand),

    /// Stage or check content on Estuary
    #[clap(about = "Stage or check content on Estuary")]
    Content(ContentCommand),
    // /// Configure the Banyan CLI
    // #[clap(about = "Configure the Banyan CLI")]
    // Config(ConfigCommand),
}

/* Deal Subcommands */

#[derive(Debug, Args)]
pub struct DealCommand {
    #[clap(subcommand)]
    pub command: DealSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum DealSubcommand {
    /// Create a new deal
    #[clap(about = "Submit a new deal")]
    Submit(SubmitDeal),

    /// Show information about a deal
    #[clap(about = "Show information about a deal")]
    Show(ShowDeal),
}

/* Submit Deal Command */

#[derive(Debug, Args)]
pub struct SubmitDeal {
    /// The path to the file to submit a deal for
    #[clap(short, long)]
    pub file: String,

    /// The Executor to use for the Deal
    #[clap(short, long)]
    pub executor: Option<String>,

    /// The Deal Duration in Blocks
    #[clap(short, long)]
    pub length: Option<u64>,

    /// The Proof Frequency in Blocks to use for the Deal
    #[clap(short, long)]
    pub proof_frequency: Option<u64>,

    /// The bounty per TiB to use for the Deal
    #[clap(short, long)]
    pub bounty: Option<f64>,

    /// The Collateral per TiB to use for the Deal
    #[clap(short, long)]
    pub collateral: Option<f64>,

    /// The ERC20 Token Denomination to use for the Deal
    #[clap(short = 't', long = "token")]
    pub erc20_token_denomination: Option<String>,

    /// Optional: File gets staged on Estuary if present as a flag
    #[clap(short, long, action)]
    pub stage: bool,
    // /// The Config file to use
    // #[clap(short, long, default_value = "banyan.toml")]
    // pub config: String,
}

/* Show Deal Command */
#[derive(Debug, Args)]
pub struct ShowDeal {
    /// The ID of the deal to show
    pub deal_id: String,
    // /// The Config file to use
    // #[clap(short, long, default_value = "banyan.toml")]
    // pub config: String,
}

/* Content Subcommands */

#[derive(Debug, Args)]
pub struct ContentCommand {
    #[clap(subcommand)]
    pub command: ContentSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ContentSubcommand {
    /// Stage a file to Estuary
    #[clap(about = "Stage a file to Estuary")]
    Stage(StageContent),

    /// Get all content you've staged on Estuary
    #[clap(about = "Get all Content Stored on Estuary")]
    Ls,
}

/* Stage Content Command */
#[derive(Debug, Args)]
pub struct StageContent {
    /// The path to the file to stage
    pub file: String,

    /// The optional Deal ID to stage the file for
    #[clap(short, long)]
    pub deal_id: Option<String>,

    /// The optional Blake3 Hash to stage the file with
    #[clap(short = 'b', long)]
    pub b3hash: Option<String>,
    // /// The Config file to use
    // #[clap(short, long, default_value = "banyan.toml")]
    // pub config: String,
}

/* TODO: Config Subcommands */

// #[derive(Debug, Args)]
// pub struct ConfigCommand {
//     #[clap(subcommand)]
//     pub command: ConfigSubcommand,
// }

// #[derive(Debug, Subcommand)]
// pub enum ConfigSubcommand {
//     /// Create a new config file
//     #[clap(about = "Create a new config file")]
//     New(NewConfig),
//
//     /// Show information about the current Config
//     #[clap(about = "Show information about a config file")]
//     Show,
//
//     /// Update an existing config file
//     #[clap(about = "Update an existing config file")]
//     Update(UpdateConfig),
// }
