use crate::error::Result;

#[async_trait]
pub trait AbstractStateMachine {
    async fn apply(&mut self, message: &[u8]) -> Result<Vec<u8>>;
    async fn snapshot(&self) -> Result<Vec<u8>>;
    async fn restore(&mut self, snapshot: &[u8]) -> Result<()>;
}
