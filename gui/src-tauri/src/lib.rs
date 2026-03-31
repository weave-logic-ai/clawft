use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Tauri commands — thin wrappers that will call kernel APIs once integrated
// ---------------------------------------------------------------------------

/// Response envelope for all commands
#[derive(Serialize)]
struct CmdResponse<T: Serialize> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> CmdResponse<T> {
    fn success(data: T) -> Self {
        Self { ok: true, data: Some(data), error: None }
    }
}

#[allow(dead_code)]
fn err_response(msg: &str) -> CmdResponse<()> {
    CmdResponse { ok: false, data: None, error: Some(msg.to_string()) }
}

// -- Kernel status -----------------------------------------------------------

#[derive(Serialize)]
struct KernelStatus {
    version: String,
    uptime_secs: u64,
    process_count: u32,
    chain_height: u64,
    health: String,
}

#[tauri::command]
fn kernel_status() -> CmdResponse<KernelStatus> {
    // TODO: Wire to real kernel via ServiceApi
    CmdResponse::success(KernelStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0,
        process_count: 0,
        chain_height: 0,
        health: "starting".to_string(),
    })
}

// -- Agent management --------------------------------------------------------

#[derive(Deserialize)]
#[allow(dead_code)]
struct SpawnAgentArgs {
    agent_type: String,
    name: Option<String>,
}

#[derive(Serialize)]
struct SpawnResult {
    pid: u32,
    agent_id: String,
}

#[tauri::command]
fn spawn_agent(args: SpawnAgentArgs) -> CmdResponse<SpawnResult> {
    // TODO: Wire to Supervisor::spawn()
    CmdResponse::success(SpawnResult {
        pid: 1,
        agent_id: format!("{}-0", args.agent_type),
    })
}

#[tauri::command]
fn stop_agent(pid: u32) -> CmdResponse<bool> {
    // TODO: Wire to Supervisor::stop()
    let _ = pid;
    CmdResponse::success(true)
}

// -- Config ------------------------------------------------------------------

#[derive(Deserialize)]
#[allow(dead_code)]
struct SetConfigArgs {
    key: String,
    value: String,
    namespace: Option<String>,
}

#[tauri::command]
fn set_config(args: SetConfigArgs) -> CmdResponse<bool> {
    // TODO: Wire to ConfigService
    let _ = args;
    CmdResponse::success(true)
}

// -- Chain queries -----------------------------------------------------------

#[derive(Serialize)]
struct ChainEvent {
    seq: u64,
    kind: String,
    timestamp: String,
    hash: String,
}

#[tauri::command]
fn query_chain(from_seq: Option<u64>, limit: Option<u32>) -> CmdResponse<Vec<ChainEvent>> {
    // TODO: Wire to ExoChain query
    let _ = (from_seq, limit);
    CmdResponse::success(vec![])
}

// -- Service registration ----------------------------------------------------

#[derive(Deserialize)]
#[allow(dead_code)]
struct RegisterServiceArgs {
    name: String,
    service_type: String,
}

#[tauri::command]
fn register_service(args: RegisterServiceArgs) -> CmdResponse<bool> {
    // TODO: Wire to ServiceRegistry
    let _ = args;
    CmdResponse::success(true)
}

// -- Component generation (self-building thesis) ----------------------------

#[derive(Deserialize)]
struct GenerateComponentArgs {
    description: String,
}

#[derive(Serialize)]
struct GeneratedComponent {
    tsx_source: String,
    component_name: String,
}

/// Generate a React component from a text description.
/// This is the genesis of the self-building thesis: the kernel can produce
/// UI components that the shell renders at runtime.
#[tauri::command]
fn generate_component(args: GenerateComponentArgs) -> CmdResponse<GeneratedComponent> {
    let desc = &args.description;
    let component_name = "GeneratedWidget";

    // For now, produce a simple component that displays the user's text.
    // In production, this calls Weaver → LLM → validated TSX.
    let tsx_source = format!(
        r#"function {component_name}() {{
  return (
    <div className="p-4 rounded-lg border border-indigo-500/30 bg-indigo-500/10">
      <p className="text-sm text-indigo-300 mb-1 font-mono">Generated Component</p>
      <div className="text-gray-100">{desc}</div>
    </div>
  );
}}"#,
        component_name = component_name,
        desc = desc.replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;"),
    );

    CmdResponse::success(GeneratedComponent {
        tsx_source,
        component_name: component_name.to_string(),
    })
}

// ---------------------------------------------------------------------------
// App setup
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            kernel_status,
            spawn_agent,
            stop_agent,
            set_config,
            query_chain,
            register_service,
            generate_component,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WeftOS GUI");
}
