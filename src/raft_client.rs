use bytes::Bytes;
use std::net::ToSocketAddrs;
use tonic::transport::{Channel, Error as TonicError};

use super::RaftServiceClient;

// TODO: Support https schema
pub async fn create_client<A: ToSocketAddrs>(
    addr: A,
) -> Result<RaftServiceClient<Channel>, TonicError> {
    let addr = addr
        .to_socket_addrs()
        .expect("Invalid socket address format")
        .next()
        .unwrap();
    let addr = format!("http://{}", addr);
    let addr = Bytes::copy_from_slice(addr.as_bytes());

    let channel = Channel::from_shared(addr).unwrap().connect().await?;
    let client = RaftServiceClient::new(channel);

    Ok(client)
}
