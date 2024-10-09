use slog::Logger;
use sloggers::{
    file::FileLoggerBuilder,
    types::{Severity, SourceLocation},
    Build,
};

pub fn build_file_logger(basedir: &str) -> Logger {
    FileLoggerBuilder::new(basedir.to_owned() + "/cluster_log.txt")
        .level(Severity::Trace)
        .source_location(SourceLocation::LocalFileAndLine)
        .build()
        .unwrap()
}
