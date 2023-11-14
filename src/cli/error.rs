use crate::api::error::ClientError;

pub(crate) struct CliError {
    pub(crate) kind: CliErrorKind,
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
