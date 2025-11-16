// Event system for asynchronous communication between components
// Implements event bus pattern using tokio broadcast channels

use chrono;
use std::fmt;
use tokio::sync::broadcast;

/// Maximum capacity for the event broadcast channel
const EVENT_CHANNEL_CAPACITY: usize = 1000;

/// Main event structure containing all information about an event
#[derive(Debug, Clone)]
pub struct Event {
    pub source: String,
    pub destination: String,
    pub kind: EventKind,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

impl Event {
    /// Create a new event
    pub fn new(source: String, destination: String, kind: EventKind) -> Self {
        Self {
            source,
            destination,
            kind,
            timestamp: chrono::Local::now(),
        }
    }

    /// Check if this event is targeted to a specific destination
    pub fn is_for(&self, target: &str) -> bool {
        self.destination == "broadcast" || self.destination == target
    }
}

/// Types of events that can be sent through the event bus
#[derive(Debug, Clone)]
pub enum EventKind {
    /// Message from user to agent
    UserMessage(String),

    /// Message from agent to user
    AgentMessage {
        agent_id: String,
        content: String,
    },

    /// Agent status update
    AgentStatusChange {
        agent_id: String,
        status: AgentStatus,
    },

    /// System command (clear conversation, save state, etc.)
    SystemCommand(SystemCommand),

    /// MCP plugin lifecycle events (Phase 3)
    McpPluginEvent(McpPluginEvent),

    /// Test event for initial implementation
    Test(String),
}

/// MCP Plugin events for lifecycle and state changes
///
/// These events allow UI and other components to react to plugin state changes,
/// errors, and health status updates.
#[derive(Debug, Clone)]
pub enum McpPluginEvent {
    /// Plugin successfully started
    Started {
        plugin_id: String,
        tool_count: usize,
    },

    /// Plugin successfully stopped
    Stopped {
        plugin_id: String,
    },

    /// Plugin encountered an error
    Error {
        plugin_id: String,
        message: String,
    },

    /// Plugin tools changed (after reload or initialization)
    ToolsChanged {
        plugin_id: String,
        tool_count: usize,
    },

    /// Plugin health status update
    HealthStatus {
        plugin_id: String,
        status: PluginHealthStatus,
    },

    /// Plugin restart attempted
    RestartAttempt {
        plugin_id: String,
        attempt: u32,
        max_retries: u32,
    },

    /// Configuration reloaded
    ConfigReloaded {
        plugins_added: Vec<String>,
        plugins_removed: Vec<String>,
        plugins_updated: Vec<String>,
    },
}

/// Health status for MCP plugins
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginHealthStatus {
    /// Plugin is healthy and responding
    Healthy,

    /// Plugin process exists but not responding to requests
    Unresponsive,

    /// Plugin process has died
    Dead,
}

/// Agent status states
#[derive(Debug, Clone)]
pub enum AgentStatus {
    Idle,
    Thinking,
    Responding,
    ExecutingTool(String),  // Tool name being executed
    Error(String),
}

/// System-level commands
#[derive(Debug, Clone)]
pub enum SystemCommand {
    ClearConversation,
    SaveState,
    LoadState,
}

/// Event bus for publishing and subscribing to events
pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

impl EventBus {
    /// Create a new event bus with default capacity
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self { tx }
    }

    /// Create a new event bus with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Subscribe to events - returns a receiver
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: Event) -> Result<usize, EventError> {
        self.tx.send(event).map_err(|_| EventError::SendFailed)
    }

    /// Get a clone of the sender for publishing from async tasks
    pub fn sender(&self) -> broadcast::Sender<Event> {
        self.tx.clone()
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during event operations
#[derive(Debug, Clone)]
pub enum EventError {
    SendFailed,
    ReceiveFailed,
    ChannelClosed,
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventError::SendFailed => write!(f, "Failed to send event"),
            EventError::ReceiveFailed => write!(f, "Failed to receive event"),
            EventError::ChannelClosed => write!(f, "Event channel closed"),
        }
    }
}

impl std::error::Error for EventError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new(
            "user".to_string(),
            "agent".to_string(),
            EventKind::UserMessage("Hello".to_string()),
        );

        assert_eq!(event.source, "user");
        assert_eq!(event.destination, "agent");
        assert!(matches!(event.kind, EventKind::UserMessage(_)));
    }

    #[test]
    fn test_event_targeting() {
        let targeted = Event::new(
            "user".to_string(),
            "agent1".to_string(),
            EventKind::Test("test".to_string()),
        );

        assert!(targeted.is_for("agent1"));
        assert!(!targeted.is_for("agent2"));

        let broadcast = Event::new(
            "system".to_string(),
            "broadcast".to_string(),
            EventKind::Test("test".to_string()),
        );

        assert!(broadcast.is_for("agent1"));
        assert!(broadcast.is_for("agent2"));
        assert!(broadcast.is_for("anyone"));
    }

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_event_bus_subscription() {
        let bus = EventBus::new();
        let _rx1 = bus.subscribe();
        let _rx2 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 2);
    }

    #[test]
    fn test_event_bus_publish_receive() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        let event = Event::new(
            "test".to_string(),
            "dest".to_string(),
            EventKind::Test("message".to_string()),
        );

        let result = bus.publish(event.clone());
        assert!(result.is_ok());

        let received = rx.try_recv();
        assert!(received.is_ok());

        let received_event = received.unwrap();
        assert_eq!(received_event.source, "test");
        assert_eq!(received_event.destination, "dest");
    }

    #[test]
    fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let event = Event::new(
            "sender".to_string(),
            "broadcast".to_string(),
            EventKind::Test("broadcast message".to_string()),
        );

        bus.publish(event).unwrap();

        // Both subscribers should receive the event
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }
}
