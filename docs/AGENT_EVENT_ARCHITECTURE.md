# Agent and Event System Architecture

## Overview

This document outlines the architecture for implementing an event-driven agent system in Rustbot, where multiple agents can communicate through a central event bus, each with their own instructions, personality, and LLM configuration.

## Research Summary

### Event-Driven Patterns in Rust (2024-2025)

**Key Findings:**
- **tokio::sync::broadcast** is ideal for event bus implementation (single sender, multiple receivers)
- **Actor pattern** commonly used with tokio mpsc channels for mailbox-style communication
- **tiny-tokio-actor** crate provides minimal actor system with event bus
- **Event Bus pattern** decouples producers from consumers via publish-subscribe

**Recommended Approach:**
- Use native Tokio channels (broadcast for events, mpsc for agent mailboxes)
- Implement custom lightweight event bus tailored to our needs
- Avoid heavy dependencies - keep it simple and maintainable

## Architecture Design

### 1. Event System

#### Event Structure
```rust
#[derive(Debug, Clone)]
pub enum EventKind {
    UserMessage(String),           // Message from user
    AgentMessage {                 // Message from agent
        agent_id: String,
        content: String,
    },
    AgentStatusChange {            // Agent state updates
        agent_id: String,
        status: AgentStatus,
    },
    SystemCommand(SystemCommand),  // System-level commands
}

#[derive(Debug, Clone)]
pub struct Event {
    pub source: String,            // Who sent this event
    pub destination: String,       // Who should receive it (or "broadcast")
    pub kind: EventKind,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone)]
pub enum AgentStatus {
    Idle,
    Thinking,
    Responding,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum SystemCommand {
    ClearConversation,
    SaveState,
    LoadState,
}
```

#### Event Bus Implementation
```rust
pub struct EventBus {
    tx: tokio::sync::broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: Event) -> Result<(), EventError> {
        self.tx.send(event).map(|_| ()).map_err(|_| EventError::SendFailed)
    }

    pub fn sender(&self) -> tokio::sync::broadcast::Sender<Event> {
        self.tx.clone()
    }
}
```

#### Event Routing
Events can be:
- **Targeted**: Destination specifies a specific agent ID or "user"
- **Broadcast**: Destination is "broadcast" - all subscribers receive it

Each subscriber filters events based on destination:
```rust
if event.destination == "broadcast" || event.destination == agent_id {
    // Process event
}
```

### 2. Agent System

#### Agent Configuration
```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,                      // Unique agent identifier
    pub name: String,                    // Display name
    pub llm_model: String,               // Which LLM to use
    pub agent_instructions: String,      // Core agent behavior
    pub agent_personality: String,       // Personality/tone instructions
    pub enabled: bool,                   // Is agent active
}

impl AgentConfig {
    pub fn default_assistant() -> Self {
        Self {
            id: "assistant".to_string(),
            name: "Assistant".to_string(),
            llm_model: "Claude Sonnet 4.5".to_string(),
            agent_instructions: String::new(),
            agent_personality: String::new(),
            enabled: true,
        }
    }
}
```

#### Agent Structure
```rust
pub struct Agent {
    config: AgentConfig,
    llm_adapter: Arc<dyn LlmAdapter>,
    event_rx: tokio::sync::broadcast::Receiver<Event>,
    event_tx: tokio::sync::broadcast::Sender<Event>,
    conversation_history: Vec<ChatMessage>,
}

impl Agent {
    pub fn new(
        config: AgentConfig,
        llm_adapter: Arc<dyn LlmAdapter>,
        event_rx: tokio::sync::broadcast::Receiver<Event>,
        event_tx: tokio::sync::broadcast::Sender<Event>,
    ) -> Self {
        Self {
            config,
            llm_adapter,
            event_rx,
            event_tx,
            conversation_history: Vec::new(),
        }
    }

    pub async fn run(&mut self) {
        loop {
            match self.event_rx.recv().await {
                Ok(event) => {
                    // Filter events for this agent
                    if event.destination != "broadcast" && event.destination != self.config.id {
                        continue;
                    }

                    self.handle_event(event).await;
                }
                Err(e) => {
                    tracing::error!("Agent {} event recv error: {}", self.config.id, e);
                    break;
                }
            }
        }
    }

    async fn handle_event(&mut self, event: Event) {
        match event.kind {
            EventKind::UserMessage(content) => {
                self.handle_user_message(content).await;
            }
            EventKind::SystemCommand(cmd) => {
                self.handle_system_command(cmd).await;
            }
            _ => {}
        }
    }

    async fn handle_user_message(&mut self, content: String) {
        // Add to conversation history
        self.conversation_history.push(ChatMessage {
            role: MessageRole::User,
            content: content.clone(),
            input_tokens: None,
            output_tokens: None,
        });

        // Update status
        self.publish_status_change(AgentStatus::Thinking).await;

        // Build LLM request with agent instructions
        let request = self.build_llm_request();

        // Stream response
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        match self.llm_adapter.stream_chat(request, tx).await {
            Ok(_) => {
                let mut response = String::new();
                while let Some(chunk) = rx.recv().await {
                    response.push_str(&chunk);

                    // Publish partial response
                    self.publish_agent_message(response.clone()).await;
                }

                // Add to history
                self.conversation_history.push(ChatMessage {
                    role: MessageRole::Assistant,
                    content: response,
                    input_tokens: None,
                    output_tokens: None,
                });

                self.publish_status_change(AgentStatus::Idle).await;
            }
            Err(e) => {
                self.publish_status_change(AgentStatus::Error(e.to_string())).await;
            }
        }
    }

    fn build_llm_request(&self) -> LlmRequest {
        // Combine system prompts + agent instructions
        let system_content = format!(
            "{}\n\n# Agent Instructions\n{}\n\n# Agent Personality\n{}",
            "System context here",
            self.config.agent_instructions,
            self.config.agent_personality
        );

        let mut messages = vec![
            Message {
                role: "system".to_string(),
                content: system_content,
            }
        ];

        // Add conversation history
        for msg in &self.conversation_history {
            messages.push(Message {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: msg.content.clone(),
            });
        }

        LlmRequest {
            model: None, // Use adapter's default
            messages,
            temperature: Some(0.7),
            max_tokens: Some(4096),
        }
    }

    async fn publish_agent_message(&self, content: String) {
        let event = Event {
            source: self.config.id.clone(),
            destination: "user".to_string(),
            kind: EventKind::AgentMessage {
                agent_id: self.config.id.clone(),
                content,
            },
            timestamp: chrono::Local::now(),
        };

        let _ = self.event_tx.send(event);
    }

    async fn publish_status_change(&self, status: AgentStatus) {
        let event = Event {
            source: self.config.id.clone(),
            destination: "broadcast".to_string(),
            kind: EventKind::AgentStatusChange {
                agent_id: self.config.id.clone(),
                status,
            },
            timestamp: chrono::Local::now(),
        };

        let _ = self.event_tx.send(event);
    }

    async fn handle_system_command(&mut self, cmd: SystemCommand) {
        match cmd {
            SystemCommand::ClearConversation => {
                self.conversation_history.clear();
            }
            _ => {}
        }
    }
}
```

