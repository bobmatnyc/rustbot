use super::config::JsonAgentConfig;
use crate::agent::AgentConfig;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Agent loader for loading agent configurations from JSON files
///
/// Design Decision: Directory-based agent discovery with configurable search paths
///
/// Rationale: Selected directory scanning approach to enable:
/// 1. Automatic agent discovery without configuration files listing agents
/// 2. Easy addition of custom agents (drop JSON in directory)
/// 3. Separation of built-in (presets) vs user-defined (custom) agents
/// 4. Graceful degradation when directories don't exist
///
/// Trade-offs:
/// - Simplicity: Auto-discovery vs. explicit agent registry
/// - Performance: O(n) directory scan vs. O(1) config lookup (acceptable for <100 agents)
/// - Flexibility: Filesystem-based vs. database-backed (filesystem simpler for local app)
///
/// Error Handling: Individual agent load failures are logged but don't block other agents.
/// This allows partial degradation rather than all-or-nothing failure.
///
/// Extension Points: Could add agent validation, dependency resolution, or database backend
/// if agent count exceeds 100 or network deployment is needed.
pub struct AgentLoader {
    /// Directories to search for agent JSON files
    search_paths: Vec<PathBuf>,
}

impl AgentLoader {
    /// Create a new agent loader with default search paths
    ///
    /// Default paths:
    /// - `agents/presets` - Built-in agent templates
    /// - `agents/custom` - User-defined agents
    pub fn new() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("agents/presets"),
                PathBuf::from("agents/custom"),
            ],
        }
    }

    /// Add a custom search path for agent JSON files
    ///
    /// Allows extending agent search to additional directories.
    /// Paths are searched in order, later paths override earlier ones.
    pub fn add_search_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.search_paths.push(path.into());
    }

    /// Load all agents from configured search paths
    ///
    /// Scans all search paths and loads valid agent JSON files.
    /// Individual agent load failures are logged but don't stop processing.
    ///
    /// # Returns
    /// Vector of successfully loaded agent configurations
    ///
    /// # Errors
    /// - Returns error only if ALL directories fail to scan (filesystem issues)
    /// - Individual agent parse errors are logged and skipped
    pub fn load_all(&self) -> Result<Vec<AgentConfig>> {
        let mut agents = Vec::new();
        let mut any_success = false;

        for search_path in &self.search_paths {
            if !search_path.exists() {
                tracing::debug!("Agent search path does not exist: {:?}", search_path);
                continue;
            }

            match self.load_from_directory(search_path) {
                Ok(dir_agents) => {
                    any_success = true;
                    agents.extend(dir_agents);
                }
                Err(e) => {
                    tracing::warn!("Failed to load agents from {:?}: {}", search_path, e);
                }
            }
        }

        if !any_success && !self.search_paths.is_empty() {
            tracing::warn!("No agents loaded from any search path");
        }

        Ok(agents)
    }

    /// Load agents from a specific directory
    ///
    /// Scans directory for `.json` files and attempts to load each as an agent config.
    ///
    /// # Errors
    /// - Directory read errors
    /// - Individual agent parse errors are logged but don't fail the entire load
    pub fn load_from_directory(&self, path: &Path) -> Result<Vec<AgentConfig>> {
        let mut agents = Vec::new();

        let entries = std::fs::read_dir(path)
            .with_context(|| format!("Failed to read agent directory: {:?}", path))?;

        for entry in entries {
            let entry = entry.with_context(|| {
                format!("Failed to read directory entry in {:?}", path)
            })?;

            let entry_path = entry.path();

            // Only process .json files
            if entry_path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match self.load_agent(&entry_path) {
                Ok(agent) => {
                    tracing::info!("Loaded agent '{}' from {:?}", agent.id, entry_path);
                    agents.push(agent);
                }
                Err(e) => {
                    tracing::error!("Failed to load agent from {:?}: {}", entry_path, e);
                    // Continue processing other agents
                }
            }
        }

        Ok(agents)
    }

    /// Load a single agent from a JSON file
    ///
    /// # Errors
    /// - File I/O errors
    /// - JSON parsing errors
    /// - Environment variable resolution errors
    /// - Validation errors
    pub fn load_agent(&self, path: &Path) -> Result<AgentConfig> {
        // Parse JSON config
        let mut json_config = JsonAgentConfig::from_file(path)
            .with_context(|| format!("Failed to parse agent JSON from {:?}", path))?;

        // Resolve environment variables
        json_config.resolve_env_vars()
            .with_context(|| format!("Failed to resolve environment variables in {:?}", path))?;

        // Validate configuration
        json_config.validate()
            .with_context(|| format!("Invalid agent configuration in {:?}", path))?;

        // Convert to runtime AgentConfig
        let agent_config = self.json_to_agent_config(json_config)?;

        Ok(agent_config)
    }

    /// Convert JsonAgentConfig to runtime AgentConfig
    ///
    /// Maps JSON configuration to the runtime agent configuration format.
    ///
    /// Design Decision: Separate JSON config from runtime config
    ///
    /// Rationale: JsonAgentConfig is focused on serialization/deserialization,
    /// while AgentConfig is optimized for runtime use. This separation allows:
    /// 1. Independent evolution of config format and runtime structure
    /// 2. Validation and transformation layer between config and runtime
    /// 3. Legacy AgentConfig compatibility without modifying existing code
    ///
    /// Trade-off: Additional conversion step vs. direct deserialization to AgentConfig
    /// (chose conversion for flexibility and backward compatibility).
    fn json_to_agent_config(&self, json: JsonAgentConfig) -> Result<AgentConfig> {
        // Build full system message combining instruction and personality
        let mut system_parts = vec![json.instruction.clone()];
        if let Some(personality) = &json.personality {
            if !personality.is_empty() {
                system_parts.push(format!("\n## Personality\n\n{}", personality));
            }
        }

        Ok(AgentConfig {
            id: json.name.clone(),
            name: json.name,
            instructions: system_parts.join("\n"),
            personality: json.personality,
            model: json.model,
            enabled: json.enabled,
            is_primary: json.is_primary,
            web_search_enabled: json.capabilities.web_search,
            mcp_extensions: json.mcp_extensions,
        })
    }
}

