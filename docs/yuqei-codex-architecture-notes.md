# Codex 架构勘察报告

> 目的：为后续把 `/Users/yuqei/codex` 改造成长期个人 AI 开发工具提供工程地图，尤其标出多模型供应商改造的最小切入点。

## 1. 总体判断

Codex 当前是一个 Rust monorepo，主系统已经具备长期 AI agent runtime 的核心部件：

- CLI 多命令入口
- TUI 交互界面
- 非交互 exec 模式
- app-server 协议层
- thread/session 管理
- model provider 抽象
- tools registry/router/runtime
- MCP server/client
- plugin/skills/memory/multi-agent 基础设施
- sandbox/permission/approval 系统

因此，后续不需要从零搭建 agent runtime，应该在现有架构上做“渐进式增强”。

## 2. Workspace 和关键 crate

根 workspace：

```text
codex-rs/Cargo.toml
```

关键 crate：

```text
codex-rs/cli                  # `codex` 主命令入口
codex-rs/tui                  # 交互式 TUI
codex-rs/exec                 # 非交互 `codex exec` / review
codex-rs/core                 # agent runtime 核心
codex-rs/model-provider       # runtime model provider trait/实例
codex-rs/model-provider-info  # provider 配置结构和内置 provider catalog
codex-rs/models-manager       # 模型列表/模型 catalog 管理
codex-rs/app-server           # 本地/远程 UI 后端协议服务
codex-rs/app-server-protocol  # app-server JSON-RPC 协议类型
codex-rs/app-server-client    # app-server client / in-process client
codex-rs/mcp-server           # Codex 作为 MCP server 暴露能力
codex-rs/mcp-server           # MCP server 实现
codex-rs/skills               # skill 支持
codex-rs/plugin               # plugin 支持
codex-rs/sandboxing           # sandbox 相关
codex-rs/exec-server          # 执行环境服务
codex-rs/thread-store         # thread/session 持久化
codex-rs/state                # 状态数据库相关
```

## 3. CLI 入口

主二进制定义在：

```text
codex-rs/cli/Cargo.toml
```

关键位置：

```text
codex-rs/cli/src/main.rs
```

主入口：

```rust
fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        cli_main(arg0_paths).await?;
        Ok(())
    })
}
```

核心分发函数：

```rust
async fn cli_main(arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()>
```

命令枚举：

```rust
enum Subcommand {
    Exec(ExecCli),
    Review(ReviewArgs),
    Login(LoginCommand),
    Logout(LogoutCommand),
    Mcp(McpCli),
    Plugin(PluginCli),
    McpServer,
    AppServer(AppServerCommand),
    App(app_cmd::AppCommand),
    Completion(CompletionCommand),
    Update,
    Sandbox(SandboxArgs),
    Debug(DebugCommand),
    Execpolicy(ExecpolicyCommand),
    Apply(ApplyCommand),
    Resume(ResumeCommand),
    Fork(ForkCommand),
    Cloud(CloudTasksCli),
    ResponsesApiProxy(ResponsesApiProxyArgs),
    StdioToUds(StdioToUdsCommand),
    ExecServer(ExecServerCommand),
    Features(FeaturesCli),
}
```

重要分发：

```text
无子命令              -> run_interactive_tui(...) -> codex_tui::run_main(...)
codex exec            -> codex_exec::run_main(...)
codex review          -> codex_exec::run_main(...), command = Review
codex mcp-server      -> codex_mcp_server::run_main(...)
codex app-server      -> codex_app_server::run_main_with_transport(...)
codex resume/fork     -> 构造 TuiCli 后进入 TUI
```

后续如果要新增顶层命令，例如：

```text
codex provider
codex model
codex workflow
codex memory
```

切入点就是：

```text
codex-rs/cli/src/main.rs
```

## 4. TUI 入口

TUI crate：

```text
codex-rs/tui
```

入口文件：

```text
codex-rs/tui/src/lib.rs
codex-rs/tui/src/main.rs
```

`codex-tui` 独立二进制入口：

