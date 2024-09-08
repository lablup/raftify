use bytes::Bytes;
use std::net::ToSocketAddrs;
use tonic::transport::{Channel, Error as TonicError};

use super::RaftServiceClient;

pub async fn create_client<A: ToSocketAddrs>(
    addr: A,
) -> Result<RaftServiceClient<Channel>, TonicError> {
    _create_client(addr, false).await
}

pub async fn create_client_with_ssl<A: ToSocketAddrs>(
    addr: A,
) -> Result<RaftServiceClient<Channel>, TonicError> {
    _create_client(addr, true).await
}

async fn _create_client<A: ToSocketAddrs>(
    addr: A,
    b_ssl: bool,
) -> Result<RaftServiceClient<Channel>, TonicError> {
    let httpx = if b_ssl {
        String::from("https")
    } else {
        String::from("http")
    };
    let addr = addr
        .to_socket_addrs()
        .expect("Invalid socket address format")
        .next()
        .unwrap();
    let url = format!("{}://{}", httpx, addr);
    let url = Bytes::copy_from_slice(url.as_bytes());

    let channel = Channel::from_shared(url).unwrap().connect().await?;
    let client = RaftServiceClient::new(channel);

    Ok(client)
}

// Todo!
#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_create_client() {
        todo!("!!!")
    }
}
