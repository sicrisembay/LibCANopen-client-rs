// NMT (Network Management) implementation
// Handles node state management, NMT commands, and heartbeat monitoring

use crate::canopen::message::CanMessage;
use crate::{CANopenError, Result};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

/// NMT (Network Management) state values as defined by CANopen standard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmtState {
    /// Initial state after power-on (0x00)
    Boot = 0x00,
    /// Node stopped - not processing PDOs (0x04)
    Stopped = 0x04,
    /// Node operational - full functionality (0x05)
    Operational = 0x05,
    /// Node pre-operational - SDO only, no PDOs (0x7F)
    PreOperational = 0x7F,
    /// Invalid/unknown state (0xFF)
    Invalid = 0xFF,
}

impl From<u8> for NmtState {
    fn from(value: u8) -> Self {
        match value {
            0x00 => NmtState::Boot,
            0x04 => NmtState::Stopped,
            0x05 => NmtState::Operational,
            0x7F => NmtState::PreOperational,
            _ => NmtState::Invalid,
        }
    }
}

impl std::fmt::Display for NmtState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NmtState::Boot => write!(f, "Boot"),
            NmtState::Stopped => write!(f, "Stopped"),
            NmtState::Operational => write!(f, "Operational"),
            NmtState::PreOperational => write!(f, "Pre-Operational"),
            NmtState::Invalid => write!(f, "Invalid"),
        }
    }
}

/// NMT command codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmtCommand {
    /// Start remote node - transition to Operational (0x01)
    StartRemoteNode = 0x01,
    /// Stop remote node - transition to Stopped (0x02)
    StopRemoteNode = 0x02,
    /// Enter pre-operational state (0x80)
    EnterPreOperational = 0x80,
    /// Reset node - full device reset (0x81)
    ResetNode = 0x81,
    /// Reset communication - reset communication parameters only (0x82)
    ResetCommunication = 0x82,
}

/// State information for a single CANopen node
#[derive(Debug, Clone)]
pub struct NodeState {
    /// Current NMT state
    pub state: NmtState,
    /// Previous NMT state (for detecting transitions)
    pub last_state: NmtState,
    /// Timestamp of last received heartbeat
    pub last_heartbeat: SystemTime,
    /// Node ID (1-127)
    pub node_id: u8,
}

impl NodeState {
    /// Create a new node state tracker for the specified node ID
    pub fn new(node_id: u8) -> Self {
        Self {
            state: NmtState::Invalid,
            last_state: NmtState::Invalid,
            last_heartbeat: SystemTime::now(),
            node_id,
        }
    }

    /// Update node state from heartbeat and return true if state changed
    pub fn update_state(&mut self, new_state: NmtState) -> bool {
        self.last_heartbeat = SystemTime::now();

        if new_state != self.state {
            self.last_state = self.state;
            self.state = new_state;
            true
        } else {
            false
        }
    }

    /// Check if heartbeat has timed out
    pub fn is_timeout(&self, timeout: Duration) -> bool {
        if let Ok(elapsed) = self.last_heartbeat.elapsed() {
            elapsed > timeout
        } else {
            false
        }
    }
}

/// NMT Manager - handles node state management and NMT commands
pub struct NmtManager {
    /// Per-node state tracking (node_id -> NodeState)
    nodes: HashMap<u8, NodeState>,
    /// Channel for sending CAN messages
    message_sender: mpsc::Sender<CanMessage>,
}

impl NmtManager {
    /// Create a new NMT manager
    ///
    /// # Arguments
    /// * `message_sender` - Channel for sending CAN messages to hardware
    pub fn new(message_sender: mpsc::Sender<CanMessage>) -> Self {
        let mut nodes = HashMap::new();

        // Pre-allocate state trackers for all possible nodes (1-127)
        for node_id in 1..=127 {
            nodes.insert(node_id, NodeState::new(node_id));
        }

        Self {
            nodes,
            message_sender,
        }
    }

    /// Process an incoming heartbeat message (COB-ID 0x700 + node_id)
    ///
    /// # Arguments
    /// * `message` - The heartbeat CAN message
    ///
    /// # Returns
    /// * `Ok(true)` if state changed
    /// * `Ok(false)` if state unchanged
    /// * `Err` on error
    pub async fn process_heartbeat(&mut self, message: &CanMessage) -> Result<bool> {
        // Heartbeat messages are at COB-ID 0x700 + node_id
        let cob_id = message.id.raw();

        if !(0x700..=0x77F).contains(&cob_id) {
            return Ok(false); // Not a heartbeat message
        }

        let node_id = (cob_id - 0x700) as u8;

        if message.data.is_empty() {
            return Err(CANopenError::InvalidMessage);
        }

        let state_byte = message.data[0];
        let new_state = NmtState::from(state_byte);

        if let Some(node_state) = self.nodes.get_mut(&node_id) {
            let state_changed = node_state.update_state(new_state);

            if state_changed {
                log::info!(
                    "Node {} state changed: {} -> {}",
                    node_id,
                    node_state.last_state,
                    node_state.state
                );
            }

            Ok(state_changed)
        } else {
            // Node ID out of range
            log::warn!("Received heartbeat from invalid node ID: {}", node_id);
            Ok(false)
        }
    }