### 3. Application Integration

#### App State
```rust
struct RustbotApp {
    // Event system
    event_bus: Arc<EventBus>,
    event_rx: tokio::sync::broadcast::Receiver<Event>,

    // Agents
    agents: HashMap<String, AgentConfig>,
    active_agent_id: String,

    // UI state
    current_view: AppView,
    settings_view: SettingsView,

    // Runtime
    runtime: tokio::runtime::Runtime,
}
```

#### Initialization
```rust
impl RustbotApp {
    pub fn new(api_key: String) -> Self {
        let event_bus = Arc::new(EventBus::new(1000));
        let event_rx = event_bus.subscribe();

        // Create default assistant agent
        let mut agents = HashMap::new();
        agents.insert(
            "assistant".to_string(),
            AgentConfig::default_assistant(),
        );

        // Spawn agent task
        let runtime = tokio::runtime::Runtime::new().unwrap();
        Self::spawn_agent(
            &runtime,
            agents.get("assistant").unwrap().clone(),
            api_key.clone(),
            event_bus.clone(),
        );

        Self {
            event_bus,
            event_rx,
            agents,
            active_agent_id: "assistant".to_string(),
            current_view: AppView::Chat,
            settings_view: SettingsView::AiSettings,
            runtime,
        }
    }

    fn spawn_agent(
        runtime: &tokio::runtime::Runtime,
        config: AgentConfig,
        api_key: String,
        event_bus: Arc<EventBus>,
    ) {
        let llm_adapter: Arc<dyn LlmAdapter> = Arc::new(
            OpenRouterAdapter::new(api_key)
        );

        let event_rx = event_bus.subscribe();
        let event_tx = event_bus.sender();

        runtime.spawn(async move {
            let mut agent = Agent::new(config, llm_adapter, event_rx, event_tx);
            agent.run().await;
        });
    }
}
```

#### UI Event Publishing
```rust
// When user sends message
fn send_message(&mut self) {
    let content = self.input_text.clone();
    self.input_text.clear();

    let event = Event {
        source: "user".to_string(),
        destination: self.active_agent_id.clone(),
        kind: EventKind::UserMessage(content),
        timestamp: chrono::Local::now(),
    };

    let _ = self.event_bus.publish(event);
}
```

#### UI Event Listening
```rust
impl eframe::App for RustbotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for events from agents
        while let Ok(event) = self.event_rx.try_recv() {
            match event.kind {
                EventKind::AgentMessage { agent_id, content } => {
                    // Update chat display
                    self.update_chat_display(agent_id, content);
                    ctx.request_repaint();
                }
                EventKind::AgentStatusChange { agent_id, status } => {
                    // Update status indicator
                    self.update_agent_status(agent_id, status);
                    ctx.request_repaint();
                }
                _ => {}
            }
        }

        // Render UI...
    }
}
```

### 4. UI Changes

