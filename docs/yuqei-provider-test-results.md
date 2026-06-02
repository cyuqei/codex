# Codex Provider 本机实测记录

> 目的：记录各大模型供应商在本机 `/Users/yuqei/codex` 中的实际跑通情况，包括基础连通性、streaming、工具调用、shell、文件修改、错误类型和后续改造结论。

## 1. 测试原则

1. 每个 provider 都必须记录实际测试日期、模型、配置、命令、结果。
2. 不只记录成功，也要完整记录失败原因。
3. 所有会修改文件的测试必须在临时目录或专门测试目录中执行。
4. API key 不写入本文档，只记录使用的环境变量名。
5. 如果 provider 需要手改 TOML 才能测试，也要记录；但产品化目标仍是未来通过 UI 完成配置。
6. 每个 provider 最后必须给出明确结论：可直接使用 / 部分可用 / 不可用 / 需要源码改造。

## 2. 测试环境

| 项目 | 内容 |
|---|---|
| 测试日期 | 2026-05-07 |
| 操作系统 | macOS |
| Codex 路径 | `/Users/yuqei/codex` |
| Codex 分支 | `main` |
| Codex commit | `cc84e6bc6d` |
| Rust 版本 | `rustc 1.95.0 (59807616e 2026-04-14)` |
| Cargo 版本 | `cargo 1.95.0 (f2d3ce0bd 2026-03-21)` |
| 默认 shell | zsh |
| 测试目录 | `/tmp/codex-provider-tests` |

获取环境信息建议命令：

```bash
git -C /Users/yuqei/codex branch --show-current
git -C /Users/yuqei/codex rev-parse --short HEAD
rustc --version
cargo --version
```

## 3. 统一测试项

每个 provider 尽量执行以下测试。

### 3.1 基础连通性

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Say hello in one sentence."
```

通过标准：

- 命令退出码为 0
- 返回正常自然语言
- 没有 auth、endpoint、schema、stream parse 错误

### 3.2 简单代码生成

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Write a TypeScript function that adds two numbers."
```

通过标准：

- 返回可读代码
- 没有 JSON/schema/tool call 错误

### 3.3 文件写入工具调用

仅在临时目录执行：

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Create a file hello.txt containing hello."
```

通过标准：

- agent 能触发文件修改
- 权限审批/沙箱逻辑正常
- `hello.txt` 实际存在且内容正确

### 3.4 Shell 工具调用

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Run pwd and summarize the result."
```

通过标准：

- 能触发 shell/local shell 工具
- shell 输出可被模型正确总结
- sandbox/approval 不异常

### 3.5 Streaming 稳定性

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Count from 1 to 100 with commas."
```

通过标准：

- streaming 输出稳定
- 不出现 stream parse error
- 不因 idle timeout 中断

### 3.6 工具调用兼容性

测试目标：确认模型/provider 是否支持 Codex 当前工具 schema。

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "List the files in the current directory and tell me which one looks like the README."
```

通过标准：

- 能正确调用文件/目录相关工具或 shell
- 能基于真实结果回答

## 4. 结果状态标记

使用以下状态：

| 状态 | 含义 |
|---|---|
| `PASS` | 完全通过 |
| `PARTIAL` | 部分通过，有限制 |
| `FAIL_AUTH` | 鉴权失败 |
| `FAIL_ENDPOINT` | endpoint 不兼容，例如 `/v1/responses` 不存在 |
| `FAIL_STREAM` | streaming/SSE 解析失败 |
| `FAIL_SCHEMA` | request/response/tool schema 不兼容 |
| `FAIL_TOOL` | 工具调用失败 |
| `FAIL_MODEL` | 模型不存在或模型能力不足 |
| `BLOCKED` | 缺少 key、本地服务未启动、暂时无法测试 |
| `NEEDS_ADAPTER` | 需要新增源码 adapter 才能支持 |

## 5. 总览表