```rust
let exit_info = run_main(
    inner,
    arg0_paths,
    LoaderOverrides::default(),
    /*remote*/ None,
    /*remote_auth_token*/ None,
).await?;
```

真正 TUI 主入口：

```rust
pub async fn run_main(
    mut cli: Cli,
    arg0_paths: Arg0DispatchPaths,
    loader_overrides: LoaderOverrides,
    remote: Option<String>,
    remote_auth_token: Option<String>,
) -> std::io::Result<AppExitInfo>
```

TUI 启动大致流程：

1. 解析 remote / embedded app-server 模式
2. 处理 sandbox / approval CLI override
3. 读取 `~/.codex/config.toml`
4. 处理 `--oss` provider/model override
5. 加载 Config
6. 初始化 state db、日志、otel、feedback
7. 初始化 terminal / ratatui
8. 启动 embedded 或 remote app-server
9. onboarding / trust / login
10. resume/fork/session selection
11. 调用 `App::run(...)`

关键调用：

```rust
run_ratatui_app(...)
App::run(...)
```

保存位置：

```text
codex-rs/tui/src/lib.rs
```

## 5. App Server 架构

app-server 是未来桌面端/Web UI/远程控制最重要的层。

入口 crate：

```text
codex-rs/app-server
```

入口文件：

```text
codex-rs/app-server/src/main.rs
codex-rs/app-server/src/lib.rs
```

`main.rs` 解析：

```text
--listen stdio:// | unix:// | ws://IP:PORT | off
--session-source vscode 等
websocket auth 参数
```

主入口：

```rust
pub async fn run_main_with_transport(...)
pub async fn run_main_with_transport_options(...)
```

核心流程：

1. 创建 `EnvironmentManager`
2. 创建 transport channel
3. 创建 `ConfigManager`
4. 加载 config
5. 初始化 state db
6. 初始化 auth manager / cloud requirements
7. 初始化 feedback / log db / analytics
8. 创建 `MessageProcessor`
9. 接收 JSON-RPC 请求并分发
10. outbound router 发送响应和通知

核心 processor：

```text
codex-rs/app-server/src/message_processor.rs
```

`MessageProcessor::new(...)` 内部创建：

```rust
let thread_manager = Arc::new(ThreadManager::new(...));
```

然后创建多个 request processor：

```text
AccountRequestProcessor
AppsRequestProcessor
CatalogRequestProcessor
CommandExecRequestProcessor
ConfigRequestProcessor
DeviceKeyRequestProcessor
FeedbackRequestProcessor
FsRequestProcessor
GitRequestProcessor
InitializeRequestProcessor
MarketplaceRequestProcessor
McpRequestProcessor
PluginRequestProcessor
SearchRequestProcessor
ThreadGoalRequestProcessor
ThreadRequestProcessor
TurnRequestProcessor
WindowsSandboxRequestProcessor
```

请求分发位置：

```rust
ClientRequest::ThreadStart
ClientRequest::TurnStart
ClientRequest::ReviewStart
ClientRequest::ModelList
ClientRequest::ModelProviderCapabilitiesRead
ClientRequest::McpServerStatusList
ClientRequest::McpServerToolCall
...
```

后续做 Web UI / 桌面端时，建议优先接 app-server，而不是直接嵌入 TUI 或 core。

## 6. Agent Runtime 核心

核心 crate：

```text
codex-rs/core
```

入口导出：

```text
codex-rs/core/src/lib.rs
```

关键模块：

```text
agent
codex_thread
thread_manager
session
client
config
context_manager
tools
mcp
skills
plugins
sandboxing
exec_policy
rollout
thread_store
```

重要导出：

```rust
pub use codex_thread::CodexThread;
pub use thread_manager::ThreadManager;
pub use thread_manager::NewThread;
pub use client::ModelClient;
pub use client::ModelClientSession;
pub use client_common::Prompt;
pub use client_common::ResponseStream;
```

### ThreadManager

位置：

```text
codex-rs/core/src/thread_manager.rs
```

用途：管理 thread/session 生命周期。

