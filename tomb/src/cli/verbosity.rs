use log::LevelFilter;

/// Level of verbosity in debugs
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum MyVerbosity {
    /// Quiet
    Quiet,
    /// Normal
    Normal,
    /// Verbose
    Verbose,
    /// Very Verbose
    VeryVerbose,
    /// Debug
    Debug,
}

impl From<MyVerbosity> for LevelFilter {
    fn from(val: MyVerbosity) -> Self {
        match val {
            MyVerbosity::Quiet => LevelFilter::Off,
            MyVerbosity::Normal => LevelFilter::Info,
            MyVerbosity::Verbose => LevelFilter::Debug,
            MyVerbosity::VeryVerbose => LevelFilter::Trace,
            MyVerbosity::Debug => LevelFilter::Trace,
        }
    }
}