| Provider | Model | Wire API | 基础连通 | 代码生成 | 文件写入 | Shell | Streaming | 工具兼容 | 结论 |
|---|---|---|---|---|---|---|---|---|---|
| OpenAI-compatible custom | `gpt-5.4` | responses | PASS | PASS | PASS | PASS | PASS | PASS | 自定义 base URL + API key 可跑通基础 Responses/SSE、shell、写文件、目录识别 |
| Ollama | 待填写 | responses | BLOCKED | BLOCKED | BLOCKED | BLOCKED | BLOCKED | BLOCKED | 本机服务未启动 |
| LM Studio | 待填写 | responses | BLOCKED | BLOCKED | BLOCKED | BLOCKED | BLOCKED | BLOCKED | 本机服务未启动 |
| OpenRouter | `anthropic/claude-sonnet-4.5` / `openrouter/owl-alpha` | responses | PARTIAL | PENDING | PASS | PASS | PASS | PASS | provider 本身兼容 `/v1/responses`，但原始目标模型受区域限制；替代模型 `openrouter/owl-alpha` 已通过 shell、写文件、目录识别 |
| DeepSeek | `deepseek-chat` | chat_completions | PASS | PASS | PASS | PASS | PASS | PASS | `chat_completions` adapter 已跑通真实文本、代码生成、shell、写文件、目录识别；旧 `/v1/responses` 仍然 404 |
| Anthropic-compatible | `doubao-seed-2.0-code` | anthropic_messages | PASS | PASS | PASS | PASS | PASS | PASS | 火山 Ark Coding endpoint 使用 base_url 追加 `/v1`、`x-api-key` 鉴权后可跑通基础、代码生成、文件写入、shell、streaming |

## 6. OpenAI 实测记录

### 6.1 配置

```toml
model_provider = "openai"
model = "待填写"
```

或自定义：

```toml
model_provider = "openai-custom"
model = "待填写"

[model_providers.openai-custom]
name = "OpenAI Custom"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
```

### 6.2 测试结果

| 测试项 | 命令 | 状态 | 结果/错误 |
|---|---|---|---|
| 基础连通性 | `env CODEX_HOME=/tmp/codex-provider-tests/codex-home codex exec --skip-git-repo-check "Say hello in one sentence."` | PASS | `openai-custom` + `gpt-5.4` 基础 Responses/SSE smoke test 成功，返回 `Hello.`。报告：`automation/reports/test-runs/20260507-173646-A003.log` |
| 简单代码生成 | `env CODEX_HOME=/tmp/codex-provider-tests/codex-home codex exec --skip-git-repo-check "Reply with only TypeScript code for a function add(a: number, b: number): number that returns their sum. Do not inspect files. Do not use tools."` | PASS | 直接返回期望 TypeScript 函数。报告：`automation/reports/test-runs/20260507-180224-A003-codegen-direct.log` |
| 文件写入 | `env CODEX_HOME=/tmp/codex-provider-tests/codex-home codex exec --skip-git-repo-check --sandbox workspace-write "Create a file hello.txt containing hello."` | PASS | 成功调用 patch 写入 `hello.txt`，并读回确认内容。报告：`automation/reports/test-runs/20260507-180434-A003-filewrite.log` |
| Shell 调用 | `env CODEX_HOME=/tmp/codex-provider-tests/codex-home codex exec --skip-git-repo-check "Run pwd and summarize the result."` | PASS | 成功调用 shell 并正确总结 `pwd` 结果。报告：`automation/reports/test-runs/20260507-180255-A003-shell.log` |
| Streaming | `env CODEX_HOME=/tmp/codex-provider-tests/codex-home codex exec --skip-git-repo-check "Count from 1 to 100 with commas."` | PASS | 长输出稳定返回，无断流。报告：`automation/reports/test-runs/20260507-180354-A003-stream.log` |
| 工具兼容 | `env CODEX_HOME=/tmp/codex-provider-tests/codex-home codex exec --skip-git-repo-check --sandbox read-only "List the files in the current directory and tell me which one looks like the README."` | PASS | 成功列目录并识别 `README.md`。报告：`automation/reports/test-runs/20260507-180513-A003-tool.log` |

