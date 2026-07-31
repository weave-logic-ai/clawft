use super::*;
use clawft_core::agent::spawn::{SpawnError, SpawnHandle, TaskOutcome};
use clawft_types::delegation::{DelegationConfig, DelegationRule, DelegationTarget};
use std::sync::Mutex;
use std::time::Duration;

fn make_reviewer_profile() -> AgentDefinition {
    AgentDefinition {
        name: "code-reviewer".into(),
        description: "Reviews code for correctness and style".into(),
        model: None,
        system_prompt: Some(
            "You are a careful code reviewer. Report issues as a bullet list.".into(),
        ),
        skills: vec!["code-review".into()],
        allowed_tools: vec!["read_file".into()],
        max_turns: Some(5),
        variables: Default::default(),
        source_path: None,
    }
}

fn agents_with_reviewer() -> Arc<AgentRegistry> {
    let mut reg = AgentRegistry::empty();
    reg.register(make_reviewer_profile());
    Arc::new(reg)
}

/// Mock specialist that returns a canned code-review (mock LLM).
struct MockReviewRunner;

#[async_trait]
impl SpecialistRunner for MockReviewRunner {
    async fn run(&self, profile: &AgentDefinition, task: &str) -> Result<String, ToolError> {
        Ok(format!(
            "## Code review by {}\n\n\
             Task: {task}\n\n\
             - No critical issues found\n\
             - Consider adding unit tests for edge cases\n\
             - LGTM with minor nits",
            profile.name
        ))
    }
}

#[derive(Default)]
struct MockSpawner {
    last_spec: Mutex<Option<SpawnSpec>>,
}

#[async_trait]
impl SubagentSpawner for MockSpawner {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnHandle, SpawnError> {
        *self.last_spec.lock().unwrap() = Some(spec.clone());
        Ok(SpawnHandle {
            task_id: "t-review-1".into(),
            child_conv_id: "sub:chat:1".into(),
            status: TaskStatus::Completed,
            outcome: Some(TaskOutcome {
                task_id: "t-review-1".into(),
                child_conv_id: "sub:chat:1".into(),
                status: TaskStatus::Completed,
                result: Some(format!("Spawned review of: {}", spec.goal)),
                finish_reason: Some("stop".into()),
                iterations: 1,
                group_id: None,
                error: None,
            }),
        })
    }
    async fn status(&self, _task_id: &str) -> Option<TaskOutcome> {
        None
    }
    async fn group_status(&self, _group_id: &str) -> Vec<TaskOutcome> {
        vec![]
    }
    async fn result(
        &self,
        _task_id: &str,
        _wait: Option<Duration>,
    ) -> Result<TaskOutcome, SpawnError> {
        Err(SpawnError::NotFound {
            task_id: "x".into(),
        })
    }
    async fn message(&self, _task_id: &str, _text: &str) -> Result<(), SpawnError> {
        Ok(())
    }
    async fn cancel(&self, _task_id: &str) -> Result<(), SpawnError> {
        Ok(())
    }
}

fn tool_with_runner() -> DelegateTaskTool {
    let config = DelegationConfig::default();
    let engine = Arc::new(DelegationEngine::new(config));
    let registry = Arc::new(ToolRegistry::new());
    DelegateTaskTool::new_with_options(
        None,
        engine,
        Vec::new(),
        registry,
        agents_with_reviewer(),
        None,
    )
    .with_specialist_runner(Arc::new(MockReviewRunner))
}

#[test]
fn schema_includes_agent_field() {
    let tool = tool_with_runner();
    let params = tool.parameters();
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["task"].is_object());
    assert!(params["properties"]["agent"].is_object());
    let desc = params["properties"]["agent"]["description"]
        .as_str()
        .unwrap();
    assert!(
        desc.contains("code-reviewer"),
        "schema should list available agents: {desc}"
    );
    let required = params["required"].as_array().unwrap();
    assert!(required.contains(&json!("task")));
    assert!(!required.contains(&json!("agent")));
}

#[test]
fn tool_name_and_description() {
    let tool = tool_with_runner();
    assert_eq!(tool.name(), "delegate_task");
    assert!(!tool.description().is_empty());
    assert!(tool.description().contains("specialist"));
}

/// WEFT-349 AC: chat agent invokes delegate_task with agent=code-reviewer
/// and receives review output (mock LLM via SpecialistRunner).
#[tokio::test]
async fn chat_agent_delegates_code_review_subtask() {
    let tool = tool_with_runner();
    let result = tool
        .execute(json!({
            "task": "Review crates/clawft-tools/src/delegate_tool.rs for panics and error handling",
            "agent": "code-reviewer"
        }))
        .await
        .expect("delegate_task should succeed");

    assert_eq!(result["status"], "delegated");
    assert_eq!(result["target"], "specialist");
    assert_eq!(result["agent"], "code-reviewer");
    let response = result["response"].as_str().unwrap();
    assert!(
        response.contains("Code review"),
        "expected review output, got: {response}"
    );
    assert!(response.contains("LGTM") || response.contains("No critical"));
    assert_eq!(result["profile"]["name"], "code-reviewer");
    assert!(
        result["profile"]["skills"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s == "code-review")
    );
}