关键方法：

```rust
pub async fn start_thread(&self, config: Config) -> CodexResult<NewThread>
pub async fn start_thread_with_tools(...)
pub async fn start_thread_with_options(...)
pub async fn resume_thread_from_rollout(...)
pub async fn resume_thread_with_history(...)
pub async fn fork_thread(...)
pub async fn get_thread(&self, thread_id: ThreadId) -> CodexResult<Arc<CodexThread>>
```

### CodexThread

位置：

```text
codex-rs/core/src/codex_thread.rs
```

用途：对外暴露一个 agent thread 的操作接口。

关键方法：

```rust
pub async fn submit(&self, op: Op) -> CodexResult<String>
pub async fn submit_with_trace(...)
pub async fn submit_with_id(&self, sub: Submission) -> CodexResult<()>
pub async fn agent_status(&self) -> AgentStatus
```

大致调用链：

```text
TUI/AppServer/Exec
  -> ThreadManager.start_thread / get_thread
  -> CodexThread.submit(Op)
  -> session / agent loop
  -> ModelClientSession.stream(...)
  -> ToolRouter / ToolRegistry dispatch tool calls
```

## 7. Model Provider 架构

这是第一阶段改造的最重要区域。

相关 crate：

```text
codex-rs/model-provider
codex-rs/model-provider-info
codex-rs/models-manager
codex-rs/core/src/client.rs
codex-rs/core/src/config/mod.rs
```

### provider 配置结构

位置：

```text
codex-rs/model-provider-info/src/lib.rs
```

核心结构：

```rust
pub struct ModelProviderInfo {
    pub name: String,
    pub base_url: Option<String>,
    pub env_key: Option<String>,
    pub env_key_instructions: Option<String>,
    pub experimental_bearer_token: Option<String>,
    pub auth: Option<ModelProviderAuthInfo>,
    pub aws: Option<ModelProviderAwsAuthInfo>,
    pub wire_api: WireApi,
    pub auth_style: AuthStyle,
    pub query_params: Option<HashMap<String, String>>,
    pub http_headers: Option<HashMap<String, String>>,
    pub env_http_headers: Option<HashMap<String, String>>,
    pub request_max_retries: Option<u64>,
    pub stream_max_retries: Option<u64>,
    pub stream_idle_timeout_ms: Option<u64>,
    pub websocket_connect_timeout_ms: Option<u64>,
    pub requires_openai_auth: bool,
    pub supports_websockets: bool,
}
```

已支持 wire protocol：

```rust
pub enum WireApi {
    Responses,
    AnthropicMessages,
}
```

这很关键：说明 Codex 已经具备 OpenAI Responses API 和 Anthropic Messages API 两套 wire path 的抽象基础。

### 内置 provider catalog

位置：

```rust
pub fn built_in_model_providers(openai_base_url: Option<String>) -> HashMap<String, ModelProviderInfo>
```

当前内置：

```text
openai
amazon-bedrock
ollama
lmstudio
```

备注：源码注释明确说默认不内置太多第三方 provider，用户可通过 `config.toml` 的 `model_providers` 添加。

### 配置合并

位置：

```rust
pub fn merge_configured_model_providers(...)
```

逻辑：

- built-in providers 先加载
- 用户 `model_providers` 扩展内置列表
- 大多数 built-in 不允许覆盖
- amazon-bedrock 只允许覆盖 aws profile/region

### Config 中的 provider

位置：

```text
codex-rs/core/src/config/mod.rs
```

关键字段：

```rust
pub model_provider_id: String,
pub model_provider: ModelProviderInfo,
pub model_providers: HashMap<String, ModelProviderInfo>,
```

provider 解析逻辑：

```rust
let model_providers = merge_configured_model_providers(
    built_in_model_providers(openai_base_url),
    cfg.model_providers,
)?;

let model_provider_id = model_provider
    .or(config_profile.model_provider)
    .or(cfg.model_provider)
    .unwrap_or_else(|| "openai".to_string());

let model_provider = model_providers
    .get(&model_provider_id)
    .ok_or_else(...)?
    .clone();
```

