use anyhow::{Error, Result};
use banyan_shared::{eth::*};
use ethers::types::Address;
use std::env::var;
use crate::estuary::EstuaryClient;

/// BanyanClient - A one stop struct for interacting with our Backend
pub struct BanyanClient {
    /// Our Eth Client for Banyan
    pub eth_client: EthClient,
    /// Our Estuary Client for Banyan
    pub estuary_client: EstuaryClient,
}

impl Default for BanyanClient {
    /// Create a new BanyanClient from the Environment
    fn default() -> Self {
        BanyanClient {
            // Initialize our Eth Client from the environment
            eth_client: EthClient::default(),
            // Initialize our Estuary Client from the environment
            estuary_client: EstuaryClient::default(),

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
            // Initialize our Eth Client
            eth_client: EthClient::new(
                eth_api_url,
                eth_api_key,
                Some(eth_chain_id),
                Some(eth_private_key),
                eth_contract_address,
            )
            .unwrap(),
            // Initialize our Estuary Client
            estuary_client: EstuaryClient::new(estuary_api_hostname, Some(estuary_api_key)),
        }
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
