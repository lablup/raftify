use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;

pub fn parse_peers_json(
    peers: &str,
) -> Result<HashMap<u64, SocketAddr>, Box<dyn std::error::Error>> {
    let peers: Value = serde_json::from_str(peers)?;

    let mut result = HashMap::new();

    if let Value::Object(peer) = peers {
        for (node_id, addr) in peer {
            let key = node_id.parse::<u64>()?;
            let value = SocketAddr::from_str(addr.as_str().unwrap())?;
            result.insert(key, value);
        }
    }

    Ok(result)
}
