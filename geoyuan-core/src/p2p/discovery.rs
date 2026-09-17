//! Node discovery module
//! 
//! Peer discovery using libp2p mDNS and Kademlia DHT

use libp2p::{Multiaddr, PeerId};
use libp2p::kad::QueryId;
use std::collections::HashMap;

/// Discovery event types
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// New peer discovered
    PeerDiscovered(PeerId, Multiaddr),
    /// Peer connected
    PeerConnected(PeerId),
    /// Peer disconnected
    PeerDisconnected(PeerId),
    /// Query completed
    QueryCompleted(QueryId, Vec<(PeerId, Multiaddr)>),
    /// Error occurred
    Error(String),
}

/// Discovery service
pub struct DiscoveryService {
    peers: HashMap<PeerId, Multiaddr>,
}

impl DiscoveryService {
    /// Create new discovery service
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }
    
    /// Add discovered peer
    pub fn add_peer(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.peers.insert(peer_id, addr);
    }
    
    /// Remove peer
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.remove(peer_id);
    }
    
    /// Get all known peers
    pub fn get_peers(&self) -> Vec<(PeerId, Multiaddr)> {
        self.peers.iter()
            .map(|(id, addr)| (*id, addr.clone()))
            .collect()
    }
    
    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
    
    /// Handle newly discovered peers (from mDNS or other sources)
    pub fn handle_discovered(&mut self, peers: Vec<(PeerId, Multiaddr)>) -> Vec<DiscoveryEvent> {
        peers.into_iter()
            .map(|(peer_id, addr)| {
                self.add_peer(peer_id, addr.clone());
                DiscoveryEvent::PeerDiscovered(peer_id, addr)
            })
            .collect()
    }
    
    /// Handle expired peers
    pub fn handle_expired(&mut self, peers: Vec<(PeerId, Multiaddr)>) {
        for (peer_id, _) in peers {
            self.remove_peer(&peer_id);
        }
    }
    
    /// Handle Kademlia closest peers result
    pub fn handle_closest_peers(
        &self,
        query_id: QueryId,
        peer_ids: Vec<PeerId>,
    ) -> DiscoveryEvent {
        let results: Vec<(PeerId, Multiaddr)> = peer_ids.into_iter()
            .filter_map(|peer_id| {
                self.peers.get(&peer_id).map(|addr| (peer_id, addr.clone()))
            })
            .collect();
        DiscoveryEvent::QueryCompleted(query_id, results)
    }
}

impl Default for DiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_discovery_service() {
        let mut svc = DiscoveryService::new();
        assert_eq!(svc.peer_count(), 0);
        
        let peer_id = PeerId::random();
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        
        svc.add_peer(peer_id, addr.clone());
        assert_eq!(svc.peer_count(), 1);
        
        svc.remove_peer(&peer_id);
        assert_eq!(svc.peer_count(), 0);
    }
}
