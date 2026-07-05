//! Unit tests for the M4 subagent tools. Split out of
//! `subagent_tools.rs` to keep both files under the 500-line ceiling;
//! included as a white-box submodule via `#[path]` so it can reach the
//! module-private helpers (`spawn_err_to_result`, `MAX_GOAL_LEN`, …).

    use super::*;
    use clawft_core::agent::spawn::{SpawnBackend, SpawnHandle, TaskStatus};
    use std::sync::Mutex;

    /// Configurable mock spawner. Records the last spec it was handed and
    /// returns canned outcomes/errors per method.
    #[derive(Default)]
    struct MockSpawner {
        last_spec: Mutex<Option<SpawnSpec>>,
        spawn_result: Mutex<Option<Result<SpawnHandle, SpawnErrorKind>>>,
    }

    /// A cloneable stand-in for `SpawnError` (which is not `Clone`) so the
    /// mock can be seeded with an error to replay.
    #[derive(Clone)]
    enum SpawnErrorKind {
        Denied,
        Depth,
        Concurrency,
        Backend,
    }

    impl SpawnErrorKind {
        fn into_err(self) -> SpawnError {
            match self {
                Self::Denied => SpawnError::Denied {
                    reason: "policy said no".into(),
                },
                Self::Depth => SpawnError::DepthExceeded { depth: 4, max: 3 },
                Self::Concurrency => SpawnError::ConcurrencyExceeded { max: 5 },
                Self::Backend => SpawnError::BackendNotAvailable {
                    backend: "native".into(),
                    reason: "seam only".into(),
                },
            }
        }
    }

    fn ok_handle(status: TaskStatus, outcome: Option<TaskOutcome>) -> SpawnHandle {
        SpawnHandle {
            task_id: "t-1".into(),
            child_conv_id: "sub:P:1".into(),
            status,
            outcome,
        }
    }

    fn outcome(status: TaskStatus, result: Option<&str>) -> TaskOutcome {
        TaskOutcome {
            task_id: "t-1".into(),
            child_conv_id: "sub:P:1".into(),
            status,
            result: result.map(String::from),
            finish_reason: Some("stop".into()),
            iterations: 2,
            group_id: None,
            error: None,
        }
    }

    #[async_trait]
    impl SubagentSpawner for MockSpawner {
        async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnHandle, SpawnError> {
            *self.last_spec.lock().unwrap() = Some(spec);
            match self.spawn_result.lock().unwrap().take() {
                Some(Ok(h)) => Ok(h),
                Some(Err(kind)) => Err(kind.into_err()),
                None => Ok(ok_handle(TaskStatus::Running, None)),
            }
        }
        async fn status(&self, task_id: &str) -> Option<TaskOutcome> {
            if task_id == "t-1" {
                Some(outcome(TaskStatus::Running, None))
            } else {
                None
            }
        }
        async fn group_status(&self, group_id: &str) -> Vec<TaskOutcome> {
            if group_id == "g-1" {
                vec![
                    outcome(TaskStatus::Completed, Some("a")),
                    outcome(TaskStatus::Running, None),
                ]
            } else {
                vec![]
            }
        }
        async fn result(
            &self,
            task_id: &str,
            _wait: Option<Duration>,
        ) -> Result<TaskOutcome, SpawnError> {
            if task_id == "t-1" {
                Ok(outcome(TaskStatus::Completed, Some("4")))
            } else {
                Err(SpawnError::NotFound {
                    task_id: task_id.into(),
                })
            }
        }
        async fn message(&self, task_id: &str, _text: &str) -> Result<(), SpawnError> {
            if task_id == "t-1" {
                Ok(())
            } else {
                Err(SpawnError::NotFound {
                    task_id: task_id.into(),
                })
            }
        }
        async fn cancel(&self, _task_id: &str) -> Result<(), SpawnError> {
            Ok(())
        }
    }

    fn seed(err_or_handle: Result<SpawnHandle, SpawnErrorKind>) -> Arc<MockSpawner> {
        let m = MockSpawner::default();
        *m.spawn_result.lock().unwrap() = Some(err_or_handle);
        Arc::new(m)
    }

    // ── schema / names ──────────────────────────────────────────────────
    #[test]
    fn tool_names_and_schemas() {
        let m = Arc::new(MockSpawner::default());
        let spawn = AgentSpawnTool::new(m.clone());
        assert_eq!(spawn.name(), "agent_spawn");
        assert!(!spawn.description().is_empty());
        assert!(
            spawn.parameters()["required"]
                .as_array()
                .unwrap()
                .contains(&json!("goal"))
        );

        assert_eq!(TaskStatusTool::new(m.clone()).name(), "task_status");
        let result = TaskResultTool::new(m.clone());
        assert_eq!(result.name(), "task_result");
        assert!(
            result.parameters()["required"]
                .as_array()
                .unwrap()
                .contains(&json!("task_id"))
        );
        let msg = AgentMessageTool::new(m);
        assert_eq!(msg.name(), "agent_message");
        let msg_params = msg.parameters();
        let req = msg_params["required"].as_array().unwrap();
        assert!(req.contains(&json!("task_id")) && req.contains(&json!("message")));
    }

    // ── agent_spawn: happy paths ────────────────────────────────────────
    #[tokio::test]
    async fn spawn_await_true_returns_completed_result() {
        let handle = ok_handle(
            TaskStatus::Completed,
            Some(outcome(TaskStatus::Completed, Some("4"))),
        );
        let tool = AgentSpawnTool::new(seed(Ok(handle)));
        let out = tool
            .execute(json!({ "goal": "answer 2+2", "await": true }))
            .await
            .unwrap();
        assert_eq!(out["status"], "completed");
        assert_eq!(out["result"], "4");
        assert_eq!(out["finish_reason"], "stop");
        assert_eq!(out["task_id"], "t-1");
        assert_eq!(out["child_conv_id"], "sub:P:1");
    }

    #[tokio::test]
    async fn spawn_await_false_returns_running_no_result() {
        let tool = AgentSpawnTool::new(seed(Ok(ok_handle(TaskStatus::Running, None))));
        let out = tool.execute(json!({ "goal": "background job" })).await.unwrap();
        assert_eq!(out["status"], "running");
        assert!(out.get("result").is_none());
        assert_eq!(out["task_id"], "t-1");
    }

    #[tokio::test]
    async fn spawn_threads_args_into_spec() {
        let m = Arc::new(MockSpawner::default());
        let tool = AgentSpawnTool::new(m.clone());
        tool.execute(json!({
            "goal": "g",
            "persona": "researcher",
            "skills": ["search", "summarize"],
            "group_id": "g-42",
            "budget": { "iterations": 7 }
        }))
        .await
        .unwrap();
        let spec = m.last_spec.lock().unwrap().clone().unwrap();
        assert_eq!(spec.persona.as_deref(), Some("researcher"));
        assert_eq!(spec.skills, vec!["search", "summarize"]);
        assert_eq!(spec.group_id.as_deref(), Some("g-42"));
        assert_eq!(spec.budget.unwrap().iterations, Some(7));
        // No ambient loop context in tests → root spawn at depth 1.
        assert_eq!(spec.depth, 1);
        assert_eq!(spec.backend, SpawnBackend::Conversation);
    }

    // ── agent_spawn: validation + error mapping ─────────────────────────
    #[tokio::test]
    async fn spawn_missing_goal_is_invalid_args() {
        let tool = AgentSpawnTool::new(Arc::new(MockSpawner::default()));
        let err = tool.execute(json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn spawn_oversized_goal_is_invalid_args() {
        let tool = AgentSpawnTool::new(Arc::new(MockSpawner::default()));
        let big = "x".repeat(MAX_GOAL_LEN + 1);
        let err = tool.execute(json!({ "goal": big })).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn spawn_denied_returns_denied_value_not_error() {
        let tool = AgentSpawnTool::new(seed(Err(SpawnErrorKind::Denied)));
        let out = tool.execute(json!({ "goal": "g" })).await.unwrap();
        assert_eq!(out["denied"], true);
        assert_eq!(out["reason"], "policy said no");
    }

    #[tokio::test]
    async fn spawn_depth_cap_is_a_denial() {
        let tool = AgentSpawnTool::new(seed(Err(SpawnErrorKind::Depth)));
        let out = tool.execute(json!({ "goal": "g" })).await.unwrap();
        assert_eq!(out["denied"], true);
        assert!(out["reason"].as_str().unwrap().contains("depth"));
    }

    #[tokio::test]
    async fn spawn_concurrency_cap_is_a_runtime_error() {
        let tool = AgentSpawnTool::new(seed(Err(SpawnErrorKind::Concurrency)));
        let err = tool.execute(json!({ "goal": "g" })).await.unwrap_err();
        match err {
            ToolError::ExecutionFailed(m) => assert!(m.contains("max concurrent subagents")),
            other => panic!("expected ExecutionFailed, got {other}"),
        }
    }

    #[tokio::test]
    async fn spawn_backend_unavailable_is_a_runtime_error() {
        let tool = AgentSpawnTool::new(seed(Err(SpawnErrorKind::Backend)));
        let err = tool.execute(json!({ "goal": "g" })).await.unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
    }

    // ── task_status ─────────────────────────────────────────────────────
    #[tokio::test]
    async fn task_status_by_task_id() {
        let tool = TaskStatusTool::new(Arc::new(MockSpawner::default()));
        let out = tool.execute(json!({ "task_id": "t-1" })).await.unwrap();
        let tasks = out["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["status"], "running");
        assert_eq!(tasks[0]["iterations"], 2);
    }

    #[tokio::test]
    async fn task_status_unknown_task_is_empty_list() {
        let tool = TaskStatusTool::new(Arc::new(MockSpawner::default()));
        let out = tool.execute(json!({ "task_id": "nope" })).await.unwrap();
        assert_eq!(out["tasks"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn task_status_by_group_aggregates() {
        let tool = TaskStatusTool::new(Arc::new(MockSpawner::default()));
        let out = tool.execute(json!({ "group_id": "g-1" })).await.unwrap();
        assert_eq!(out["tasks"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn task_status_requires_exactly_one_selector() {
        let tool = TaskStatusTool::new(Arc::new(MockSpawner::default()));
        assert!(matches!(
            tool.execute(json!({})).await.unwrap_err(),
            ToolError::InvalidArgs(_)
        ));
        assert!(matches!(
            tool.execute(json!({ "task_id": "t", "group_id": "g" }))
                .await
                .unwrap_err(),
            ToolError::InvalidArgs(_)
        ));
    }

    // ── task_result ─────────────────────────────────────────────────────
    #[tokio::test]
    async fn task_result_returns_completed_outcome() {
        let tool = TaskResultTool::new(Arc::new(MockSpawner::default()));
        let out = tool.execute(json!({ "task_id": "t-1" })).await.unwrap();
        assert_eq!(out["status"], "completed");
        assert_eq!(out["result"], "4");
    }

    #[tokio::test]
    async fn task_result_missing_task_is_runtime_error() {
        let tool = TaskResultTool::new(Arc::new(MockSpawner::default()));
        let err = tool.execute(json!({ "task_id": "ghost" })).await.unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
    }

    #[tokio::test]
    async fn task_result_requires_task_id() {
        let tool = TaskResultTool::new(Arc::new(MockSpawner::default()));
        assert!(matches!(
            tool.execute(json!({})).await.unwrap_err(),
            ToolError::InvalidArgs(_)
        ));
    }

    // ── agent_message ───────────────────────────────────────────────────
    #[tokio::test]
    async fn agent_message_delivered() {
        let tool = AgentMessageTool::new(Arc::new(MockSpawner::default()));
        let out = tool
            .execute(json!({ "task_id": "t-1", "message": "hi" }))
            .await
            .unwrap();
        assert_eq!(out["delivered"], true);
    }

    #[tokio::test]
    async fn agent_message_missing_fields_is_invalid_args() {
        let tool = AgentMessageTool::new(Arc::new(MockSpawner::default()));
        assert!(matches!(
            tool.execute(json!({ "task_id": "t-1" })).await.unwrap_err(),
            ToolError::InvalidArgs(_)
        ));
    }

    #[tokio::test]
    async fn agent_message_unknown_task_is_runtime_error() {
        let tool = AgentMessageTool::new(Arc::new(MockSpawner::default()));
        let err = tool
            .execute(json!({ "task_id": "ghost", "message": "hi" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
    }

    #[test]
    fn tools_are_object_safe() {
        fn accepts(_t: &dyn Tool) {}
        let m = Arc::new(MockSpawner::default());
        accepts(&AgentSpawnTool::new(m.clone()));
        accepts(&TaskStatusTool::new(m.clone()));
        accepts(&TaskResultTool::new(m.clone()));
        accepts(&AgentMessageTool::new(m));
    }