### 6.3 结论

当前 OpenAI-compatible custom provider 为 `PASS`。使用 `OPENAI_BASE_URL` 指向的自定义 base URL、`OPENAI_API_KEY`、`model = "gpt-5.4"`，以及临时 provider `openai-custom` 设置 `wire_api = "responses"`、`supports_websockets = false` 后，基础连通、代码生成、文件写入、shell、streaming、工具兼容全部通过。后续如要验证 WebSocket，可单独追加测试，但当前不影响 Responses/SSE 路径可用性。

## 7. Ollama 实测记录

### 7.1 前置条件

```bash
ollama serve
ollama pull gpt-oss:20b
```

### 7.2 配置

```toml
model_provider = "ollama"
model = "gpt-oss:20b"
```

或自定义：

```toml
model_provider = "ollama-custom"
model = "gpt-oss:20b"

[model_providers.ollama-custom]
name = "Ollama Custom"
base_url = "http://localhost:11434/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
```

### 7.3 测试结果

| 测试项 | 命令 | 状态 | 结果/错误 |
|---|---|---|---|
| 基础连通性 | `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check "Say hello in one sentence."` | PARTIAL | `anthropic/claude-sonnet-4.5` 命中 `https://openrouter.ai/api/v1/responses`，但返回 `403 Forbidden: This model is not available in your region.` |
| 替代模型 smoke | `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec -c model="openrouter/owl-alpha" --skip-git-repo-check "Say hello in one sentence."` | PASS | 同一 provider、同一 `wire_api = "responses"` 下成功返回 `Hello! 👋 How can I help you today?` |
| 文件写入 | `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec -c model="openrouter/owl-alpha" --skip-git-repo-check --sandbox workspace-write "Create a file hello.txt containing hello."` | PASS | 成功调用 shell 写入 `hello.txt`，并确认文件内容为 `hello`。报告：`automation/reports/test-runs/20260509-011407-A014-openrouter-filewrite.log` |
| Shell 调用 | `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec -c model="openrouter/owl-alpha" --skip-git-repo-check --sandbox read-only "Run pwd and summarize the result."` | PASS | 成功调用 shell 并正确总结当前工作目录。报告：`automation/reports/test-runs/20260509-011326-A014-openrouter-shell.log` |
| Streaming | 同上两个 smoke 命令，以及 `... "Count from 1 to 30 with commas."` | PASS | 默认模型在 provider 返回 403 前多次自动重连；替代模型 `openrouter/owl-alpha` 的长输出稳定完成。报告：`automation/reports/test-runs/20260509-011326-A014-openrouter-stream.log` |
| 工具兼容 | `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec -c model="openrouter/owl-alpha" --skip-git-repo-check --sandbox read-only "List the files in the current directory and tell me which one looks like the README."` | PASS | 成功列目录并识别 `README.md`。中途发生一次自动重连，但 turn 最终成功完成。报告：`automation/reports/test-runs/20260509-011407-A014-openrouter-tool.log` |

### 7.4 结论

待填写。

## 8. LM Studio 实测记录

### 8.1 前置条件

- 启动 LM Studio local server
- 确认监听：`http://localhost:1234/v1`
- 确认模型名称

### 8.2 配置

```toml
model_provider = "lmstudio"
model = "待填写"
```

或自定义：

```toml
model_provider = "lmstudio-custom"
model = "待填写"

[model_providers.lmstudio-custom]
name = "LM Studio Custom"
base_url = "http://localhost:1234/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
```

### 8.3 测试结果

| 测试项 | 命令 | 状态 | 结果/错误 |
|---|---|---|---|
| 基础连通性 | 待填写 | 待测 | 待填写 |
| 简单代码生成 | 待填写 | 待测 | 待填写 |
| 文件写入 | 待填写 | 待测 | 待填写 |
| Shell 调用 | 待填写 | 待测 | 待填写 |
| Streaming | 待填写 | 待测 | 待填写 |
| 工具兼容 | 待填写 | 待测 | 待填写 |