#### New Settings Panel: Agents
```rust
enum SettingsView {
    AiSettings,
    SystemPrompts,
    Agents,  // NEW
}

fn render_agents_settings(&mut self, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Agent Configuration");
        ui.add_space(10.0);

        // List of agents
        for (id, config) in &mut self.agents {
            ui.collapsing(&config.name, |ui| {
                self.render_agent_config(ui, config);
            });
            ui.add_space(10.0);
        }

        // Add new agent button
        if ui.button("+ Add New Agent").clicked() {
            // Create new agent
        }
    });
}

fn render_agent_config(&mut self, ui: &mut egui::Ui, config: &mut AgentConfig) {
    // Agent name
    ui.label("Agent Name:");
    ui.text_edit_singleline(&mut config.name);
    ui.add_space(10.0);

    // LLM selection
    ui.label("LLM Model:");
    egui::ComboBox::from_id_salt(format!("agent_model_{}", config.id))
        .selected_text(&config.llm_model)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut config.llm_model, "Claude Sonnet 4.5".to_string(), "Claude Sonnet 4.5");
            ui.selectable_value(&mut config.llm_model, "Claude Sonnet 4".to_string(), "Claude Sonnet 4");
            ui.selectable_value(&mut config.llm_model, "Claude Opus 4".to_string(), "Claude Opus 4");
            ui.selectable_value(&mut config.llm_model, "GPT-4".to_string(), "GPT-4");
        });
    ui.add_space(10.0);

    // Agent instructions
    ui.label(egui::RichText::new("Agent Instructions:").strong());
    ui.add_space(5.0);
    ui.add_sized(
        [ui.available_width() - 20.0, 200.0],
        egui::TextEdit::multiline(&mut config.agent_instructions)
            .hint_text("Enter agent-specific instructions...")
            .margin(egui::vec2(8.0, 8.0))
    );
    ui.add_space(10.0);

    // Agent personality
    ui.label(egui::RichText::new("Agent Personality:").strong());
    ui.add_space(5.0);
    ui.add_sized(
        [ui.available_width() - 20.0, 200.0],
        egui::TextEdit::multiline(&mut config.agent_personality)
            .hint_text("Enter agent personality/tone...")
            .margin(egui::vec2(8.0, 8.0))
    );
    ui.add_space(10.0);

    // Enabled toggle
    ui.checkbox(&mut config.enabled, "Agent Enabled");
    ui.add_space(10.0);

    // Save button
    if ui.button("Save Agent Configuration").clicked() {
        // Save to file
    }
}
```

### 5. Configuration Persistence

#### Agent Storage
```
~/.rustbot/agents/{agent_id}/
├── config.json          # Agent configuration
├── instructions/
│   ├── current         # Current instructions
│   └── backup_*        # Timestamped backups
└── personality/
    ├── current         # Current personality
    └── backup_*        # Timestamped backups
```

#### Config File Format
```json
{
  "id": "assistant",
  "name": "Assistant",
  "llm_model": "Claude Sonnet 4.5",
  "enabled": true
}
```

## Implementation Phases

### Phase 1: Event System Foundation (High Priority)
1. Create event types and Event struct
2. Implement EventBus with broadcast channel
3. Integrate into RustbotApp
4. Test event publishing and receiving

### Phase 2: Basic Agent Framework (High Priority)
1. Create AgentConfig struct
2. Create Agent struct with run loop
3. Implement agent spawning
4. Test user-to-agent communication

### Phase 3: Agent UI (Medium Priority)
1. Add Agents tab to settings
2. Implement agent configuration UI
3. Add agent list and editing
4. Add save/load functionality

### Phase 4: Configuration Migration (Medium Priority)
1. Move personality prompts to agent level
2. Create migration function from old system
3. Update file storage structure
4. Test backward compatibility

### Phase 5: Multi-Agent Support (Low Priority)
1. Support multiple active agents
2. Agent selection in UI
3. Agent-to-agent communication
4. Advanced routing logic

## Benefits of This Architecture

### Decoupling
- UI doesn't directly call LLM
- Agents don't know about UI
- Easy to add new event sources/sinks

### Scalability
- Add new agents without changing core code
- Each agent has independent LLM instance
- Event bus handles all routing

### Flexibility
- Agents can have different models
- Different instructions per agent
- Easy to implement agent specialization

### Testing
- Mock event bus for testing
- Test agents independently
- Verify event flows in isolation

## Interrupt and Trigger System

### Overview
The event-driven model supports **asynchronous interrupts**, allowing any component to inject events into the conversation flow without waiting for synchronous request-response cycles. This enables real-time notifications, scheduled tasks, external triggers, and multi-source communication.

### Interrupt Event Types
```rust
#[derive(Debug, Clone)]
pub enum InterruptKind {
    // Timer-based interrupts
    ScheduledReminder {
        message: String,
        scheduled_for: chrono::DateTime<chrono::Local>,
    },
    PeriodicUpdate {
        interval: std::time::Duration,
        message: String,
    },

    // External system interrupts
    ExternalNotification {
        source: String,
        priority: InterruptPriority,
        content: String,
    },

    // Agent-initiated interrupts
    AgentProactiveMessage {
        agent_id: String,
        reason: String,
        content: String,
    },

    // System interrupts
    SystemAlert {
        level: AlertLevel,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterruptPriority {
    Low,        // Can wait for user attention
    Medium,     // Should notify but not disruptive
    High,       // Interrupt current flow
    Critical,   // Immediate attention required
}

#[derive(Debug, Clone)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}
```