    /// Get the current state of a node
    pub fn get_node_state(&self, node_id: u8) -> Option<&NodeState> {
        self.nodes.get(&node_id)
    }

    /// Check if a node has been discovered (not in Invalid state)
    pub fn is_node_found(&self, node_id: u8) -> bool {
        self.nodes
            .get(&node_id)
            .map(|node| node.state != NmtState::Invalid)
            .unwrap_or(false)
    }

    /// Check if a node's heartbeat is within the timeout period
    ///
    /// # Arguments
    /// * `node_id` - The node to check
    /// * `timeout` - Maximum time since last heartbeat
    pub fn check_heartbeat(&self, node_id: u8, timeout: Duration) -> bool {
        self.nodes
            .get(&node_id)
            .map(|node| !node.is_timeout(timeout))
            .unwrap_or(false)
    }

    /// Send an NMT command to a specific node or broadcast (node_id = 0)
    ///
    /// # Arguments
    /// * `command` - The NMT command to send
    /// * `node_id` - Target node ID (0 = broadcast to all nodes)
    async fn send_nmt_command(&self, command: NmtCommand, node_id: u8) -> Result<()> {
        let message = CanMessage::new(
            0x000, // NMT commands use COB-ID 0x000
            vec![command as u8, node_id],
        )?;

        self.message_sender
            .send(message)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        log::debug!("Sent NMT command {:?} to node {}", command, node_id);
        Ok(())
    }

    /// Start a remote node (transition to Operational state)
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn start_node(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(NmtCommand::StartRemoteNode, node_id)
            .await
    }

    /// Stop a remote node (transition to Stopped state)
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn stop_node(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(NmtCommand::StopRemoteNode, node_id)
            .await
    }

    /// Put a node into Pre-Operational state
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn enter_pre_operational(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(NmtCommand::EnterPreOperational, node_id)
            .await
    }

    /// Reset a node (full device reset)
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn reset_node(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(NmtCommand::ResetNode, node_id).await
    }

    /// Reset communication parameters of a node
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn reset_communication(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(NmtCommand::ResetCommunication, node_id)
            .await
    }

