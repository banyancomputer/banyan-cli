use clap::Subcommand;

/// Subcommand for endpoint configuration
#[derive(Subcommand, Clone, Debug)]
pub enum ApiCommand {
    /// Address of Core server
    Core {
        /// Server address
        #[clap(subcommand)]
        command: Option<AddressCommand>,
    },
    /// Address of Data server
    Data {
        /// Server address
        #[clap(subcommand)]
        command: Option<AddressCommand>,
    },
    /// Address of Frontend server
    Frontend {
        /// Server address
        #[clap(subcommand)]
        command: Option<AddressCommand>,
    },
}

/// Subcommand for getting and setting remote addresses
#[derive(Subcommand, Clone, Debug)]
pub enum AddressCommand {
    /// Set the address to a new value
    Set {
        /// Server address
        #[arg(short, long)]
        address: String,
    },
}

// #[async_trait(?Send)]
// impl RunnableCommand<TombError> for AccountCommand {
//     async fn run(&self) -> Result<String, TombError> {
//     }
// }
