pub trait AbstractLogEntry: Clone + Send + Sync {
    fn encode(&self) -> Vec<u8>;
    fn decode(bytes: &[u8]) -> Self
    where
        Self: Sized;
}