    /// Get a list of all discovered nodes (nodes not in Invalid state)
    pub fn get_discovered_nodes(&self) -> Vec<u8> {
        self.nodes
            .iter()
            .filter(|(_, state)| state.state != NmtState::Invalid)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get a list of all nodes in a specific state
    pub fn get_nodes_in_state(&self, target_state: NmtState) -> Vec<u8> {
        self.nodes
            .iter()
            .filter(|(_, state)| state.state == target_state)
            .map(|(id, _)| *id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canopen::message::NmtCommand as MsgNmtCommand;

    #[test]
    fn test_nmt_state_from_u8() {
        // Test all valid NMT states
        assert_eq!(NmtState::from(0x00), NmtState::Boot);
        assert_eq!(NmtState::from(0x04), NmtState::Stopped);
        assert_eq!(NmtState::from(0x05), NmtState::Operational);
        assert_eq!(NmtState::from(0x7F), NmtState::PreOperational);

        // Test invalid states
        assert_eq!(NmtState::from(0x01), NmtState::Invalid);
        assert_eq!(NmtState::from(0xFF), NmtState::Invalid);
        assert_eq!(NmtState::from(0x80), NmtState::Invalid);
    }

    #[test]
    fn test_nmt_state_display() {
        assert_eq!(format!("{}", NmtState::Boot), "Boot");
        assert_eq!(format!("{}", NmtState::Stopped), "Stopped");
        assert_eq!(format!("{}", NmtState::Operational), "Operational");
        assert_eq!(format!("{}", NmtState::PreOperational), "Pre-Operational");
        assert_eq!(format!("{}", NmtState::Invalid), "Invalid");
    }

    #[test]
    fn test_nmt_state_equality() {
        assert_eq!(NmtState::Boot, NmtState::Boot);
        assert_eq!(NmtState::Operational, NmtState::Operational);
        assert_ne!(NmtState::Boot, NmtState::Operational);
        assert_ne!(NmtState::Stopped, NmtState::PreOperational);
    }

    #[test]
    fn test_nmt_command_values() {
        assert_eq!(NmtCommand::StartRemoteNode as u8, 0x01);
        assert_eq!(NmtCommand::StopRemoteNode as u8, 0x02);
        assert_eq!(NmtCommand::EnterPreOperational as u8, 0x80);
        assert_eq!(NmtCommand::ResetNode as u8, 0x81);
        assert_eq!(NmtCommand::ResetCommunication as u8, 0x82);
    }

    #[test]
    fn test_node_state_creation() {
        let node = NodeState::new(5);

        assert_eq!(node.node_id, 5);
        assert_eq!(node.state, NmtState::Invalid);
        assert_eq!(node.last_state, NmtState::Invalid);
    }

    #[test]
    fn test_node_state_update() {
        let mut node = NodeState::new(10);

        // Initial state
        assert_eq!(node.state, NmtState::Invalid);

        // Update to Boot
        node.update_state(NmtState::Boot);
        assert_eq!(node.state, NmtState::Boot);
        assert_eq!(node.last_state, NmtState::Invalid);

        // Update to Operational
        node.update_state(NmtState::Operational);
        assert_eq!(node.state, NmtState::Operational);
        assert_eq!(node.last_state, NmtState::Boot);

        // Update to Stopped
        node.update_state(NmtState::Stopped);
        assert_eq!(node.state, NmtState::Stopped);
        assert_eq!(node.last_state, NmtState::Operational);
    }

    #[test]
    fn test_node_state_transitions() {
        let mut node = NodeState::new(1);

        // Boot → Pre-Operational
        node.update_state(NmtState::Boot);
        node.update_state(NmtState::PreOperational);
        assert_eq!(node.state, NmtState::PreOperational);
        assert_eq!(node.last_state, NmtState::Boot);

        // Pre-Operational → Operational
        node.update_state(NmtState::Operational);
        assert_eq!(node.state, NmtState::Operational);
        assert_eq!(node.last_state, NmtState::PreOperational);

        // Operational → Stopped
        node.update_state(NmtState::Stopped);
        assert_eq!(node.state, NmtState::Stopped);
        assert_eq!(node.last_state, NmtState::Operational);

        // Stopped → Pre-Operational
        node.update_state(NmtState::PreOperational);
        assert_eq!(node.state, NmtState::PreOperational);
        assert_eq!(node.last_state, NmtState::Stopped);
    }

    #[test]
    fn test_heartbeat_message_parsing() {
        // Heartbeat from node 5 in Operational state
        let msg = CanMessage::new(0x705, vec![0x05]).unwrap();
        assert_eq!(msg.node_id(), Some(5));
        assert_eq!(msg.nmt_state(), Some(NmtState::Operational));

        // Heartbeat from node 1 in Boot state
        let msg = CanMessage::new(0x701, vec![0x00]).unwrap();
        assert_eq!(msg.node_id(), Some(1));
        assert_eq!(msg.nmt_state(), Some(NmtState::Boot));

        // Heartbeat from node 127 in Stopped state
        let msg = CanMessage::new(0x77F, vec![0x04]).unwrap();
        assert_eq!(msg.node_id(), Some(127));
        assert_eq!(msg.nmt_state(), Some(NmtState::Stopped));

        // Heartbeat from node 42 in Pre-Operational state
        let msg = CanMessage::new(0x72A, vec![0x7F]).unwrap();
        assert_eq!(msg.node_id(), Some(42));
        assert_eq!(msg.nmt_state(), Some(NmtState::PreOperational));
    }

    #[test]
    fn test_nmt_command_message_format() {
        // NMT messages have COB-ID 0x000 and 2 bytes: [command, node_id]

        // Start node 5
        let msg = CanMessage::new(0x000, vec![0x01, 0x05]).unwrap();
        assert_eq!(msg.nmt_command(), Some(MsgNmtCommand::StartRemoteNode));

        // Stop node 10
        let msg = CanMessage::new(0x000, vec![0x02, 0x0A]).unwrap();
        assert_eq!(msg.nmt_command(), Some(MsgNmtCommand::StopRemoteNode));

        // Enter Pre-Operational (all nodes)
        let msg = CanMessage::new(0x000, vec![0x80, 0x00]).unwrap();
        assert_eq!(msg.nmt_command(), Some(MsgNmtCommand::EnterPreOperational));

        // Reset node 1
        let msg = CanMessage::new(0x000, vec![0x81, 0x01]).unwrap();
        assert_eq!(msg.nmt_command(), Some(MsgNmtCommand::ResetNode));

        // Reset communication for all nodes
        let msg = CanMessage::new(0x000, vec![0x82, 0x00]).unwrap();
        assert_eq!(msg.nmt_command(), Some(MsgNmtCommand::ResetCommunication));
    }

    #[test]
    fn test_nmt_broadcast_vs_unicast() {
        // Broadcast to all nodes (node_id = 0)
        let broadcast = CanMessage::new(0x000, vec![0x01, 0x00]).unwrap();
        assert_eq!(broadcast.data[1], 0x00); // Node ID 0 means all nodes

        // Unicast to specific node
        let unicast = CanMessage::new(0x000, vec![0x01, 0x05]).unwrap();
        assert_eq!(unicast.data[1], 0x05); // Specific node 5
    }

    #[test]
    fn test_heartbeat_cob_id_calculation() {
        // Heartbeat COB-ID = 0x700 + node_id
        for node_id in 1..=127 {
            let cob_id = 0x700 + node_id as u16;
            let msg = CanMessage::new(cob_id, vec![0x05]).unwrap();
            assert_eq!(msg.node_id(), Some(node_id));
        }
    }

    #[test]
    fn test_invalid_heartbeat_messages() {
        // Heartbeat with no data (should be exactly 1 byte)
        let msg = CanMessage::new_unchecked(0x701, vec![]);
        assert!(!msg.is_valid());

        // Heartbeat with too much data
        let msg = CanMessage::new_unchecked(0x701, vec![0x05, 0x00]);
        assert!(!msg.is_valid());
    }

    #[test]
    fn test_invalid_nmt_command_messages() {
        // NMT command with wrong data length (should be exactly 2 bytes)
        let msg = CanMessage::new_unchecked(0x000, vec![0x01]);
        assert!(!msg.is_valid());

        let msg = CanMessage::new_unchecked(0x000, vec![0x01, 0x05, 0x00]);
        assert!(!msg.is_valid());

        // Valid NMT command
        let msg = CanMessage::new(0x000, vec![0x01, 0x05]).unwrap();
        assert!(msg.is_valid());
    }

    #[test]
    fn test_nmt_state_machine_typical_flow() {
        let mut node = NodeState::new(1);

        // Typical power-on sequence
        node.update_state(NmtState::Boot);
        assert_eq!(node.state, NmtState::Boot);

        // Automatic transition to Pre-Operational after boot
        node.update_state(NmtState::PreOperational);
        assert_eq!(node.state, NmtState::PreOperational);

        // Master starts the node
        node.update_state(NmtState::Operational);
        assert_eq!(node.state, NmtState::Operational);

        // Emergency stop
        node.update_state(NmtState::Stopped);
        assert_eq!(node.state, NmtState::Stopped);

        // Recovery: back to Pre-Operational
        node.update_state(NmtState::PreOperational);
        assert_eq!(node.state, NmtState::PreOperational);

        // Resume operation
        node.update_state(NmtState::Operational);
        assert_eq!(node.state, NmtState::Operational);
    }

    #[test]
    fn test_multiple_node_states() {
        let node1 = NodeState::new(1);
        let node2 = NodeState::new(2);
        let node3 = NodeState::new(3);

        assert_eq!(node1.node_id, 1);
        assert_eq!(node2.node_id, 2);
        assert_eq!(node3.node_id, 3);

        assert_eq!(node1.state, NmtState::Invalid);
        assert_eq!(node2.state, NmtState::Invalid);
        assert_eq!(node3.state, NmtState::Invalid);
    }

    #[test]
    fn test_nmt_command_equality() {
        assert_eq!(NmtCommand::StartRemoteNode, NmtCommand::StartRemoteNode);
        assert_ne!(NmtCommand::StartRemoteNode, NmtCommand::StopRemoteNode);
        assert_ne!(NmtCommand::ResetNode, NmtCommand::ResetCommunication);
    }

    #[test]
    fn test_node_state_clone() {
        let node1 = NodeState::new(5);
        let node2 = node1.clone();

        assert_eq!(node1.node_id, node2.node_id);
        assert_eq!(node1.state, node2.state);
        assert_eq!(node1.last_state, node2.last_state);
    }

    #[test]
    fn test_heartbeat_from_all_valid_node_ids() {
        // Test heartbeat from all valid node IDs (1-127)
        for node_id in 1..=127u8 {
            let cob_id = 0x700 + node_id as u16;
            let msg = CanMessage::new(cob_id, vec![0x05]).unwrap();
            assert_eq!(msg.node_id(), Some(node_id));
            assert_eq!(msg.nmt_state(), Some(NmtState::Operational));
        }
    }

    #[test]
    fn test_nmt_state_as_u8() {
        assert_eq!(NmtState::Boot as u8, 0x00);
        assert_eq!(NmtState::Stopped as u8, 0x04);
        assert_eq!(NmtState::Operational as u8, 0x05);
        assert_eq!(NmtState::PreOperational as u8, 0x7F);
        assert_eq!(NmtState::Invalid as u8, 0xFF);
    }
}
