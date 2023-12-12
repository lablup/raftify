use crate::{error::Result, AbstractLogEntry};

#[async_trait]
pub trait AbstractStateMachine<LogEntry: AbstractLogEntry>: Send {
    async fn apply(&mut self, log_entry: LogEntry) -> Result<LogEntry>;

    async fn snapshot(&self) -> Result<Self>
    where
        Self: Sized;

    async fn restore(&mut self, snapshot: Self) -> Result<()>
    where
        Self: Sized;

    fn encode(&self) -> Vec<u8>;
    fn decode(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;
}
