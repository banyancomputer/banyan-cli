use anyhow::{Error, Result};
use banyan_shared::{
    estuary::{Content, EstuaryClient},
    eth::*,
    types::*,
};
use ethers::types::Address;

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
    #[allow(dead_code)]
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

    /* Eth Methods */

    /// Propose a Deal for a File
    /// # Arguments
    /// * `dp` - The DealProposal to use for the Deal
    /// # Returns
    /// * `DealID` - The Deal ID of the Deal that was proposed
    pub async fn propose_deal(
        &self,
        dp: DealProposal,
        gas_limit: Option<u64>,
        gas_price: Option<u64>,
    ) -> Result<DealID, Error> {
        // Configurable Gas
        let deal_id = self
            .eth_client
            .propose_deal(dp, gas_limit, gas_price)
            .await?;
        // Return the Deal ID
        Ok(deal_id)
    }

    /// Get an on-chain Deal by its ID
    /// # Arguments
    /// * `deal_id` - The ID of the Deal to get
    /// # Returns
    /// * `onChainDeal` - The on-chain Deal info for the Deal
    pub async fn get_deal(&self, deal_id: DealID) -> Result<OnChainDealInfo, Error> {
        // Get the Deal
        let deal = self.eth_client.get_deal(deal_id).await?;
        // Return the Deal
        Ok(deal)
    }

    /* Estuary Methods */

    /// Stage a File on Estuary
    /// # Arguments
    /// * `file_path` - The path to the file to stage
    /// * `deal_id_str` - The Deal ID of the Deal to stage the file for, as a String
    /// * `b3_hash_str` - The Blake3 Hash Token of the file to stage, as a String
    pub async fn stage_file(
        &self,
        file_path: String,
        deal_id_str: Option<String>,
        b3_hash_str: Option<String>,
    ) -> Result<(), Error> {
        self.estuary_client
            .stage_file(file_path, deal_id_str, b3_hash_str)
            .await
            .unwrap();
        Ok(())
    }

    /// Get all Content stored on Estuary for this Client
    /// # Returns
    /// * `Vec<Content>` - A list of Content stored on Estuary
    pub async fn get_content(&self) -> Result<Vec<Content>, Error> {
        let content = self.estuary_client.get_content().await.unwrap();
        Ok(content)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use banyan_shared::deals::DealProposalBuilder;
    use tokio::fs::File;

    #[test]
    /// Test that we can create a BanyanClient from the Environment
    fn default_client() {
        let client = BanyanClient::default();
        return;
    }
}
