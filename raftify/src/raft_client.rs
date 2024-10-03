use std::net::ToSocketAddrs;
use tonic::transport::{Channel, Error as TonicError};

#[cfg(feature = "tls")]
use tonic::transport::{Certificate, ClientTlsConfig, Identity};

use crate::config::TlsConfig;

use super::RaftServiceClient;

// TODO: Support https schema
pub async fn create_client<A: ToSocketAddrs>(
    addr: A,
    client_tls_config: Option<TlsConfig>,
) -> Result<RaftServiceClient<Channel>, TonicError> {
    let addr = addr.to_socket_addrs().unwrap().next().unwrap();

    let scheme = if client_tls_config.is_some() {
        "https"
    } else {
        "http"
    };

    let addr = format!("{}://{}", scheme, addr);

    #[allow(unused_mut)]
    let mut endpoint = Channel::from_shared(addr).unwrap();

    #[cfg(feature = "tls")]
    if let Some(tls_cfg) = client_tls_config {
        let ca_cert_path = tls_cfg.ca_cert_path.as_ref().unwrap();
        let ca_cert = tokio::fs::read(ca_cert_path).await.unwrap();
        let ca_cert = Certificate::from_pem(ca_cert);

        let domain_name = tls_cfg
            .domain_name
            .unwrap_or_else(|| "localhost".to_string());

        let mut client_tls_config = ClientTlsConfig::new()
            .ca_certificate(ca_cert)
            .domain_name(domain_name);

        // mTLS
        if let (Some(cert_path), Some(key_path)) =
            (tls_cfg.cert_path.clone(), tls_cfg.key_path.clone())
        {
            let cert = tokio::fs::read(cert_path).await.unwrap();
            let key = tokio::fs::read(key_path).await.unwrap();
            let identity = Identity::from_pem(cert, key);
            client_tls_config = client_tls_config.identity(identity);
        }

        endpoint = endpoint.tls_config(client_tls_config)?;
    }

    let channel = endpoint.connect().await?;
    let client = RaftServiceClient::new(channel);

    Ok(client)
}

// TODO: Implement test_create_client
#[cfg(test)]
mod test {}
