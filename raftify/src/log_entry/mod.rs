use crate::Result;

pub trait AbstractLogEntry: Clone + Send + Sync {
    fn encode(&self) -> Result<Vec<u8>>;
    fn decode(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;
}