### 8.4 结论

待填写。

## 9. OpenRouter 实测记录

### 9.1 配置

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
request_max_retries = 4
stream_max_retries = 5
stream_idle_timeout_ms = 300000

[model_providers.openrouter.http_headers]
HTTP-Referer = "https://local.codex"
X-Title = "Codex Local"
```

### 9.2 测试结果

| 测试项 | 命令 | 状态 | 结果/错误 |
|---|---|---|---|
| 基础连通性 | 待填写 | 待测 | 待填写 |
| 简单代码生成 | 待填写 | 待测 | 待填写 |
| 文件写入 | 待填写 | 待测 | 待填写 |
| Shell 调用 | 待填写 | 待测 | 待填写 |
| Streaming | 待填写 | 待测 | 待填写 |
| 工具兼容 | 待填写 | 待测 | 待填写 |

### 9.3 结论

OpenRouter 当前结果应记为 `PARTIAL`。结论不是 provider 不兼容，而是更细的一层：OpenRouter 的 `/v1/responses` 路径可以被 Codex 正常调用，但你当前测试目标模型 `anthropic/claude-sonnet-4.5` 在这个账号/区域下返回 `403 Forbidden: This model is not available in your region.`。同一配置下改用 `openrouter/owl-alpha` 后，真实文本、streaming、shell、文件写入、目录识别都通过，说明 OpenRouter provider 与 `wire_api = "responses"` 是兼容的。

下一步如果要把 OpenRouter 提升到更强结论，应选一个该账号/区域可用、且更接近目标产品定位的模型，继续补跑更复杂的多步工具调用工作流。当前 provider 级别兼容性已经基本确认，剩余问题集中在模型选择与区域可用性。

## 10. DeepSeek 实测记录

### 10.1 配置

```toml
model_provider = "deepseek"
model = "deepseek-chat"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
env_key = "DEEPSEEK_API_KEY"
wire_api = "chat_completions"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 4
stream_max_retries = 5
stream_idle_timeout_ms = 300000
```

可选 reasoner 模型：

```toml
model = "deepseek-reasoner"
```

### 10.2 测试结果

| 测试项 | 命令 | 状态 | 结果/错误 |
|---|---|---|---|
| 基础连通性 | `POST https://api.deepseek.com/v1/responses` with `deepseek-chat`；对照 `POST https://api.deepseek.com/v1/chat/completions` | FAIL_ENDPOINT | `/v1/responses` 返回 HTTP 404；同一 key、同一模型下 `/v1/chat/completions` 返回 HTTP 200 并包含 `choices`。 |
| 文本 smoke | `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home cargo run -p codex-cli -- exec --skip-git-repo-check "Say hello in one sentence."` | PASS | Source-built `codex-cli` 通过 `wire_api = "chat_completions"` 成功返回 `Hey! 👋`。 |
| 代码生成 | `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check "Reply with only TypeScript code for a function add(a: number, b: number): number that returns their sum. Do not inspect files. Do not use tools."` | PASS | 直接返回期望 TypeScript 函数。报告：`automation/reports/test-runs/20260509-083511-A013-deepseek-codegen.log` |
| 文件写入 | `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check --sandbox workspace-write "Create a file hello.txt containing hello."` | PASS | 成功调用 shell 写入 `hello.txt`，并确认文件内容为 `hello`。报告：`automation/reports/test-runs/20260509-011245-A013-deepseek-filewrite.log` |
| Shell 调用 | `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check --sandbox read-only "Run pwd and summarize the result."` | PASS | 成功调用 shell 并正确总结当前工作目录。报告：`automation/reports/test-runs/20260509-011225-A013-deepseek-shell.log` |
| Streaming | `... "Say hello in one sentence."` 与 `... "Count from 1 to 30 with commas."` | PASS | 两次请求都返回为正常流式 assistant 输出，turn 完整结束。报告：`automation/reports/test-runs/20260509-011225-A013-deepseek-stream.log` |
| 工具兼容 | `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check --sandbox read-only "List the files in the current directory and tell me which one looks like the README."` | PASS | 成功列目录并识别 `README.md`。报告：`automation/reports/test-runs/20260509-011245-A013-deepseek-tool.log` |

