use pyo3::{prelude::*, types::PyString};
use raftify::raft::default_logger;
use slog::*;
use slog_async::OverflowStrategy;
use sloggers::{
    file::FileLoggerBuilder,
    types::{Severity, SourceLocation},
    Build,
};
use std::sync::{Arc, Mutex};

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

#[derive(Clone)]
pub enum LoggerMode {
    File,
    Stdout,
}

#[derive(Clone)]
#[pyclass(name = "Logger")]
pub struct PyLogger {
    pub inner: Logger,
    pub mutex: Arc<Mutex<()>>,
    pub mode: LoggerMode,
}

#[pymethods]
impl PyLogger {
    #[new]
    pub fn new() -> Self {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::CompactFormat::new(decorator).build();
        let drain = std::sync::Mutex::new(drain).fuse();

        let logger = slog::Logger::root(drain, o!());

        PyLogger {
            inner: logger,
            mutex: Arc::new(Mutex::new(())),
            mode: LoggerMode::Stdout,
        }
    }

    #[staticmethod]
    pub fn default() -> Self {
        PyLogger {
            inner: default_logger(),
            mutex: Arc::new(Mutex::new(())),
            mode: LoggerMode::Stdout,
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

        PyLogger {
            inner: logger,
            mutex: Arc::new(Mutex::new(())),
            mode: LoggerMode::File,
        }
    }

    pub fn info(&mut self, s: &PyString) {
        let print = || info!(self.inner, "{}", format!("{}", s));

        match self.mode {
            LoggerMode::Stdout => {
                let _guard = self.mutex.lock().unwrap();
                print()
            }
            LoggerMode::File => print(),
        }
    }

    pub fn debug(&mut self, s: &PyString) {
        let print = || debug!(self.inner, "{}", format!("{}", s));

        match self.mode {
            LoggerMode::Stdout => {
                let _guard = self.mutex.lock().unwrap();
                print()
            }
            LoggerMode::File => print(),
        }
    }

    pub fn trace(&mut self, s: &PyString) {
        let print = || trace!(self.inner, "{}", format!("{}", s));

        match self.mode {
            LoggerMode::Stdout => {
                let _guard = self.mutex.lock().unwrap();
                print()
            }
            LoggerMode::File => print(),
        }
    }

    pub fn error(&mut self, s: &PyString) {
        let print = || error!(self.inner, "{}", format!("{}", s));

        match self.mode {
            LoggerMode::Stdout => {
                let _guard = self.mutex.lock().unwrap();
                print()
            }
            LoggerMode::File => print(),
        }
    }

    pub fn crit(&mut self, s: &PyString) {
        let print = || crit!(self.inner, "{}", format!("{}", s));

        match self.mode {
            LoggerMode::Stdout => {
                let _guard = self.mutex.lock().unwrap();
                print()
            }
            LoggerMode::File => print(),
        }
    }
}
