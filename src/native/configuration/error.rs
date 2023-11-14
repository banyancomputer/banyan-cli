use std::fmt::Display;

pub(crate) struct ConfigurationError {
    pub(crate) kind: ConfigurationErrorKind,
}

impl ConfigurationError {
    pub(crate) fn missing_credentials() -> Self {
        Self {
            kind: ConfigurationErrorKind::MissingCredentials,
        }
    }

    pub(crate) fn missing_identifier() -> Self {
        Self {
            kind: ConfigurationErrorKind::MissingIdentifier,
        }
    }

    pub(crate) fn missing_local_drive() -> Self {
        Self {
            kind: ConfigurationErrorKind::MissingLocalDrive,
        }
    }

    pub(crate) fn missing_remote_drive() -> Self {
        Self {
            kind: ConfigurationErrorKind::MissingRemoteDrive,
        }
    }

    pub(crate) fn unique_error() -> Self {
        Self {
            kind: ConfigurationErrorKind::UniqueDriveError,
        }
    }
}

impl Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args = match self.kind {
            ConfigurationErrorKind::MissingCredentials => todo!(),
            ConfigurationErrorKind::MissingIdentifier => {
                format_args!("Unable to find a remote Identifier associated with that Drive")
            }
            ConfigurationErrorKind::MissingLocalDrive => {
                format_args!("Unable to find a local Drive with that query")
            }
            ConfigurationErrorKind::MissingRemoteDrive => {
                format_args!("Unable to find a remote Drive with that query")
            }
            ConfigurationErrorKind::UniqueDriveError => {
                format_args!("There is already a unique Drive with these specs")
            }
        };
        f.write_fmt(args)
    }
}

enum ConfigurationErrorKind {
    MissingCredentials,
    MissingIdentifier,
    MissingLocalDrive,
    MissingRemoteDrive,
    UniqueDriveError,
}
