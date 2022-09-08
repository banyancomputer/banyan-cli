mod deal_maker;
mod args;

use clap::Parser;
use args::BanyanArgs;
// use deal_maker::DealMaker;

fn main() {
    let args: BanyanArgs = BanyanArgs::parse();
    println!("Args: {:#?}", args);
    return;
}