### Interrupt Manager
```rust
pub struct InterruptManager {
    event_tx: tokio::sync::broadcast::Sender<Event>,
    active_timers: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
    interrupt_rules: Arc<Mutex<InterruptRules>>,
}

#[derive(Clone)]
pub struct InterruptRules {
    pub allow_agent_interrupts: bool,
    pub allow_scheduled_interrupts: bool,
    pub allow_external_interrupts: bool,
    pub priority_threshold: InterruptPriority,
}

impl InterruptManager {
    pub fn new(event_tx: tokio::sync::broadcast::Sender<Event>) -> Self {
        Self {
            event_tx,
            active_timers: Arc::new(Mutex::new(HashMap::new())),
            interrupt_rules: Arc::new(Mutex::new(InterruptRules::default())),
        }
    }

    // Schedule one-time interrupt
    pub async fn schedule_reminder(
        &self,
        id: String,
        delay: std::time::Duration,
        message: String,
    ) -> Result<(), InterruptError> {
        let tx = self.event_tx.clone();
        let scheduled_for = chrono::Local::now() + chrono::Duration::from_std(delay)?;

        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;

            let event = Event {
                source: "interrupt_manager".to_string(),
                destination: "user".to_string(),
                kind: EventKind::Interrupt(InterruptKind::ScheduledReminder {
                    message,
                    scheduled_for,
                }),
                timestamp: chrono::Local::now(),
            };

            let _ = tx.send(event);
        });

        self.active_timers.lock().await.insert(id, handle);
        Ok(())
    }

    // Schedule periodic interrupt
    pub async fn schedule_periodic(
        &self,
        id: String,
        interval: std::time::Duration,
        message_fn: Arc<dyn Fn() -> String + Send + Sync>,
    ) -> Result<(), InterruptError> {
        let tx = self.event_tx.clone();

        let handle = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let message = message_fn();
                let event = Event {
                    source: "interrupt_manager".to_string(),
                    destination: "user".to_string(),
                    kind: EventKind::Interrupt(InterruptKind::PeriodicUpdate {
                        interval,
                        message,
                    }),
                    timestamp: chrono::Local::now(),
                };

                if tx.send(event).is_err() {
                    break; // Event bus closed
                }
            }
        });

        self.active_timers.lock().await.insert(id, handle);
        Ok(())
    }

    // Cancel scheduled interrupt
    pub async fn cancel_timer(&self, id: &str) -> Result<(), InterruptError> {
        if let Some(handle) = self.active_timers.lock().await.remove(id) {
            handle.abort();
            Ok(())
        } else {
            Err(InterruptError::TimerNotFound)
        }
    }

    // Publish external interrupt
    pub fn publish_interrupt(
        &self,
        kind: InterruptKind,
        priority: InterruptPriority,
    ) -> Result<(), InterruptError> {
        // Check if interrupt is allowed
        let rules = self.interrupt_rules.blocking_lock();
        if priority < rules.priority_threshold {
            return Err(InterruptError::BelowThreshold);
        }

        let event = Event {
            source: "interrupt_manager".to_string(),
            destination: "user".to_string(),
            kind: EventKind::Interrupt(kind),
            timestamp: chrono::Local::now(),
        };

        self.event_tx.send(event)
            .map(|_| ())
            .map_err(|_| InterruptError::SendFailed)
    }

    // Update interrupt rules
    pub async fn update_rules(&self, rules: InterruptRules) {
        *self.interrupt_rules.lock().await = rules;
    }
}
```

### Updated Event Types
```rust
#[derive(Debug, Clone)]
pub enum EventKind {
    UserMessage(String),
    AgentMessage {
        agent_id: String,
        content: String,
    },
    AgentStatusChange {
        agent_id: String,
        status: AgentStatus,
    },
    SystemCommand(SystemCommand),
    Interrupt(InterruptKind),  // NEW
}
```

### UI Handling of Interrupts
```rust
impl RustbotApp {
    fn handle_interrupt_event(&mut self, interrupt: InterruptKind, ctx: &egui::Context) {
        match interrupt {
            InterruptKind::ScheduledReminder { message, .. } => {
                // Show notification in chat
                self.add_system_message(&format!("⏰ Reminder: {}", message));
                ctx.request_repaint();
            }

            InterruptKind::PeriodicUpdate { message, .. } => {
                // Show subtle notification
                self.add_system_message(&format!("ℹ️ Update: {}", message));
                ctx.request_repaint();
            }

            InterruptKind::ExternalNotification { source, priority, content } => {
                let icon = match priority {
                    InterruptPriority::Critical => "🚨",
                    InterruptPriority::High => "⚠️",
                    InterruptPriority::Medium => "ℹ️",
                    InterruptPriority::Low => "💬",
                };

                self.add_system_message(&format!("{} {}: {}", icon, source, content));
                ctx.request_repaint();
            }

            InterruptKind::AgentProactiveMessage { agent_id, reason, content } => {
                // Agent decided to speak without being asked
                self.add_agent_message(&agent_id, &content, Some(&reason));
                ctx.request_repaint();
            }

            InterruptKind::SystemAlert { level, message } => {
                let (icon, color) = match level {
                    AlertLevel::Critical => ("🔴", egui::Color32::RED),
                    AlertLevel::Error => ("❌", egui::Color32::from_rgb(200, 60, 60)),
                    AlertLevel::Warning => ("⚠️", egui::Color32::from_rgb(220, 180, 40)),
                    AlertLevel::Info => ("ℹ️", egui::Color32::from_rgb(60, 120, 220)),
                };

                self.add_system_message_colored(&format!("{} {}", icon, message), color);
                ctx.request_repaint();
            }
        }
    }
}
```