#[tokio::test]
async fn unknown_specialist_errors() {
    let tool = tool_with_runner();
    let err = tool
        .execute(json!({
            "task": "do something",
            "agent": "does-not-exist"
        }))
        .await
        .expect_err("unknown agent must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("unknown specialist") || msg.contains("does-not-exist"),
        "got: {msg}"
    );
    assert!(msg.contains("code-reviewer"), "should hint available: {msg}");
}

#[tokio::test]
async fn specialist_via_spawner_returns_output() {
    let config = DelegationConfig::default();
    let engine = Arc::new(DelegationEngine::new(config));
    let registry = Arc::new(ToolRegistry::new());
    let spawner = Arc::new(MockSpawner::default());
    let tool = DelegateTaskTool::new_with_options(
        None,
        engine,
        Vec::new(),
        registry,
        agents_with_reviewer(),
        Some(spawner.clone() as Arc<dyn SubagentSpawner>),
    );

    let result = tool
        .execute(json!({
            "task": "review the PR",
            "agent": "code-reviewer"
        }))
        .await
        .expect("spawner path ok");

    assert_eq!(result["status"], "delegated");
    assert_eq!(result["target"], "specialist");
    assert_eq!(result["agent"], "code-reviewer");
    assert!(
        result["response"]
            .as_str()
            .unwrap()
            .contains("Spawned review")
    );

    let spec = spawner.last_spec.lock().unwrap().clone().expect("spec recorded");
    assert_eq!(spec.persona.as_deref(), Some("code-reviewer"));
    assert!(spec.await_completion);
    assert!(
        spec.goal.contains("careful code reviewer"),
        "goal should embed system prompt: {}",
        spec.goal
    );
    assert!(spec.skills.iter().any(|s| s == "code-review"));
}

#[tokio::test]
async fn unavailable_delegator_routes_local() {
    // Rule would route to Claude, but unhealthy/missing delegator forces Local.
    let config = DelegationConfig {
        claude_enabled: true,
        rules: vec![DelegationRule {
            pattern: r"(?i)deploy".into(),
            target: DelegationTarget::Claude,
        }],
        ..Default::default()
    };
    let delegator = Arc::new(
        ClaudeDelegator::new(&config, "test-key".into()).expect("delegator with non-empty key"),
    );
    delegator.set_healthy(false);
    assert!(!delegator.is_available());

    let engine = Arc::new(DelegationEngine::new(config));
    let registry = Arc::new(ToolRegistry::new());
    let tool = DelegateTaskTool::new(delegator, engine, Vec::new(), registry);

    let result = tool
        .execute(json!({ "task": "deploy the service" }))
        .await
        .expect("execute ok");
    assert_eq!(result["status"], "local");
    assert_eq!(result["claude_available"], false);
}

#[tokio::test]
async fn specialist_profile_loaded_from_toml_file() {
    // End-to-end: write agents/code-reviewer/agent.toml, discover, run.
    let dir = tempfile::tempdir().expect("tempdir");
    let agent_dir = dir.path().join("code-reviewer");
    std::fs::create_dir_all(&agent_dir).unwrap();
    std::fs::write(
        agent_dir.join("agent.toml"),
        r#"
name = "code-reviewer"
description = "TOML specialist"
system_prompt = "You review code carefully."
skills = ["code-review"]
"#,
    )
    .unwrap();

    let agents = Arc::new(
        AgentRegistry::discover(Some(dir.path()), None, Vec::new()).expect("discover"),
    );
    assert!(agents.get("code-reviewer").is_some());

    let config = DelegationConfig::default();
    let engine = Arc::new(DelegationEngine::new(config));
    let registry = Arc::new(ToolRegistry::new());
    let tool = DelegateTaskTool::new_with_options(
        None,
        engine,
        Vec::new(),
        registry,
        agents,
        None,
    )
    .with_specialist_runner(Arc::new(MockReviewRunner));

    let result = tool
        .execute(json!({
            "task": "review helpers.rs",
            "agent": "code-reviewer"
        }))
        .await
        .expect("toml specialist ok");
    assert_eq!(result["status"], "delegated");
    assert_eq!(result["agent"], "code-reviewer");
    assert!(
        result["response"]
            .as_str()
            .unwrap()
            .contains("Code review by code-reviewer")
    );
}
