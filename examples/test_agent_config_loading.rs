use rustbot::agent;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n🧪 Testing Agent Configuration Loading\n");

    // Load agents
    let agent_loader = agent::AgentLoader::new();
    let agent_configs = agent_loader.load_all()?;

    println!("📋 Loaded {} agents\n", agent_configs.len());

    for (i, config) in agent_configs.iter().enumerate() {
        println!("Agent #{}", i + 1);
        println!("  ID: {}", config.id);
        println!("  Name: {}", config.name);
        println!("  Model: {}", config.model);
        println!("  Enabled: {}", config.enabled);
        println!("  Is Primary: {}", config.is_primary);
        println!("  Web Search: {}", config.web_search_enabled);
        println!("  Instructions length: {} chars", config.instructions.len());
        if let Some(ref personality) = config.personality {
            println!("  Personality length: {} chars", personality.len());
        } else {
            println!("  Personality: None");
        }
        println!();
    }

    println!("✅ All agent configs loaded successfully");

    Ok(())
}