### Example Use Cases

#### 1. Scheduled Reminders
```rust
// User: "Remind me in 5 minutes to check the deployment"
interrupt_manager.schedule_reminder(
    "deployment_check".to_string(),
    Duration::from_secs(300),
    "Check the deployment status".to_string(),
).await?;
```

#### 2. Periodic Status Updates
```rust
// Monitor system health every 30 seconds
interrupt_manager.schedule_periodic(
    "health_check".to_string(),
    Duration::from_secs(30),
    Arc::new(|| {
        format!("System health: CPU {}%, Memory {}%", get_cpu(), get_memory())
    }),
).await?;
```

#### 3. External Notifications
```rust
// Webhook received
interrupt_manager.publish_interrupt(
    InterruptKind::ExternalNotification {
        source: "GitHub".to_string(),
        priority: InterruptPriority::High,
        content: "Pull request #123 was merged".to_string(),
    },
    InterruptPriority::High,
)?;
```

#### 4. Proactive Agent Messages
```rust
// Agent detects something interesting
let event = Event {
    source: agent_id.clone(),
    destination: "user".to_string(),
    kind: EventKind::Interrupt(InterruptKind::AgentProactiveMessage {
        agent_id: agent_id.clone(),
        reason: "Error detected in logs".to_string(),
        content: "I noticed an error spike in the last 5 minutes. Would you like me to investigate?".to_string(),
    }),
    timestamp: chrono::Local::now(),
};
```

### Interrupt Configuration UI
```rust
fn render_interrupt_settings(&mut self, ui: &mut egui::Ui) {
    ui.heading("Interrupt Settings");
    ui.add_space(10.0);

    ui.checkbox(&mut self.interrupt_rules.allow_agent_interrupts, "Allow agents to interrupt");
    ui.checkbox(&mut self.interrupt_rules.allow_scheduled_interrupts, "Allow scheduled reminders");
    ui.checkbox(&mut self.interrupt_rules.allow_external_interrupts, "Allow external notifications");

    ui.add_space(10.0);
    ui.label("Minimum priority for interrupts:");
    egui::ComboBox::from_label("Priority Threshold")
        .selected_text(format!("{:?}", self.interrupt_rules.priority_threshold))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut self.interrupt_rules.priority_threshold, InterruptPriority::Low, "Low");
            ui.selectable_value(&mut self.interrupt_rules.priority_threshold, InterruptPriority::Medium, "Medium");
            ui.selectable_value(&mut self.interrupt_rules.priority_threshold, InterruptPriority::High, "High");
            ui.selectable_value(&mut self.interrupt_rules.priority_threshold, InterruptPriority::Critical, "Critical");
        });
}
```

### Benefits of Interrupt System

1. **Asynchronous Communication**: No blocking waits for responses
2. **Real-time Updates**: Immediate notifications without polling
3. **Flexible Triggers**: Time-based, event-based, or condition-based
4. **Priority Management**: Control what can interrupt user
5. **Multi-source Input**: Events from agents, timers, external systems
6. **User Control**: Configure interrupt behavior and thresholds

## Event Flow Visualizer

### Overview
A real-time visualization panel showing the flow of events between components: User ↔ Agent ↔ LLM. This provides transparency into system behavior and helps debug event routing.

### Visualizer Architecture