### runtime provider trait

位置：

```text
codex-rs/model-provider/src/provider.rs
```

核心 trait：

```rust
pub trait ModelProvider: fmt::Debug + Send + Sync {
    fn info(&self) -> &ModelProviderInfo;
    fn capabilities(&self) -> ProviderCapabilities;
    fn auth_manager(&self) -> Option<Arc<AuthManager>>;
    async fn auth(&self) -> Option<CodexAuth>;
    fn account_state(&self) -> ProviderAccountResult;
    async fn api_provider(&self) -> codex_protocol::error::Result<Provider>;
    async fn runtime_base_url(&self) -> codex_protocol::error::Result<Option<String>>;
    async fn api_auth(&self) -> codex_protocol::error::Result<SharedAuthProvider>;
    fn models_manager(... ) -> SharedModelsManager;
}
```

创建 runtime provider：

```rust
pub fn create_model_provider(
    provider_info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
) -> SharedModelProvider
```

当前特殊处理：

```text
amazon-bedrock -> AmazonBedrockModelProvider
其他 -> ConfiguredModelProvider
```

### ModelClient

位置：

```text
codex-rs/core/src/client.rs
```

`ModelClient::new(...)` 内部：

```rust
let model_provider = create_model_provider(provider_info, auth_manager);
```

每个 turn 创建：

```rust
pub fn new_session(&self) -> ModelClientSession
```

实际流式请求：

```rust
pub async fn stream(...)
```

根据 provider 的 `wire_api` 分发：

```rust
match wire_api {
    WireApi::Responses => stream_responses_websocket 或 stream_responses_api,
    WireApi::AnthropicMessages => stream_anthropic_messages_api,
}
```

这说明多 provider 的核心扩展点已经存在。第一阶段不应该重写 client，而应优先补 provider 配置和模型 catalog。

## 8. 多模型供应商改造的最小切入点

### 8.1 最小可用方式：只改配置，不改源码

因为 Codex 已支持：

```toml
model_provider = "xxx"

[model_providers.xxx]
name = "..."
base_url = "..."
env_key = "..."
wire_api = "responses" 或 "anthropic_messages"
```

所以第一步可以直接通过 `~/.codex/config.toml` 添加 OpenRouter/DeepSeek/Anthropic-compatible。

示例方向：

```toml
model_provider = "openrouter"
model = "anthropic/claude-sonnet-4.5"

[model_providers.openrouter]
name = "OpenRouter"
base_url = "https://openrouter.ai/api/v1"
env_key = "OPENROUTER_API_KEY"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
```

注意：是否能直接工作取决于 provider 是否兼容 OpenAI Responses API。很多 OpenAI-compatible 服务实际只兼容 Chat Completions，不兼容 Responses。当前 Codex 已移除 `wire_api = "chat"`，所以需要验证。

### 8.2 第一阶段源码改造建议

优先目标：不要大改 agent loop，只增强 provider 体验。

建议改造点：

1. `codex-rs/model-provider-info/src/lib.rs`
   - 增加常用第三方 provider factory
   - 增加 provider capability metadata
   - 可选增加 OpenRouter/DeepSeek/Gemini/Anthropic-compatible 预设

2. `codex-rs/core/src/config/mod.rs`
   - 改善 provider 选择错误信息
   - 支持更友好的 alias
   - 支持 project-level provider profile

3. `codex-rs/cli/src/main.rs`
   - 增加 `codex provider list/add/test/select`
   - 增加 `codex model list/select`

4. `codex-rs/tui/src/model_catalog.rs` / 相关 model UI
   - TUI 中显示 provider
   - 支持切换 provider/model

5. `codex-rs/models-manager`
   - 为无 models endpoint 的 provider 提供 static catalog
   - 支持手动配置 model list

6. `codex-rs/core/src/client.rs`
   - 暂不优先改；只有当 provider wire API 不兼容时再加新 wire adapter

### 8.3 关键风险

当前主要风险是：