### 10.3 结论

DeepSeek 已从 `NEEDS_ADAPTER` 进入 `WORKFLOW_PASS`。真实 key 已验证可用，DeepSeek 仍然不支持 Codex 旧的 `wire_api = "responses"` 所需 `/v1/responses` endpoint，但当前分支新增的 `WireApi::ChatCompletions` 已经能让 source-built `codex-cli` 通过 `deepseek-chat` 完成基础文本、代码生成、streaming、shell、文件写入和目录识别。对当前 adapter MVP 来说，最关键的真实 provider 闭环已经成立。

补充说明：当前 `/Applications/Codex.app` 自带的 `codex` 二进制仍不识别 `wire_api = "chat_completions"`，因此本次验证对象是当前源码工作树构建出的 `codex-cli`，不是已安装桌面 App 内置 CLI。

## 11. Anthropic-compatible 实测记录

### 11.1 配置

```toml
model_provider = "anthropic-compatible"
model = "doubao-seed-2.0-code"

[model_providers.anthropic-compatible]
name = "Anthropic Compatible"
base_url = "$ANTHROPIC_BASE_URL/v1"
env_key = "ANTHROPIC_API_KEY"
wire_api = "anthropic_messages"
auth_style = "x_api_key"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 1
stream_max_retries = 1
stream_idle_timeout_ms = 30000

[model_providers.anthropic-compatible.http_headers]
anthropic-version = "2023-06-01"
```

### 11.2 测试结果

| 测试项 | 命令 | 状态 | 结果/错误 |
|---|---|---|---|
| 基础连通性 | `POST $ANTHROPIC_BASE_URL/v1/messages` + `x-api-key` + `anthropic-version`；`CODEX_HOME=/tmp/codex-provider-tests/anthropic-v1-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check "Say hello in one sentence." </dev/null` | PASS | 原始 HTTP 非流式返回 200；Codex smoke test 返回正常自然语言。关键修正：Codex 会在 `base_url` 后拼 `messages`，所以 provider `base_url` 必须配置为现有 Ark Coding URL 再追加 `/v1`。 |
| 简单代码生成 | `codex exec --skip-git-repo-check --sandbox read-only "Reply with only TypeScript code for function add(a: number, b: number): number. Do not use tools." </dev/null` | PASS | 返回 TypeScript `add` 函数，无 schema 错误。 |
| 文件写入 | `codex exec --skip-git-repo-check --sandbox workspace-write "Create a file hello.txt containing hello." </dev/null` | PASS | 成功调用 shell 写入 `hello.txt`。一次测试误写到 repo 根目录，已立即删除测试文件；后续写入测试应显式设置临时工作目录。 |
| Shell 调用 | `codex exec --skip-git-repo-check --sandbox read-only "Run pwd and summarize the result." </dev/null` | PASS | 成功调用 shell 并总结当前工作目录。 |
| Streaming | 原始 HTTP `stream=true`；以及 `codex exec "Count from 1 to 30 with commas. Do not use tools." </dev/null` | PASS | 原始 SSE 返回 `message_start`、`content_block_delta`、`message_delta`、`message_stop`；Codex 长输出稳定完成。 |
| 工具兼容 | 原始 HTTP 请求带最小 `tools` schema；Codex shell/file-write 工具测试 | PASS | provider 接受 Anthropic tool schema 并返回 `message_stop`；Codex 可完成 shell 和文件写入类工具调用。 |

### 11.3 结论