#### Visualization State
```rust
#[derive(Clone)]
pub struct VisualizerState {
    nodes: HashMap<String, VisualizerNode>,
    connections: Vec<VisualizerConnection>,
    recent_events: VecDeque<VisualEvent>,
    max_history: usize,
}

#[derive(Clone)]
pub struct VisualizerNode {
    id: String,
    node_type: NodeType,
    position: egui::Pos2,
    status: NodeStatus,
    label: String,
}

#[derive(Clone, PartialEq)]
pub enum NodeType {
    User,
    Agent,
    LLM,
    InterruptManager,
    System,
}

#[derive(Clone)]
pub enum NodeStatus {
    Idle,
    Active,
    Thinking,
    Error,
}

#[derive(Clone)]
pub struct VisualizerConnection {
    from: String,
    to: String,
    connection_type: ConnectionType,
    active: bool,
    last_activity: std::time::Instant,
}

#[derive(Clone)]
pub enum ConnectionType {
    UserMessage,
    AgentResponse,
    LLMRequest,
    LLMResponse,
    Interrupt,
    SystemCommand,
}

#[derive(Clone)]
pub struct VisualEvent {
    from: String,
    to: String,
    event_type: ConnectionType,
    timestamp: std::time::Instant,
    content_preview: String, // First 50 chars
}

impl VisualizerState {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();

        // Initialize nodes with positions
        nodes.insert("user".to_string(), VisualizerNode {
            id: "user".to_string(),
            node_type: NodeType::User,
            position: egui::pos2(100.0, 200.0),
            status: NodeStatus::Idle,
            label: "User".to_string(),
        });

        nodes.insert("assistant".to_string(), VisualizerNode {
            id: "assistant".to_string(),
            node_type: NodeType::Agent,
            position: egui::pos2(300.0, 200.0),
            status: NodeStatus::Idle,
            label: "Assistant Agent".to_string(),
        });

        nodes.insert("llm".to_string(), VisualizerNode {
            id: "llm".to_string(),
            node_type: NodeType::LLM,
            position: egui::pos2(500.0, 200.0),
            status: NodeStatus::Idle,
            label: "Claude Sonnet 4.5".to_string(),
        });

        Self {
            nodes,
            connections: Vec::new(),
            recent_events: VecDeque::new(),
            max_history: 20,
        }
    }

    pub fn record_event(&mut self, event: &Event) {
        let connection_type = match &event.kind {
            EventKind::UserMessage(_) => ConnectionType::UserMessage,
            EventKind::AgentMessage { .. } => ConnectionType::AgentResponse,
            EventKind::Interrupt(_) => ConnectionType::Interrupt,
            EventKind::SystemCommand(_) => ConnectionType::SystemCommand,
            _ => return,
        };

        let visual_event = VisualEvent {
            from: event.source.clone(),
            to: event.destination.clone(),
            event_type: connection_type.clone(),
            timestamp: std::time::Instant::now(),
            content_preview: format!("{:?}", event.kind).chars().take(50).collect(),
        };

        self.recent_events.push_back(visual_event);
        if self.recent_events.len() > self.max_history {
            self.recent_events.pop_front();
        }

        // Update or create connection
        if let Some(conn) = self.connections.iter_mut().find(|c| {
            c.from == event.source && c.to == event.destination
        }) {
            conn.active = true;
            conn.last_activity = std::time::Instant::now();
        } else {
            self.connections.push(VisualizerConnection {
                from: event.source.clone(),
                to: event.destination.clone(),
                connection_type,
                active: true,
                last_activity: std::time::Instant::now(),
            });
        }

        // Update node status
        if let Some(node) = self.nodes.get_mut(&event.source) {
            node.status = NodeStatus::Active;
        }
    }

    pub fn record_llm_activity(&mut self, agent_id: &str, activity: LLMActivity) {
        match activity {
            LLMActivity::RequestSent => {
                let visual_event = VisualEvent {
                    from: agent_id.to_string(),
                    to: "llm".to_string(),
                    event_type: ConnectionType::LLMRequest,
                    timestamp: std::time::Instant::now(),
                    content_preview: "LLM Request".to_string(),
                };
                self.recent_events.push_back(visual_event);

                if let Some(node) = self.nodes.get_mut("llm") {
                    node.status = NodeStatus::Thinking;
                }
            }
            LLMActivity::ResponseReceived => {
                let visual_event = VisualEvent {
                    from: "llm".to_string(),
                    to: agent_id.to_string(),
                    event_type: ConnectionType::LLMResponse,
                    timestamp: std::time::Instant::now(),
                    content_preview: "LLM Response".to_string(),
                };
                self.recent_events.push_back(visual_event);

                if let Some(node) = self.nodes.get_mut("llm") {
                    node.status = NodeStatus::Idle;
                }
            }
        }
    }

    pub fn update_agent_status(&mut self, agent_id: &str, status: AgentStatus) {
        if let Some(node) = self.nodes.get_mut(agent_id) {
            node.status = match status {
                AgentStatus::Idle => NodeStatus::Idle,
                AgentStatus::Thinking | AgentStatus::Responding => NodeStatus::Thinking,
                AgentStatus::Error(_) => NodeStatus::Error,
            };
        }
    }

    pub fn decay_connections(&mut self) {
        let now = std::time::Instant::now();
        for conn in &mut self.connections {
            if now.duration_since(conn.last_activity).as_secs() > 2 {
                conn.active = false;
            }
        }
    }
}

#[derive(Clone)]
pub enum LLMActivity {
    RequestSent,
    ResponseReceived,
}
```