```text
Codex 当前主要面向 OpenAI Responses API。
很多第三方“OpenAI-compatible”只支持 /v1/chat/completions，不支持 /v1/responses。
```

所以改造路线应该是：

```text
先验证 Responses-compatible provider
再验证 AnthropicMessages-compatible provider
最后如果必要，再新增 ChatCompletions wire adapter
```

如果未来要支持更多第三方模型，可能需要恢复/新增：

```rust
WireApi::ChatCompletions
```

但这不是第一步。

## 9. Tool 系统

工具系统位置：

```text
codex-rs/core/src/tools
```

核心文件：

```text
tools/router.rs       # 把模型 ResponseItem 转成 ToolCall
tools/registry.rs     # handler 注册、pre/post hook、dispatch
tools/context.rs      # ToolPayload、ToolInvocation、ToolOutput
tools/handlers/*      # 具体工具 handler
tools/runtimes/*      # shell/apply_patch/unified_exec runtime
tools/parallel.rs     # 并行工具调度
tools/spec.rs         # 工具规格
tools/sandboxing.rs   # 工具权限和 sandbox 逻辑
```

模型输出到工具执行链路：

```text
Model response item
  -> ToolRouter::build_tool_call(...)
  -> ToolCall { tool_name, call_id, payload }
  -> ToolRouter::dispatch_tool_call_with_code_mode_result(...)
  -> ToolRegistry::dispatch_any(...)
  -> 对应 ToolHandler.handle(...)
  -> ToolRuntime.run(...)
  -> output 转回 ResponseItem
```

已支持的 handler 类型包括：

```text
ApplyPatchHandler
LocalShellHandler
ShellHandler
ShellCommandHandler
ExecCommandHandler
McpHandler
PlanHandler
RequestUserInputHandler
RequestPermissionsHandler
ToolSearchHandler
ViewImageHandler
MultiAgents handlers
Goal handlers
```

Shell runtime：

```text
codex-rs/core/src/tools/runtimes/shell.rs
```

它负责：

- approval 判断
- sandbox first attempt
- network approval
- cached approval
- guardian review
- execute_env
- stdout streaming

因此，工具增强应该走 handler/runtime 插件式路线，不要绕过 ToolRegistry。

## 10. 权限、审批和沙箱

关键模块：

```text
codex-rs/core/src/tools/sandboxing.rs
codex-rs/core/src/tools/runtimes/shell.rs
codex-rs/core/src/exec_policy.rs
codex-rs/core/src/guardian.rs
codex-rs/core/src/config/permissions.rs
codex-rs/sandboxing
```

核心概念：

```text
SandboxPermissions
ExecApprovalRequirement
PermissionRequestPayload
ApprovalCtx
SandboxAttempt
SandboxOverride
GuardianApprovalRequest
ReviewDecision
```

当前 shell runtime 已经把审批、sandbox、network approval 接在一起。后续加新工具时要尽量复用这套机制。

## 11. MCP 架构

Codex 有两类 MCP 能力：

### 11.1 Codex 作为 MCP client

核心在：

```text
codex-rs/core/src/mcp.rs
codex-rs/core/src/session/mcp.rs
codex-rs/core/src/tools/handlers/mcp.rs
codex-rs/app-server/src/mcp_refresh.rs
codex-rs/app-server/src/request_processors/mcp.rs
```

配置来自：

```text
Config.mcp_servers
```

工具调用通过 `McpHandler` 进入。

### 11.2 Codex 作为 MCP server

入口：

```text
codex-rs/mcp-server/src/lib.rs
```

启动命令：

```text
codex mcp-server
```

主入口：

```rust
pub async fn run_main(arg0_paths, cli_config_overrides)
```

流程：

1. 加载 Config
2. 初始化 otel
3. 从 stdin 读 JSON-RPC
4. `MessageProcessor` 处理 MCP request
5. 输出 JSON-RPC 到 stdout

这可以用于让其他 agent/IDE 调用 Codex 的能力。

## 12. Exec / Review 非交互模式

crate：

