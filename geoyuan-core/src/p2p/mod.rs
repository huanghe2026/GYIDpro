//! P2P networking module
//! 
//! Distributed P2P network using libp2p for wallet sync

pub mod discovery;
pub mod sync;
pub mod protocol;
pub mod swarm;

pub use swarm::{SwarmHandle, SwarmStatus, SwarmCommand, pick_listen_port, start_swarm};
pub use protocol::{Envelope, MessageType, TransferPayload, MintPayload, WalletSyncPayload, WalletSyncResponse, TxSummary, DeviceLinkRequest, DeviceLinkResponse, DeviceListRequest, DeviceListResponse, DeviceInfoEntry};

use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// P2P Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PConfig {
    /// Listen addresses
    pub listen_addresses: Vec<String>,
    
    /// Bootstrap nodes (peer_id@multiaddr)
    pub bootstrap_nodes: Vec<BootstrapNode>,
    
    /// Enable mDNS for local discovery
    pub enable_mdns: bool,
    
    /// Enable Kademlia DHT
    pub enable_kad: bool,
    
    /// Max connections
    pub max_connections: usize,
    
    /// Connection timeout (seconds)
    pub connection_timeout: u64,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/0".to_string(),
            ],
            bootstrap_nodes: vec![
                // Default seed nodes will be added
            ],
            enable_mdns: true,
            enable_kad: true,
            max_connections: 50,
            connection_timeout: 30,
        }
    }
}

/// Bootstrap node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapNode {
    /// Peer ID
    pub peer_id: String,
    
    /// Multiaddr (e.g., /ip4/1.2.3.4/tcp/4001)
    pub address: String,
}

impl BootstrapNode {
    /// Create from multiaddr string (includes peer_id)
    pub fn from_string(multiaddr_with_peer: &str) -> Option<Self> {
        // Format: /ip4/x.x.x.x/tcp/port/p2p/Qm...
        let parts: Vec<&str> = multiaddr_with_peer.split("/p2p/").collect();
        
        if parts.len() == 2 {
            Some(Self {
                peer_id: parts[1].to_string(),
                address: parts[0].to_string(),
            })
        } else {
            None
        }
    }
}

/// Connected peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: String,
    
    /// Remote address
    pub address: String,
    
    /// Is connected
    pub connected: bool,
    
    /// Last seen
    pub last_seen: i64,
    
    /// GeoID if known
    pub geo_id: Option<String>,
}

/// P2P Network state
#[derive(Debug, Clone, Default)]
pub struct NetworkState {
    /// Connected peers
    pub peers: HashMap<PeerId, PeerInfo>,
    
    /// Local peer ID
    pub local_peer_id: Option<PeerId>,
    
    /// Is network running
    pub running: bool,
}

impl NetworkState {
    /// Add connected peer
    pub fn add_peer(&mut self, peer_id: PeerId, address: Multiaddr) {
        self.peers.insert(peer_id, PeerInfo {
            peer_id: peer_id.to_base58(),
            address: address.to_string(),
            connected: true,
            last_seen: chrono::Utc::now().timestamp(),
            geo_id: None,
        });
    }
    
    /// Remove disconnected peer
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            info.connected = false;
        }
    }
    
    /// Get connected peer count
    pub fn connected_count(&self) -> usize {
        self.peers.values().filter(|p| p.connected).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bootstrap_node_parse() {
        let node = BootstrapNode::from_string(
            "/ip4/1.2.3.4/tcp/4001/p2p/QmNnooDu7bfjPFoTceAFiS4fMzo3CXmCSj2wSuAeiQ4dy"
        );
        
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.peer_id, "QmNnooDu7bfjPFoTceAFiS4fMzo3CXmCSj2wSuAeiQ4dy");
        assert_eq!(node.address, "/ip4/1.2.3.4/tcp/4001");
    }
    
    #[test]
    fn test_network_state() {
        let mut state = NetworkState::default();
        assert_eq!(state.connected_count(), 0);
        
        // Add peer
        let peer_id = PeerId::random();
        state.add_peer(peer_id, "/ip4/127.0.0.1/tcp/4001".parse().unwrap());
        assert_eq!(state.connected_count(), 1);
        
        // Remove peer
        state.remove_peer(&peer_id);
        assert_eq!(state.connected_count(), 0);
    }
}