#### Visualizer UI Rendering
```rust
fn render_visualizer(&mut self, ui: &mut egui::Ui) {
    ui.heading("Event Flow Visualizer");
    ui.add_space(10.0);

    ui.label("Real-time visualization of communication between components");
    ui.add_space(20.0);

    // Decay old connection activity
    self.visualizer_state.decay_connections();

    // Main visualization area
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), 400.0),
        egui::Sense::hover(),
    );

    let rect = response.rect;

    // Draw background
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(245, 245, 247));

    // Draw connections (arrows)
    self.draw_connections(&painter, &rect);

    // Draw nodes (boxes)
    self.draw_nodes(&painter, &rect);

    // Draw legend
    ui.add_space(20.0);
    self.draw_legend(ui);

    // Draw event log
    ui.add_space(20.0);
    self.draw_event_log(ui);
}

fn draw_nodes(&self, painter: &egui::Painter, rect: &egui::Rect) {
    for node in self.visualizer_state.nodes.values() {
        let pos = rect.min + node.position.to_vec2();
        let size = egui::vec2(80.0, 60.0);
        let node_rect = egui::Rect::from_min_size(pos, size);

        // Determine color based on status
        let (bg_color, border_color) = match node.status {
            NodeStatus::Idle => (
                egui::Color32::from_rgb(255, 255, 255),
                egui::Color32::from_rgb(180, 180, 180),
            ),
            NodeStatus::Active => (
                egui::Color32::from_rgb(220, 240, 255),
                egui::Color32::from_rgb(60, 120, 220),
            ),
            NodeStatus::Thinking => (
                egui::Color32::from_rgb(255, 245, 220),
                egui::Color32::from_rgb(220, 180, 60),
            ),
            NodeStatus::Error => (
                egui::Color32::from_rgb(255, 230, 230),
                egui::Color32::from_rgb(200, 60, 60),
            ),
        };

        // Draw box
        painter.rect_filled(node_rect, 4.0, bg_color);
        painter.rect_stroke(node_rect, 4.0, egui::Stroke::new(2.0, border_color));

        // Draw icon based on node type
        let icon = match node.node_type {
            NodeType::User => "👤",
            NodeType::Agent => "🤖",
            NodeType::LLM => "🧠",
            NodeType::InterruptManager => "⏰",
            NodeType::System => "⚙️",
        };

        let icon_pos = pos + egui::vec2(size.x / 2.0 - 10.0, 10.0);
        painter.text(
            icon_pos,
            egui::Align2::LEFT_TOP,
            icon,
            egui::FontId::proportional(20.0),
            egui::Color32::BLACK,
        );

        // Draw label
        let label_pos = pos + egui::vec2(size.x / 2.0, 40.0);
        painter.text(
            label_pos,
            egui::Align2::CENTER_CENTER,
            &node.label,
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(80, 80, 80),
        );
    }
}

fn draw_connections(&self, painter: &egui::Painter, rect: &egui::Rect) {
    for conn in &self.visualizer_state.connections {
        if !conn.active {
            continue; // Skip inactive connections
        }

        let from_node = self.visualizer_state.nodes.get(&conn.from);
        let to_node = self.visualizer_state.nodes.get(&conn.to);

        if let (Some(from), Some(to)) = (from_node, to_node) {
            let start = rect.min + from.position.to_vec2() + egui::vec2(80.0, 30.0); // Right side
            let end = rect.min + to.position.to_vec2() + egui::vec2(0.0, 30.0); // Left side

            // Determine color based on connection type
            let color = match conn.connection_type {
                ConnectionType::UserMessage => egui::Color32::from_rgb(60, 120, 220),
                ConnectionType::AgentResponse => egui::Color32::from_rgb(80, 180, 100),
                ConnectionType::LLMRequest => egui::Color32::from_rgb(220, 120, 60),
                ConnectionType::LLMResponse => egui::Color32::from_rgb(180, 100, 220),
                ConnectionType::Interrupt => egui::Color32::from_rgb(220, 60, 60),
                ConnectionType::SystemCommand => egui::Color32::from_rgb(120, 120, 120),
            };

            // Draw arrow
            painter.line_segment(
                [start, end],
                egui::Stroke::new(2.0, color),
            );

            // Draw arrowhead
            let direction = (end - start).normalized();
            let perpendicular = egui::vec2(-direction.y, direction.x);
            let arrow_size = 8.0;

            let arrow_tip = end;
            let arrow_left = end - direction * arrow_size + perpendicular * arrow_size / 2.0;
            let arrow_right = end - direction * arrow_size - perpendicular * arrow_size / 2.0;

            painter.add(egui::Shape::convex_polygon(
                vec![arrow_tip, arrow_left, arrow_right],
                color,
                egui::Stroke::NONE,
            ));
        }
    }
}

fn draw_legend(&self, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Legend:").strong());
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        ui.label("User Message");
        ui.colored_label(egui::Color32::from_rgb(60, 120, 220), "━━→");
        ui.add_space(20.0);

        ui.label("Agent Response");
        ui.colored_label(egui::Color32::from_rgb(80, 180, 100), "━━→");
        ui.add_space(20.0);

        ui.label("LLM Request");
        ui.colored_label(egui::Color32::from_rgb(220, 120, 60), "━━→");
        ui.add_space(20.0);

        ui.label("LLM Response");
        ui.colored_label(egui::Color32::from_rgb(180, 100, 220), "━━→");
    });
}

fn draw_event_log(&mut self, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Recent Events:").strong());
    ui.add_space(5.0);

    egui::ScrollArea::vertical()
        .max_height(150.0)
        .show(ui, |ui| {
            for event in self.visualizer_state.recent_events.iter().rev() {
                let elapsed = event.timestamp.elapsed().as_secs_f32();
                let icon = match event.event_type {
                    ConnectionType::UserMessage => "💬",
                    ConnectionType::AgentResponse => "🤖",
                    ConnectionType::LLMRequest => "📤",
                    ConnectionType::LLMResponse => "📥",
                    ConnectionType::Interrupt => "⚡",
                    ConnectionType::SystemCommand => "⚙️",
                };

                ui.horizontal(|ui| {
                    ui.label(icon);
                    ui.label(format!("{} → {}", event.from, event.to));
                    ui.label(egui::RichText::new(format!("({:.1}s ago)", elapsed))
                        .size(10.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)));
                });

                ui.label(egui::RichText::new(&event.content_preview)
                    .size(10.0)
                    .color(egui::Color32::from_rgb(100, 100, 100)));

                ui.add_space(5.0);
            }
        });
}
```

#### Integration with Event System

