use crate::Result;

#[async_trait]
pub trait AbstractStateMachine {
    async fn apply(&mut self, log_entry: Vec<u8>) -> Result<Vec<u8>>;
    async fn snapshot(&self) -> Result<Vec<u8>>;
    async fn restore(&mut self, snapshot: Vec<u8>) -> Result<()>;

    fn encode(&self) -> Result<Vec<u8>>;
    fn decode(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;
}
