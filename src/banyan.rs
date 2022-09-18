use anyhow::{Error, Result};
use banyan_shared::{deals::*, eth::*, types::*};
use ethers::types::Address;
use std::env::var;

/// BanyanClient - A one stop struct for interacting with our Backend
pub struct BanyanClient {
    /* Eth Stuff */
    /// Our Eth Client for Banyan
    pub eth_client: EthClient,
    /* Estuary Stuff */
    /// The Estuary API Hostname
    pub estuary_api_hostname: String,
    /// The Estuary API Key
    pub estuary_api_key: String,
}

impl Default for BanyanClient {
    /// Create a new BanyanClient from the Environment
    fn default() -> Self {
        BanyanClient {
            // Initialize our Eth Client from the environment
            eth_client: EthClient::default(),
            estuary_api_hostname: var("ESTUARY_API_HOSTNAME")
                .unwrap_or("http://localhost:3004".to_string()),
            estuary_api_key: var("ESTUARY_API_KEY").unwrap_or("".to_string()),
        }
    }
}

impl BanyanClient {
    /// Create a new BanyanClient using custom values
    /// # Arguments
    /// * `eth_api_url` - The URL of the Ethereum API to use. This is required.
    /// * `eth_api_key` - The API Key to use for the Ethereum API. This is required.
    /// * `eth_chain_id` - The Chain ID of the Ethereum Network to use. This is required.
    /// * `eth_private_key` - The Private Key to use for the Ethereum Wallet. This is required.
    /// * `eth_contract_address` - The Address of the Banyan Contract. This is required.
    /// * `estuary_api_hostname` - The Hostname of the Estuary API to use.
    /// * `estuary_api_key` - The API Key to use for the Estuary API.
    pub fn new(
        eth_api_url: String,
        eth_api_key: String,
        eth_chain_id: u64,
        eth_private_key: String,
        eth_contract_address: Address,
        estuary_api_hostname: String,
        estuary_api_key: String,
    ) -> BanyanClient {
        BanyanClient {
            // Initialize our Eth Client from the environment
            eth_client: EthClient::new(
                eth_api_url,
                eth_api_key,
                Some(eth_chain_id),
                Some(eth_private_key),
                eth_contract_address,
            )
            .unwrap(),
            estuary_api_hostname,
            estuary_api_key,
        }
    }

    /// Process a File into a DealProposal and Web Request
    /// # Arguments
    /// * `file_path` - The path to the file to process
    /// * `dp_builder` - The (optional) DealProposalBuilder to use. If None, we will use the default.
    /// # Returns
    /// * `Result<DealProposal, Error>` - The DealProposal
    /// # Errors
    /// * `Error` - If there is an error processing the file
    pub fn prepare_deal(
        &self,
        file_path: &str,
        dp_builder: Option<DealProposalBuilder>,
    ) -> Result<DealProposal, Error> {
        // In order to create a deal, we need to read the file into a std::fs::File
        let file = std::fs::File::open(file_path)?;
        let dp = match dp_builder {
            Some(dp_builder) => dp_builder.build(&file).unwrap(),
            None => DealProposalBuilder::default().build(&file).unwrap(),
        };
        Ok(dp)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    /// Test that we can create a BanyanClient from the Environment
    fn default_client() {
        let client = BanyanClient::default();
        return;
    }
}
