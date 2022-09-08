use banyan_shared::eth::*;
use banyan_shared::types::*;

/// BanyanClient - A one stop struct for interacting with our Backend
pub struct BanyanClient {
    /* Eth Stuff */
    /// The Banyan Contract
    pub banyan_contract_address: Address,
    /// The EthProvider to use. This handles all Ethereum interactions.
    pub eth_provider: EthProvider,
    /* Estuary Stuff */
    /// The Estuary API Hostname
    pub estuary_api_hostname: String,
    /// The Estuary API Key
    pub estuary_api_key: String,
}

impl BanyanClient {
    pub fn builder() -> BanyanClientBuilder {
        BanyanClientBuilder::default()
    }
}

pub struct BanyanClientBuilder {
    /* Eth Stuff */
    /// The Banyan Contract Address
    pub banyan_contract_address: Address,
    /// The Private key to initialize the Eth Provider with
    pub eth_private_key: String,
    /* Estuary Stuff */
    /// The Estuary API Hostname
    pub estuary_api_hostname: String,
    /// The Estuary API Key
    pub estuary_api_key: String,
}

impl Default for BanyanClientBuilder {
    fn default() -> Self {
        BanyanClientBuilder {
            banyan_contract_address: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            eth_private_key: String::from(""),
            estuary_api_hostname: String::from("http://localhost:3004"),
            estuary_api_key: String::from(""),
        }
    }
}

impl BanyanClientBuilder {
    // TODO: Custimize this to allow for more options
    pub fn new() -> Self {
        BanyanClientBuilder::default()
    }

    pub fn build(self) -> Result<BanyanClient, Error> {
        Ok(BanyanClient {
            banyan_contract_address: self.banyan_contract_address,
            // eth_provider: EthProvider::new()
            estuary_api_hostname: self.estuary_api_hostname,
            estuary_api_key: self.estuary_api_key,
        })
    }
}