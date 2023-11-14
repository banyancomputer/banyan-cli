use std::fmt::Display;
use thiserror::Error;

use crate::api::error::ClientError;

#[derive(Debug, Error)]
pub(crate) struct CliError {
    pub(crate) kind: CliErrorKind,
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl CliError {
    pub(crate) fn client_error(err: ClientError) -> Self {
        Self {
            kind: CliErrorKind::Client(err),
        }
    }
}

pub(crate) enum CliErrorKind {
    Client(ClientError),
}
