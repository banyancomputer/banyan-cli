use anyhow::{Result, Error};
use banyan_shared::types::*;
use banyan_shared::deals::*;
use banyan_shared::eth::*;
use serde::{Serialize, Deserialize};

/// DealMaker - Handles Building and Submitting Deals for a Client
#[derive(Serialize, Deserialize, Debug)]
pub struct DealHandler {
    /* Logic Management Structs */
    /// The DealProposalBuilder to use. This constructs DealProposals for us.
    pub deal_proposal_builder: DealProposalBuilder,


    /// The EthProvider to use. This handles all Ethereum interactions.
    #[serde(skip)]
    #[debug(skip)]
    pub eth_provider: EthProvider,

    /* Configuration Structs */
    pub estuary_api_hostname: String,
    pub estuary_api_key: String,
}

impl DealHandler {
    pub fn builder() -> DealMakerBuilder {
        DealMakerBuilder::default()
    }

    /// Submit a new deal to the Estuary API
    ///
    /// # Arguments
    ///
    /// * `file` - The file to submit a deal for
    ///
    /// # Returns
    ///
    /// * `Result<DealId, Error>` - The DealId of the submitted deal
    ///
    /// # Errors
    /// TODO: Add error handling
    pub fn submit_deal(&self, file: std::fs::File) -> Result<DealId, Error> {
        let deal_proposal = self.deal_proposal_builder.build(file)?;
        println!("Submitting Deal Proposal: {:#?}", deal_proposal);
        Err(Error::msg("TODO: Deal Making is not yet implemented"))
    }


    pub fn stage_file(&self, file: std::fs::File) -> Result<u64, Error> {
        Err(Error::msg("TODO: File Staging is not yet implemented"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DealMakerBuilder {

}

impl Default for DealMakerBuilder {
    fn default() -> Self {
        DealMakerBuilder {

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
