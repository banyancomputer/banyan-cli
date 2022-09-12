use banyan_shared::eth::*;
use banyan_shared::types::*;

/// BanyanClient - A one stop struct for interacting with our Backend
pub struct BanyanClient {
    /* Eth Stuff */
    /// The Banyan Contract
    pub banyan_contract_address: Address,
    /// The EthProvider to use. This handles all Ethereum interactions.
    pub eth_client: EthClient,
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
    /// The Eth API URL
    pub eth_api_url: String,
    /// The Eth API Key
    pub eth_api_key: String,
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
            eth_api_url: String::from("https://mainnet.infura.io/v3/"),
            eth_api_key: String::from(env::var("ETH_API_KEY").unwrap_or("".to_string())),
            eth_private_key: String::from(env::var("ETH_PRIVATE_KEY").unwrap_or("".to_string())),
            estuary_api_hostname: String::from("http://localhost:3004"),
            estuary_api_key: String::from(""),
        }
    }
}

impl BanyanClientBuilder {
    /// new - Create a new BanyanClientBuilder
    /// # Arguments
    /// * `banyan_contract_address` - The address of the Banyan Contract
    /// * `eth_api_url` - The URL of the Ethereum API to use
    /// * `eth_api_key` - The API Key to use for the Ethereum API
    /// * `eth_private_key` - The private key to use for signing transactions
    /// * `estuary_api_hostname` - The hostname of the Estuary API
    /// * `estuary_api_key` - The API Key to use for the Estuary API
    pub fn new(

    ) -> Self {
     Self {

     }
    }
    pub fn build(self) -> Result<BanyanClient, Error> {
        Ok(BanyanClient {
            banyan_contract_address: self.banyan_contract_address,
            eth_client: EthClient::new(
                self.eth_api_url,
                Some(self.eth_api_key),
                Some(self.eth_private_key),
                Some(10),
            )?,
            estuary_api_hostname: self.estuary_api_hostname,
            estuary_api_key: self.estuary_api_key,
        })
    }
}