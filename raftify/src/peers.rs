use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::ToSocketAddrs};

use super::Peer;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Peers {
    pub inner: HashMap<u64, Peer>,
}

impl Default for Peers {
    fn default() -> Self {
        Self::with_empty()
    }
}

impl Peers {
    pub fn new<A: ToSocketAddrs>(self_id: u64, self_addr: A) -> Self {
        let mut inner = HashMap::new();
        inner.insert(self_id, Peer::new(self_addr));
        Self { inner }
    }

    pub fn with_empty() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn iter(&self) -> SortedPeersIter {
        let mut keys: Vec<_> = self.inner.keys().cloned().collect();
        keys.sort();

        SortedPeersIter {
            keys,
            peers: &self.inner,
            index: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn get(&self, id: &u64) -> Option<&Peer> {
        self.inner.get(id)
    }

    pub fn get_mut(&mut self, id: &u64) -> Option<&mut Peer> {
        self.inner.get_mut(id)
    }

    pub fn remove(&mut self, id: &u64) -> Option<Peer> {
        self.inner.remove(id)
    }

    pub fn add_peer<A: ToSocketAddrs>(&mut self, id: u64, addr: A) {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        let peer = Peer::new(addr);
        self.inner.insert(id, peer);
    }

    pub fn reserve_id(&mut self) -> u64 {
        match self.inner.keys().max() {
            Some(id) => id + 1,
            None => 1,
        }
    }

    pub fn get_node_id_by_addr<A: ToSocketAddrs>(&self, addr: A) -> Option<u64> {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        self.inner
            .iter()
            .find(|(_, peer)| peer.addr == addr)
            .map(|(id, _)| *id)
    }
}

impl<'a> Iterator for SortedPeersIter<'a> {
    type Item = (u64, &'a Peer);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.keys.len() {
            let key = self.keys[self.index];
            self.index += 1;
            self.peers.get(&key).map(|value| (key, value))
        } else {
            None
        }
    }
}

pub struct SortedPeersIter<'a> {
    keys: Vec<u64>,
    peers: &'a HashMap<u64, Peer>,
    index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "invalid socket address")]
    fn test_add_wrong_peer_addr() {
        let mut peers = Peers::with_empty();
        peers.add_peer(1, "wrong peer addr");
    }

    #[test]
    fn test_peers_serial_reserve_peer() {
        let mut peers = Peers::new(1, "127.0.0.1:8081");
        let next_id = peers.reserve_id();
        peers.add_peer(next_id, "127.0.0.1:8082");
        assert_eq!(next_id, 2);

        let next_id = peers.reserve_id();
        peers.add_peer(next_id, "127.0.0.1:8083");
        assert_eq!(next_id, 3);

        let next_id = peers.reserve_id();
        peers.add_peer(next_id, "127.0.0.1:8084");
        assert_eq!(next_id, 4);

        peers.remove(&2);

        let next_id = peers.reserve_id();
        peers.add_peer(next_id, "127.0.0.1:8085");
        assert_eq!(next_id, 5);
    }
}
