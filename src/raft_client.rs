use std::net::ToSocketAddrs;

use crate::RaftServiceClient;
use bytes::Bytes;
use tonic::transport::Channel;
use tonic::transport::Error as TonicError;

// TODO: Support https schema
pub async fn create_client<A: ToSocketAddrs>(
    addr: &A,
) -> Result<RaftServiceClient<Channel>, TonicError> {
    let addr = addr
        .to_socket_addrs()
        .expect("Wrong socket address")
        .next()
        .unwrap();
    let addr = format!("http://{}", addr.to_string());
    let addr = Bytes::copy_from_slice(addr.as_bytes());

    let channel = Channel::from_shared(addr).unwrap().connect().await?;
    let client = RaftServiceClient::new(channel);

    Ok(client)
}
