use std::{net::ToSocketAddrs};

use tonic::transport::Channel;
use bytes::Bytes;
use crate::RaftServiceClient;

pub async fn create_client<A: ToSocketAddrs>(addr: A) -> Result<RaftServiceClient<Channel>, Box<dyn std::error::Error>> {
    let addr = addr.to_socket_addrs()?.next().unwrap();
    let addr = addr.to_string();
    let addr = Bytes::copy_from_slice(addr.as_bytes());

    let channel = Channel::from_shared(addr)?.connect().await?;
    let client = RaftServiceClient::new(channel);

    Ok(client)
}