```text
codex-rs/exec
```

入口：

```text
codex-rs/exec/src/lib.rs
codex-rs/exec/src/cli.rs
```

`codex exec` 和 `codex review` 都走这里。

主入口：

```rust
pub async fn run_main(cli: Cli, arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()>
```

`review` 实际被包装成 `exec` 的 `Review` command。

后续要做 `/review`、自动化检查、CI 模式，可以优先复用 exec crate。

## 13. Skills / Plugin / Memory / Multi-agent

已有基础：

```text
codex-rs/core/src/skills.rs
codex-rs/core/src/skills_watcher.rs
codex-rs/skills
codex-rs/plugin
codex-rs/core/src/plugins.rs
codex-rs/core/src/tools/handlers/multi_agents.rs
codex-rs/core/src/tools/handlers/multi_agents_v2.rs
codex-rs/core/src/config/mod.rs 中 Config.memories / agent_roles / agent limits
codex-rs/memories/*
```

这说明路线图中的 skills、memory、multi-agent 不需要从零做，应该先调研现有实现，再决定是增强还是重写上层体验。

## 14. 推荐后续开发顺序

### Step 1：验证无源码改动的 provider 配置

目标：确认当前 provider 抽象能支持哪些第三方模型。

验证列表：

1. OpenAI 默认
2. Ollama 内置
3. LM Studio 内置
4. OpenRouter with `wire_api = "responses"`
5. Anthropic-compatible with `wire_api = "anthropic_messages"`
6. DeepSeek：重点确认是否支持 Responses API

产出：

```text
docs/yuqei-provider-compatibility.md
```

### Step 2：补 provider 管理命令

新增：

```text
codex provider list
codex provider show <id>
codex provider add openrouter
codex provider add deepseek
codex provider test <id>
codex provider select <id>
```

切入点：

```text
codex-rs/cli/src/main.rs
codex-rs/core/src/config/edit.rs
codex-rs/model-provider-info/src/lib.rs
```

### Step 3：补 model 管理命令

新增：

```text
codex model list
codex model select <model>
codex model current
```

切入点：

```text
codex-rs/models-manager
codex-rs/model-provider
codex-rs/cli/src/main.rs
```

### Step 4：TUI 显示和切换 provider/model

切入点：

```text
codex-rs/tui/src/model_catalog.rs
codex-rs/tui/src/status.rs
codex-rs/tui/src/slash_command.rs
```

### Step 5：必要时新增 ChatCompletions adapter

只有当 OpenRouter/DeepSeek 等核心供应商无法通过 Responses/AnthropicMessages 跑通时再做。

可能位置：

```text
codex-rs/model-provider-info/src/lib.rs   # WireApi::ChatCompletions
codex-rs/core/src/client.rs               # stream_chat_completions_api
codex-rs/codex-api                        # API client support
codex-rs/protocol                         # response/event 映射
```

这是较大改造，不建议第一步做。

## 15. 当前最重要结论

1. Codex 已经有比较完整的 agent runtime，不要重写核心。
2. 第一个长期改造点应该是 model provider/config/model catalog，而不是 UI。
3. 当前 provider 支持 `Responses` 和 `AnthropicMessages`，但缺少 `ChatCompletions`，这是第三方模型兼容性的最大不确定性。
4. app-server 是未来桌面端/Web UI/远程控制的正确底座。
5. tools/permission/sandbox 架构已经比较完整，新工具应接入 ToolRegistry 和 ToolRuntime。
6. skills/plugin/memory/multi-agent 已有基础，后续应先调研现有能力，再做产品化增强。

## 16. 下一份建议文档

建议下一步产出：

```text
docs/yuqei-provider-compatibility.md
```

内容：

- 当前 `model_providers` 配置格式
- OpenAI / Ollama / LM Studio / OpenRouter / DeepSeek / Anthropic-compatible 的配置样例
- 每个 provider 是否支持 Responses API / Anthropic Messages API
- 能否通过 `codex exec "hello"` 跑通
- 需要源码改造的兼容点
