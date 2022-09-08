use anyhow::{Result, Error};
use serde::{Serialize, Deserialize};

// TODO: This should all be moved to Banyan-Shared-RS

/// Struct for
///  - Submitting new deals to Chain/Staging
///  - managing Application Defaults/State
#[derive(Serialize, Deserialize, Debug)]
pub struct DealMaker {
    /* ESTUARY CONFIGURATION */
    estuary_api_key: String, // You need an API key to stage on Estuary
    estuary_host: String, // The host of the Estuary API

    /* ETHEREUM CONFIGURATION */
    contract_address: String, // The address of the contract to submit deals too
    signer_address: String, // The address of the signer (you)

    /* DEAL CONFIGURATION */
    pub executor_address: String, // The address of the executor
    pub deal_length_in_blocks: u32, // How long the deal should last
    pub proof_frequency_in_blocks: u32, // How often the executor should submit proofs
    pub bounty_per_tib: f64, // How much to pay the executor per TiB
    pub collateral_per_tib: f64, // How much collateral to put up per TiB
    pub erc20_token_denomination: String, // The ERC20 token to use for collateral/bounty
}

/// `DealMaker` implements `Default`
impl ::std::default::Default for DealMaker {
    fn default() -> Self { Self {
        // Estuary Configuration
        estuary_api_key: String::from("***"),
        estuary_host: String::from("localhost:3004"), // TODO: Change this to the actual Estuary host

        // Ethereum Configuration
        contract_address: String::from("0x595481A61df02A716b829411daD9838578d10072"), // TODO: Change this to the actual contract address
        signer_address: String::from("0x0000000000000000000000000000000000000000"), // TODO: Change this to the actual signer address

        // Deal Configuration
        executor_address: String::from("0x0000000000000000000000000000000000000000"), // TODO: Change this to the actual executor address
        deal_length_in_blocks: 1000, // TODO: This should be updated to be ~1 year long
        proof_frequency_in_blocks: 100, // TODO: This should be updated to right
        bounty_per_tib: 10.0,
        collateral_per_tib: 0.1,
        erc20_token_denomination: String::from("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"), // USDC
    }}
}

/// Struct for holding what information is needed to submit a deal
#[derive(Serialize, Deserialize, Debug)]
pub struct DealProposal {
    // File State
    pub file_path: String, // The path to the file to submit a deal for
    // Chain Relevant Information
    pub executor_address: String, // The address of the executor
    pub deal_length_in_blocks: u32, // How long the deal should last
    pub proof_frequency_in_blocks: u32, // How often the executor should submit proofs
    pub bounty: u32, // An Int representing the bounty in the ERC20 token
    pub collateral: u32, // An Int representing the collateral in the ERC20 token
    pub erc20_token_denomination: String, // The ERC20 token to use for collateral/bounty
    pub file_size: u32, // The size of the file in bytes
    pub file_cid: String, // The CID of the file
    pub file_blake3: u32, // The Blake3 hash of the file
}

/// `DealMaker` implements
/// - `DealMaker::new()`
/// - `DealMaker::DealProposal()`
/// - `DealMaker::submit_deal()`
impl DealMaker {
    /// Create a new `DealMaker`
    pub fn new() -> Self {
        Self::default()
    }

    /// Submit a new deal to the Estuary API
    pub fn submit_deal(&self, file_path: String) -> Result<u64, Error> {
        // TODO: Implement Deal Making
        let deal_proposal = self.deal_proposal(file_path);
        println!("Submitting Deal Proposal: {:#?}", deal_proposal);
        // let deal_id = self.submit_deal_to_chain(deal_proposal)?;
        Err(Error::msg("Deal Making is not yet implemented"))
    }

    pub fn stage_file(&self, file_path: String) -> Result<u64, Error> {
        // TODO: Implement File Staging
        Err(Error::msg("File Staging is not yet implemented"))
    }

    /// Create a new `DealProposal` from the `DealMaker`
    fn deal_proposal(&self, file_path: String) -> DealProposal {
        // Check if the file exists
        if !std::path::Path::new(&file_path).exists() {
            println!("File does not exist: {}", file_path);
            std::process::exit(1);
        }
        // Get the size of the file in TiB
        let file_size = std::fs::metadata(&file_path).unwrap().len() as u32;
        // Determine how many TiB the file is
        let num_tib: f64 = file_size as f64 / (1024^4) as f64;
        // Get the CID of the file
        let file_cid = "TODO: Get the CID of the file";
        // Get the Blake3 hash of the file
        let file_blake3 = 1234; // TODO: Get the Blake3 hash of the file

        DealProposal {
            // Record the file path
            file_path: file_path.clone(),
            // Chain Relevant Information
            executor_address: self.executor_address.clone(),
            deal_length_in_blocks: self.deal_length_in_blocks,
            proof_frequency_in_blocks: self.proof_frequency_in_blocks,
            // The bounty is the bounty per TiB * the size of the file in TiB * 10^18
            bounty: (self.bounty_per_tib * num_tib * ((10^18) as f64)).round() as u32,
            // Same for the collateral
            collateral: (self.collateral_per_tib * num_tib * ((10^18) as f64)).round() as u32,
            erc20_token_denomination: self.erc20_token_denomination.clone(),
            file_size,
            file_cid: file_cid.to_string(),
            file_blake3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deal_proposal() {
        println!("TODO: Implement Deal Proposal Tests");
    }
}