```rust
impl RustbotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for events from agents
        while let Ok(event) = self.event_rx.try_recv() {
            // Record event in visualizer
            self.visualizer_state.record_event(&event);

            match event.kind {
                EventKind::AgentMessage { agent_id, content } => {
                    self.update_chat_display(agent_id, content);
                    ctx.request_repaint();
                }
                EventKind::AgentStatusChange { agent_id, status } => {
                    self.visualizer_state.update_agent_status(&agent_id, status);
                    self.update_agent_status(agent_id, status);
                    ctx.request_repaint();
                }
                _ => {}
            }
        }

        // ... rest of update logic
    }
}

// In Agent implementation
impl Agent {
    async fn handle_user_message(&mut self, content: String) {
        // ... existing code ...

        // Notify visualizer of LLM request
        self.publish_llm_activity(LLMActivity::RequestSent).await;

        let request = self.build_llm_request();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        match self.llm_adapter.stream_chat(request, tx).await {
            Ok(_) => {
                // Notify visualizer of LLM response
                self.publish_llm_activity(LLMActivity::ResponseReceived).await;

                // ... rest of handling ...
            }
            Err(e) => {
                self.publish_status_change(AgentStatus::Error(e.to_string())).await;
            }
        }
    }

    async fn publish_llm_activity(&self, activity: LLMActivity) {
        let event = Event {
            source: self.config.id.clone(),
            destination: "visualizer".to_string(),
            kind: EventKind::LLMActivity {
                agent_id: self.config.id.clone(),
                activity,
            },
            timestamp: chrono::Local::now(),
        };

        let _ = self.event_tx.send(event);
    }
}
```

#### Settings Integration

```rust
enum SettingsView {
    AiSettings,
    SystemPrompts,
    Agents,
    Visualizer,  // NEW
}

fn render_settings_view(&mut self, ui: &mut egui::Ui) {
    // Secondary navigation bar under header
    ui.horizontal(|ui| {
        let ai_settings_button = ui.add(
            egui::SelectableLabel::new(
                self.settings_view == SettingsView::AiSettings,
                "AI Settings"
            )
        );
        if ai_settings_button.clicked() {
            self.settings_view = SettingsView::AiSettings;
        }

        ui.add_space(10.0);

        let system_prompts_button = ui.add(
            egui::SelectableLabel::new(
                self.settings_view == SettingsView::SystemPrompts,
                "System Prompts"
            )
        );
        if system_prompts_button.clicked() {
            self.settings_view = SettingsView::SystemPrompts;
        }

        ui.add_space(10.0);

        let agents_button = ui.add(
            egui::SelectableLabel::new(
                self.settings_view == SettingsView::Agents,
                "Agents"
            )
        );
        if agents_button.clicked() {
            self.settings_view = SettingsView::Agents;
        }

        ui.add_space(10.0);

        let visualizer_button = ui.add(
            egui::SelectableLabel::new(
                self.settings_view == SettingsView::Visualizer,
                "Visualizer"
            )
        );
        if visualizer_button.clicked() {
            self.settings_view = SettingsView::Visualizer;
        }
    });

    ui.separator();

    // Render selected view
    match self.settings_view {
        SettingsView::AiSettings => self.render_ai_settings(ui),
        SettingsView::SystemPrompts => self.render_system_prompts(ui),
        SettingsView::Agents => self.render_agents_settings(ui),
        SettingsView::Visualizer => self.render_visualizer(ui),
    }
}
```

### Visualizer Features

1. **Real-time Updates**: Immediate visualization of all events
2. **Node Status**: Color-coded status indicators (idle, active, thinking, error)
3. **Animated Arrows**: Show active connections between components
4. **Event Log**: Scrollable history of recent events with timestamps
5. **Component Icons**: Clear visual distinction between User, Agent, LLM
6. **Color-coded Connections**: Different colors for different message types
7. **Auto-decay**: Connections fade after 2 seconds of inactivity

### Benefits

1. **Transparency**: See exactly what's happening in the system
2. **Debugging**: Identify communication bottlenecks or failures
3. **Understanding**: Visual representation aids user comprehension
4. **Monitoring**: Real-time system health at a glance
5. **Educational**: Learn how event-driven architecture works

## Future Enhancements

1. **Agent Specialization**: Code agent, research agent, writing agent
2. **Agent Collaboration**: Agents can communicate with each other
3. **Tool Use**: Agents can publish tool-use events
4. **Streaming UI**: Real-time partial responses
5. **Agent Marketplace**: Import/export agent configs
6. **Agent Memory**: Per-agent conversation history
7. **Advanced Routing**: Priority queues, message filtering
8. **Monitoring**: Event logging, agent performance metrics
9. **Conditional Interrupts**: Rule-based triggering (e.g., "if CPU > 90%, notify")
10. **Smart Interrupt Batching**: Group low-priority interrupts
11. **Do Not Disturb Mode**: Temporary interrupt suppression
12. **Interrupt History**: View and replay past interrupts
13. **Advanced Visualizer**: Zoom, pan, custom layouts, export diagrams
14. **Performance Metrics**: Message latency, throughput visualization
15. **Playback Mode**: Replay event history for debugging

## References

- Tokio Event Bus Implementation: https://blog.digital-horror.com/blog/event-bus-in-tokio/
- Tiny Tokio Actor: https://crates.io/crates/tiny-tokio-actor
- Tokio Channels Documentation: https://tokio.rs/tokio/tutorial/channels
- Event-Driven Architecture Patterns: https://elitedev.in/rust/building-powerful-event-driven-systems-in-rust-7-/