当前 Anthropic-compatible provider 为 `PASS`。用户提供的 Ark Coding 配置可用，关键是 Codex provider 的 `base_url` 要写成现有 `ANTHROPIC_BASE_URL` 再追加 `/v1`，因为 Codex 的 `anthropic_messages` client 会固定拼接 `messages`。鉴权使用 `auth_style = "x_api_key"`，本地测试时将 `ANTHROPIC_AUTH_TOKEN` 映射到 provider `env_key = "ANTHROPIC_API_KEY"`。基础连通、代码生成、streaming、shell、文件写入和最小 tool schema 均已通过。

## 12. 是否需要 ChatCompletions adapter 的决策记录

### 12.1 触发条件

如果以下 provider 主要失败原因都是 `/v1/responses` 不存在或不兼容：

- OpenRouter
- DeepSeek
- Ollama
- LM Studio

则应将新增 ChatCompletions adapter 提升为高优先级。

### 12.2 决策

当前决策：B. 需要但不紧急。现有已完成证据表明，自定义 OpenAI-compatible GPT-5.4 endpoint 在 `wire_api = "responses"`、`supports_websockets = false` 下已完整通过基础连通、代码生成、文件写入、shell、streaming、工具兼容测试。Ollama/LM Studio 仍因本地服务未启动未实测，OpenRouter/DeepSeek 仍因缺少 key 未实测，因此还没有足够证据把 ChatCompletions adapter 提升为当前最高优先级。只有当这些关键 provider 明确失败在 `/v1/responses` 缺失或不兼容时，再升级到 C。

可选结论：

```text
A. 暂不需要：Responses / AnthropicMessages 已覆盖核心供应商。
B. 需要但不紧急：少数 provider 需要 ChatCompletions，可后续做。
C. 高优先级需要：OpenRouter/DeepSeek 等关键 provider 无法使用，必须新增 ChatCompletions adapter。
```

### 12.3 依据

- 已验证的自定义 OpenAI-compatible GPT-5.4 endpoint 能稳定跑通 `responses` 路径，说明当前 Codex 不被限制在官方 OpenAI provider。
- OpenRouter/DeepSeek/Ollama/LM Studio 仍未产生 `/v1/responses` 缺失证据；当前阻塞来自本地服务未启动或缺少 key，而不是已确认的 wire API 不兼容。
- 因此 ChatCompletions adapter 不是当前最高确定性下一步，优先级继续保持在 B。

## 13. UI-first 配置体验验证记录

长期产品目标：大模型接口配置必须能在界面中完成，而不是要求用户手改配置文件。

当前阶段如果仍需要手改配置文件，应记录为产品体验缺口。

| 能力 | 当前状态 | 目标体验 | 缺口 |
|---|---|---|---|
| 新增 provider | 待填写 | UI 表单新增 | 待填写 |
| 编辑 provider | 待填写 | UI 表单编辑 | 待填写 |
| 删除 provider | 待填写 | UI 删除/禁用 | 待填写 |
| 设置默认 provider | 待填写 | 下拉选择 | 待填写 |
| 设置默认 model | 待填写 | 搜索/下拉选择 | 待填写 |
| 输入 API key | 待填写 | 安全输入 + keyring/env 提示 | 待填写 |
| 测试连接 | 待填写 | 一键测试并展示诊断 | 待填写 |
| 项目级覆盖 | 待填写 | 项目设置页 | 待填写 |
| 导入/导出配置 | 待填写 | UI 导入导出 | 待填写 |

## 14. 后续行动项

- [ ] 填写测试环境信息
- [ ] 实测 OpenAI
- [ ] 实测 Ollama
- [ ] 实测 LM Studio
- [ ] 实测 OpenRouter
- [ ] 实测 DeepSeek
- [ ] 实测 Anthropic-compatible
- [ ] 根据失败类型判断是否需要 ChatCompletions adapter
- [ ] 设计 UI-first provider 配置页
- [ ] 设计 `codex provider test` 命令，作为 UI 测试连接能力的后端
