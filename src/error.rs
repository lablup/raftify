use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Raft error: cause:`{0}`")]
    RaftError(#[from] raft::Error),
    #[error("Error joining the cluster")]
    JoinError,
    #[error("Request rejected, cause: `{0}`")]
    Rejected(String),

    #[error("Request timeout")]
    Timeout,
    #[error("gRPC error: `{0}`")]
    Grpc(#[from] tonic::transport::Error),
    #[error("Error calling remote procedure: `{0}`")]
    RemoteCall(#[from] tonic::Status),

    #[error("IO error: {0}")]
    Io(#[from] tokio::io::Error),
    #[error("Storage error: `{0}`")]
    Database(#[from] heed::Error),
    #[error("Unexpected error")]
    Other(#[source] Box<dyn std::error::Error + Sync + Send + 'static>),
    #[error("Unknown error")]
    Unknown,

    #[error("Encoding error")]
    EncodingError(String),
    #[error("Decoding error")]
    DecodingError(String),
}

#[derive(Debug, ThisError)]
pub enum SendMessageError {
    #[error("Failed to connect to node {0}")]
    ConnectionError(String),
    #[error("Peer not found node {0}")]
    PeerNotFound(String),
    #[error("Failed to send message to node {0}")]
    TransmissionError(String),
}

impl From<prost::DecodeError> for Error {
    fn from(e: prost::DecodeError) -> Self {
        Self::Other(Box::new(e))
    }
}

impl From<bincode::Error> for Error {
    fn from(e: bincode::Error) -> Self {
        Self::Other(e)
    }
}
