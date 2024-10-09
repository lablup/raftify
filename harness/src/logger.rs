use slog::Logger;
use sloggers::{
    file::FileLoggerBuilder,
    types::{Severity, SourceLocation},
    Build,
};

pub fn get_logger(basedir: &str) -> Logger {
    FileLoggerBuilder::new(basedir.to_owned() + "/log")
        .level(Severity::Trace)
        .source_location(SourceLocation::LocalFileAndLine)
        .build()
        .unwrap()
}
