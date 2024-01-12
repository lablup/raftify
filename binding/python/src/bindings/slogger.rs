use pyo3::{prelude::*, types::PyString};
use raftify::raft::{default_logger, logger::Logger};
use slog::{crit, debug, error, info, o, trace, warn, Logger as Slogger};
use slog_async::OverflowStrategy;
use sloggers::{
    file::FileLoggerBuilder,
    types::{Severity, SourceLocation},
    Build,
};

#[pyclass(name = "OverflowStrategy")]
pub struct PyOverflowStrategy(pub OverflowStrategy);

impl From<OverflowStrategy> for PyOverflowStrategy {
    fn from(x: OverflowStrategy) -> Self {
        match x {
            OverflowStrategy::Block => PyOverflowStrategy(OverflowStrategy::Block),
            OverflowStrategy::Drop => PyOverflowStrategy(OverflowStrategy::Drop),
            OverflowStrategy::DropAndReport => PyOverflowStrategy(OverflowStrategy::DropAndReport),
            _ => todo!(),
        }
    }
}

#[pymethods]
impl PyOverflowStrategy {
    pub fn __hash__(&self) -> u64 {
        self.0 as u64
    }

    pub fn __repr__(&self) -> String {
        match self.0 {
            OverflowStrategy::Block => "Block".to_string(),
            OverflowStrategy::Drop => "Drop".to_string(),
            OverflowStrategy::DropAndReport => "DropAndReport".to_string(),
            _ => todo!(),
        }
    }

    #[classattr]
    pub fn Block() -> Self {
        PyOverflowStrategy(OverflowStrategy::Block)
    }

    #[classattr]
    pub fn Drop() -> Self {
        PyOverflowStrategy(OverflowStrategy::Drop)
    }

    #[classattr]
    pub fn DropAndReport() -> Self {
        PyOverflowStrategy(OverflowStrategy::DropAndReport)
    }
}

#[derive(Clone, Debug)]
#[pyclass(name = "Slogger")]
pub struct PySlogger {
    pub inner: Slogger,
}

impl Logger for PySlogger {
    fn info(&self, s: &str) {
        info!(self.inner, "{}", s);
    }

    fn debug(&self, s: &str) {
        debug!(self.inner, "{}", s);
    }

    fn trace(&self, s: &str) {
        trace!(self.inner, "{}", s);
    }

    fn error(&self, s: &str) {
        error!(self.inner, "{}", s);
    }

    fn warn(&self, s: &str) {
        warn!(self.inner, "{}", s);
    }

    fn fatal(&self, s: &str) {
        crit!(self.inner, "{}", s);
    }
}

#[pymethods]
impl PySlogger {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: Slogger::root(slog::Discard, o!()),
        }
    }

    #[staticmethod]
    pub fn default() -> Self {
        PySlogger {
            inner: default_logger(),
        }
    }

    #[staticmethod]
    pub fn new_file_logger(
        log_path: &PyString,
        chan_size: usize,
        rotate_size: u64,
        rotate_keep: usize,
    ) -> Self {
        let log_path = log_path.to_str().unwrap();

        let logger = FileLoggerBuilder::new(log_path)
            // TODO: Implement this
            .level(Severity::Debug)
            .source_location(SourceLocation::LocalFileAndLine)
            .channel_size(chan_size)
            .rotate_size(rotate_size)
            .rotate_keep(rotate_keep)
            // TODO: Implement this
            // .overflow_strategy(overflow_strategy.0)
            .build()
            .unwrap();

        PySlogger { inner: logger }
    }

    fn info(&self, s: &str) {
        info!(self.inner, "{}", s);
    }

    fn debug(&self, s: &str) {
        debug!(self.inner, "{}", s);
    }

    fn trace(&self, s: &str) {
        trace!(self.inner, "{}", s);
    }

    fn error(&self, s: &str) {
        error!(self.inner, "{}", s);
    }

    fn warn(&self, s: &str) {
        warn!(self.inner, "{}", s);
    }

    fn fatal(&self, s: &str) {
        crit!(self.inner, "{}", s);
    }
}
