use crate::config::Config;
use crate::features::Feature;
use crate::protocol::SandboxPolicy;
use serde::Deserialize;
use serde::Serialize;

/// Base instructions for the orchestrator role.
const ORCHESTRATOR_PROMPT: &str = include_str!("../../templates/agents/orchestrator.md");
/// Base instructions for the worker role.
const WORKER_PROMPT: &str = include_str!("../../gpt-5.2-codex_prompt.md");
/// Default worker model override used by the worker role.
const WORKER_MODEL: &str = "gpt-5.2-codex";

/// Enumerated list of all supported agent roles.
const ALL_ROLES: [AgentRole; 3] = [
    AgentRole::Default,
    AgentRole::Orchestrator,
    AgentRole::Worker,
];

/// Hard-coded agent role selection used when spawning sub-agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Inherit the parent agent's configuration unchanged.
    Default,
    /// Coordination-only agent that delegates to workers.
    Orchestrator,
    /// Task-executing agent with a fixed model override.
    Worker,
}

/// Immutable profile data that drives per-agent configuration overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgentProfile {
    /// Optional base instructions override.
    pub base_instructions: Option<&'static str>,
    /// Optional model override.
    pub model: Option<&'static str>,
    /// Whether to force a read-only sandbox policy.
    pub read_only: bool,
}

impl AgentRole {
    /// Returns the string values used by JSON schema enums.
    pub fn enum_values() -> Vec<String> {
        ALL_ROLES
            .iter()
            .filter_map(|role| serde_json::to_string(role).ok())
            .collect()
    }

    /// Returns the hard-coded profile for this role.
    pub fn profile(self) -> AgentProfile {
        match self {
            AgentRole::Default => AgentProfile::default(),
            AgentRole::Orchestrator => AgentProfile {
                base_instructions: Some(ORCHESTRATOR_PROMPT),
                ..Default::default()
            },
            AgentRole::Worker => AgentProfile {
                base_instructions: Some(WORKER_PROMPT),
                model: Some(WORKER_MODEL),
                ..Default::default()
            },
        }
    }

    /// Applies this role's profile onto the provided config.
    pub fn apply_to_config(self, config: &mut Config) -> Result<(), String> {
        let profile = self.profile();
        if let Some(base_instructions) = profile.base_instructions {
            config.base_instructions = Some(base_instructions.to_string());
        }
        if let Some(model) = profile.model
            && (self != AgentRole::Worker || config.features.enabled(Feature::WorkerModelOverride))
        {
            config.model = Some(model.to_string());
        }
        if profile.read_only {
            config
                .sandbox_policy
                .set(SandboxPolicy::new_read_only_policy())
                .map_err(|err| format!("sandbox_policy is invalid: {err}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigBuilder;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    async fn test_config() -> (TempDir, Config) {
        let home = TempDir::new().expect("create temp dir");
        let config = ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .build()
            .await
            .expect("load default test config");
        (home, config)
    }

    #[tokio::test]
    async fn worker_role_does_not_override_model_when_feature_disabled() {
        let (_home, mut config) = test_config().await;
        config.model = Some("gpt-5.2".to_string());
        config.features.disable(Feature::WorkerModelOverride);

        AgentRole::Worker
            .apply_to_config(&mut config)
            .expect("apply worker role");

        assert_eq!(config.model.as_deref(), Some("gpt-5.2"));
    }

    #[tokio::test]
    async fn worker_role_overrides_model_when_feature_enabled() {
        let (_home, mut config) = test_config().await;
        config.model = Some("gpt-5.2".to_string());
        config.features.enable(Feature::WorkerModelOverride);

        AgentRole::Worker
            .apply_to_config(&mut config)
            .expect("apply worker role");

        assert_eq!(config.model.as_deref(), Some("gpt-5.2-codex"));
    }
}