impl Default for AgentLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_agent_json(name: &str, provider: &str, model: &str) -> String {
        format!(
            r#"{{
                "name": "{}",
                "provider": "{}",
                "model": "{}",
                "instruction": "Test instruction for {}"
            }}"#,
            name, provider, model, name
        )
    }

    #[test]
    fn test_agent_loader_creation() {
        let loader = AgentLoader::new();
        assert_eq!(loader.search_paths.len(), 2);
        assert!(loader.search_paths[0].ends_with("agents/presets"));
        assert!(loader.search_paths[1].ends_with("agents/custom"));
    }

    #[test]
    fn test_add_search_path() {
        let mut loader = AgentLoader::new();
        loader.add_search_path("/custom/path");
        assert_eq!(loader.search_paths.len(), 3);
        assert_eq!(loader.search_paths[2], PathBuf::from("/custom/path"));
    }

    #[test]
    fn test_load_agent_from_json_string() {
        let json = create_test_agent_json("test_agent", "ollama", "llama2");
        let json_config = JsonAgentConfig::from_json(&json).unwrap();

        let loader = AgentLoader::new();
        let agent_config = loader.json_to_agent_config(json_config).unwrap();

        assert_eq!(agent_config.id, "test_agent");
        assert_eq!(agent_config.name, "test_agent");
        assert_eq!(agent_config.model, "llama2");
        assert!(agent_config.instructions.contains("Test instruction"));
    }

    #[test]
    fn test_load_agent_with_personality() {
        let json = r#"{
            "name": "friendly_agent",
            "provider": "ollama",
            "model": "llama2",
            "instruction": "Be helpful",
            "personality": "Friendly and warm"
        }"#;

        let json_config = JsonAgentConfig::from_json(json).unwrap();
        let loader = AgentLoader::new();
        let agent_config = loader.json_to_agent_config(json_config).unwrap();

        assert_eq!(agent_config.personality, Some("Friendly and warm".to_string()));
        assert!(agent_config.instructions.contains("Be helpful"));
        assert!(agent_config.instructions.contains("Personality"));
    }

    #[test]
    fn test_load_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test agent files
        let agent1_json = create_test_agent_json("agent1", "ollama", "llama2");
        let agent2_json = create_test_agent_json("agent2", "ollama", "mistral");

        fs::write(temp_path.join("agent1.json"), agent1_json).unwrap();
        fs::write(temp_path.join("agent2.json"), agent2_json).unwrap();
        fs::write(temp_path.join("not_json.txt"), "ignored").unwrap();

        let loader = AgentLoader::new();
        let agents = loader.load_from_directory(temp_path).unwrap();

        assert_eq!(agents.len(), 2);
        assert!(agents.iter().any(|a| a.id == "agent1"));
        assert!(agents.iter().any(|a| a.id == "agent2"));
    }

    #[test]
    fn test_load_from_nonexistent_directory() {
        let loader = AgentLoader::new();
        let result = loader.load_from_directory(Path::new("/nonexistent/path"));

        assert!(result.is_err());
    }

    #[test]
    fn test_load_all_with_missing_directories() {
        let mut loader = AgentLoader::new();
        loader.search_paths.clear();
        loader.add_search_path("/definitely/does/not/exist");

        // Should not error, just return empty vec
        let agents = loader.load_all().unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[test]
    fn test_invalid_json_agent_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create one valid and one invalid agent
        let valid_json = create_test_agent_json("valid", "ollama", "llama2");
        let invalid_json = r#"{"name": "invalid", "missing": "required_fields"}"#;

        fs::write(temp_path.join("valid.json"), valid_json).unwrap();
        fs::write(temp_path.join("invalid.json"), invalid_json).unwrap();

        let loader = AgentLoader::new();
        let agents = loader.load_from_directory(temp_path).unwrap();

        // Only valid agent should load
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, "valid");
    }

    #[test]
    fn test_web_search_capability_mapping() {
        let json = r#"{
            "name": "search_agent",
            "provider": "ollama",
            "model": "llama2",
            "instruction": "Search specialist",
            "capabilities": {
                "webSearch": true
            }
        }"#;

        let json_config = JsonAgentConfig::from_json(json).unwrap();
        let loader = AgentLoader::new();
        let agent_config = loader.json_to_agent_config(json_config).unwrap();

        assert!(agent_config.web_search_enabled);
    }
}
