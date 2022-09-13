use banyan_shared::eth::*;
use banyan_shared::types::*;
use anyhow::{Result, Error};
use std::env::var;
use ethers::types::Address;
use blake3::Hasher;

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
            estuary_api_key: var("ESTUARY_API_KEY")
                .unwrap_or("".to_string()),
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
                Some(eth_contract_address),
            ),
            estuary_api_hostname: estuary_api_hostname.unwrap_or("http://localhost:3004".to_string()),
            estuary_api_key: estuary_api_key.unwrap_or("".to_string()),
        }
    }

    /// Submit a Deal Proposal to the Banyan Contract and Update the Estuary API
    /// # Arguments
    /// * `deal_proposal` - The DealProposal to submit
    /// # Returns
    /// * `Result<DealId, Error>` - The DealProposal that was submitted
    /// # Errors
    /// * `Error` - If there was an error submitting the DealProposal
    /// * `Error` - If there was an error updating the Estuary API
    pub async fn submit_deal_proposal(&self, deal_proposal: DealProposal) -> Result<DealId, Error> {

        // Submit the Deal Proposal to the Banyan Contract
        // let deal_id: DealID = self.eth_client.propose_deal(deal_proposal).await?;
        let deal_id = DealId(0);


        // Update the Estuary API
        self.update_estuary_api(deal_id).await?;
        // Return the Deal ID
        Ok(deal_id)
    }
}
