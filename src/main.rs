use clap::Parser;

mod deal_maker;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    // Optionally provide a file to submit a deal for
    #[clap(short, long)]
    file: Option<String>,

    // Flag for indicating if the config should be written if provided
    #[clap(short, long)]
    write_config: bool,

    // Flag for Specifying Bounty-Per-TiB
    #[clap(short, long)]
    bounty_per_tib: Option<f64>,
    // Flag for Specifying Collateral-Per-TiB
    #[clap(short, long)]
    collateral_per_tib: Option<f64>,
    // Flag for Specifying ERC20 Token Denomination
    #[clap(short, long)]
    erc20_token_denomination: Option<String>,

}

fn main() {
    // Parse the command line arguments
    let args = Args::parse();


    let dm: deal_maker::DealMaker = confy::load("banyan-cli").unwrap();
    println!("Configuring Deals with Current Settings: {:#?}", dm);
    let deal_id = dm.submit_deal(args.file).unwrap();
    // Configure a deal for the file
}
