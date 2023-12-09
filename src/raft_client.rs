use std::net::ToSocketAddrs;

use crate::RaftServiceClient;
use crate::Result;
use bytes::Bytes;
use tonic::transport::Channel;

// TODO: Support https schema
pub async fn create_client<A: ToSocketAddrs>(addr: A) -> Result<RaftServiceClient<Channel>> {
    let addr = addr.to_socket_addrs()?.next().unwrap();
    let addr = format!("http://{}", addr.to_string());
    let addr = Bytes::copy_from_slice(addr.as_bytes());

    let channel = Channel::from_shared(addr).unwrap().connect().await?;
    let client = RaftServiceClient::new(channel);

    Ok(client)
}
