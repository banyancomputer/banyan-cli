use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum MetadataRequest {
    Create,
    Get,
    Delete,
}
