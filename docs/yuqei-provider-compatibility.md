# Codex Provider 兼容性文档

> 目的：记录 Codex 当前 `model_providers` 配置方式，给出 OpenAI、Ollama、LM Studio、OpenRouter、DeepSeek、Anthropic-compatible 的配置样例，并设计兼容性验证方案。

## 1. 当前结论

Codex 当前 provider 架构已经支持自定义供应商，但重点限制是：

1. 支持的 `wire_api` 只有：
   - `responses`
   - `anthropic_messages`
2. 已不支持旧式 `chat` wire API。
3. 很多第三方所谓 OpenAI-compatible 服务只兼容 `/v1/chat/completions`，不一定兼容 `/v1/responses`。
4. 因此，第三方供应商能否直接接入，关键取决于它是否支持：
   - OpenAI Responses API；或
   - Anthropic Messages API。
5. 如果核心供应商只支持 Chat Completions，后续需要新增 `WireApi::ChatCompletions`，这属于较大改造，不建议第一步做。

## 2. 相关源码位置

Provider 配置结构：

```text
codex-rs/model-provider-info/src/lib.rs
```

Runtime provider trait：

```text
codex-rs/model-provider/src/provider.rs
```

Config 加载与 provider 选择：

```text
codex-rs/core/src/config/mod.rs
```

ModelClient 根据 `wire_api` 分发请求：

```text
codex-rs/core/src/client.rs
```

现有配置文档：

```text
docs/config.md
```

## 3. model_providers 配置字段

### 3.1 产品化要求：UI-first

本节虽然记录 TOML 配置格式，但长期产品目标不是让用户手改配置文件。

产品化要求：

- 普通用户应在界面中完成所有大模型接口配置。
- 配置文件只作为高级用户、调试、备份、导入导出机制存在。
- UI 必须支持新增/编辑/删除 provider、设置默认 provider/model、项目级覆盖、测试连接。
- API key、base URL、auth style、wire API、headers、model list 等都应有表单，不应要求用户记 TOML 字段。
- 保存配置前应提供“测试连接”能力，并给出清晰诊断：环境变量缺失、鉴权失败、endpoint 不兼容、模型不存在、streaming 不兼容、tool calling 不兼容。
- 对常见 provider 应提供模板：OpenAI、OpenRouter、DeepSeek、Anthropic-compatible、Ollama、LM Studio。

### 3.2 底层 TOML 格式

基础格式：

```toml
model_provider = "provider_id"
model = "model-name"

[model_providers.provider_id]
name = "Display Name"
base_url = "https://provider.example.com/v1"
env_key = "PROVIDER_API_KEY"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
```

核心字段：

```toml
name = "..."                         # 显示名称
base_url = "..."                     # API base URL
env_key = "..."                      # API key 环境变量
wire_api = "responses"               # responses 或 anthropic_messages
auth_style = "bearer"                # bearer 或 x_api_key
requires_openai_auth = false          # 第三方 provider 通常 false
supports_websockets = false           # 第三方 provider 通常 false
request_max_retries = 4
stream_max_retries = 5
stream_idle_timeout_ms = 300000
websocket_connect_timeout_ms = 15000
```

可选 headers：

```toml
[model_providers.provider_id.http_headers]
Header-Name = "literal-value"

[model_providers.provider_id.env_http_headers]
Header-Name = "ENV_VAR_NAME"
```

注意：

- `env_key` 会从环境变量读取 API key。
- `experimental_bearer_token` 可以直接写 token，但不建议长期使用。
- `auth_style = "bearer"` 表示 `Authorization: Bearer <token>`。
- `auth_style = "x_api_key"` 表示 `x-api-key: <token>`，常用于 Anthropic。
- `requires_openai_auth = true` 会走 OpenAI/ChatGPT 登录逻辑，第三方一般不要开。

## 4. 内置 provider

当前内置 provider 包括：

```text
openai
amazon-bedrock
ollama
lmstudio
```

内置 OSS provider：

```text
ollama  -> 默认 http://localhost:11434/v1
lmstudio -> 默认 http://localhost:1234/v1
```

相关常量：

```text
DEFAULT_OLLAMA_PORT = 11434
DEFAULT_LMSTUDIO_PORT = 1234
```

## 5. OpenAI 配置

### 5.1 默认 OpenAI

默认情况下可使用内置 `openai` provider：

```toml
model_provider = "openai"
model = "gpt-5.1-codex-max"
```

认证方式通常通过：

```bash
codex login
```

或者官方支持的 API key 登录方式。

### 5.2 自定义 OpenAI-compatible Responses provider

如果你想显式使用 API key 环境变量：

```toml
model_provider = "openai-custom"
model = "gpt-5.1-codex-max"

[model_providers.openai-custom]
name = "OpenAI Custom"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 4
stream_max_retries = 5
stream_idle_timeout_ms = 300000
```

环境变量：

```bash
export OPENAI_API_KEY="..."
```

验证：

```bash
codex exec "Say hello in one sentence."
```

预期：

- 能正常返回文本。
- 如果报 `/v1/responses` 不存在，说明 provider/base_url 不兼容 Responses API。

## 6. Ollama 配置

### 6.1 使用内置 ollama provider

Codex 已内置：

```toml
model_provider = "ollama"
model = "gpt-oss:20b"
```

Ollama 默认地址：

```text
http://localhost:11434/v1
```

启动 Ollama：

```bash
ollama serve
```

拉取模型：

```bash
ollama pull gpt-oss:20b
```

验证：

```bash
codex exec --oss "Say hello in one sentence."
```

或显式指定：

```bash
codex exec -c model_provider=\"ollama\" -c model=\"gpt-oss:20b\" "Say hello in one sentence."
```

### 6.2 自定义 Ollama base_url

如果 Ollama 不在默认端口：

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

也可用环境变量影响内置 OSS provider：

```bash
export CODEX_OSS_BASE_URL="http://localhost:11434/v1"
```

风险：

- Ollama 的 OpenAI-compatible 接口是否完整支持 Responses API 需要实测。
- 如果只支持 Chat Completions，则需要新增 ChatCompletions adapter。

## 7. LM Studio 配置

### 7.1 使用内置 lmstudio provider

Codex 已内置：

```toml
model_provider = "lmstudio"
model = "local-model-name"
```

LM Studio 默认地址：

```text
http://localhost:1234/v1
```

验证：

```bash
codex exec -c model_provider=\"lmstudio\" -c model=\"local-model-name\" "Say hello in one sentence."
```

### 7.2 自定义 LM Studio provider

```toml
model_provider = "lmstudio-custom"
model = "local-model-name"

[model_providers.lmstudio-custom]
name = "LM Studio Custom"
base_url = "http://localhost:1234/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
```

风险：

- LM Studio 通常 OpenAI-compatible，但是否支持 `/v1/responses` 需要实测。
- 如果只支持 `/v1/chat/completions`，当前 Codex 不能直接完整兼容。

## 8. OpenRouter 配置

OpenRouter 是第一阶段重点验证对象。

### 8.1 Responses API 尝试配置

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

环境变量：

```bash
export OPENROUTER_API_KEY="..."
```

验证：

```bash
codex exec -c model_provider=\"openrouter\" -c model=\"anthropic/claude-sonnet-4.5\" "Say hello in one sentence."
```

判断：

- 如果成功：OpenRouter 可先作为主力第三方 provider。
- 如果返回 404/unsupported endpoint：说明 OpenRouter 当前路径不支持 Responses API。
- 如果 tool calling 失败：需要进一步检查 OpenRouter 对 Responses tool schema 的兼容性。

### 8.2 可能的问题

OpenRouter 常见兼容面更偏 Chat Completions。当前 Codex 没有 `wire_api = "chat"`，所以可能需要后续新增：

```rust
WireApi::ChatCompletions
```

如果 OpenRouter 不支持 Responses，但用户仍想作为主供应商，则 ChatCompletions adapter 的优先级会升高。

## 9. DeepSeek 配置

DeepSeek 也需要重点验证，因为它常见接口是 OpenAI-compatible Chat Completions。

### 9.1 Responses API 尝试配置

```toml
model_provider = "deepseek"
model = "deepseek-chat"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
env_key = "DEEPSEEK_API_KEY"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 4
stream_max_retries = 5
stream_idle_timeout_ms = 300000
```

环境变量：

```bash
export DEEPSEEK_API_KEY="..."
```

验证：

```bash
codex exec -c model_provider=\"deepseek\" -c model=\"deepseek-chat\" "Say hello in one sentence."
```

### 9.2 Reasoner 模型尝试

```toml
model_provider = "deepseek"
model = "deepseek-reasoner"
```

验证：

```bash
codex exec -c model_provider=\"deepseek\" -c model=\"deepseek-reasoner\" "Explain briefly why tests are useful."
```

风险：

- DeepSeek 可能不支持 `/v1/responses`。
- 如果报 endpoint 不存在，后续要么接代理转换层，要么新增 ChatCompletions adapter。

## 10. Anthropic-compatible 配置

Codex 当前支持 `wire_api = "anthropic_messages"`，这是接 Claude-compatible provider 的关键路径。

### 10.1 Native Anthropic Messages-compatible provider

```toml
model_provider = "anthropic-compatible"
model = "claude-sonnet-4-5"

[model_providers.anthropic-compatible]
name = "Anthropic Compatible"
base_url = "https://your-provider.example.com/v1"
env_key = "ANTHROPIC_API_KEY"
wire_api = "anthropic_messages"
auth_style = "x_api_key"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 4
stream_max_retries = 5
stream_idle_timeout_ms = 300000

[model_providers.anthropic-compatible.http_headers]
anthropic-version = "2023-06-01"
```

环境变量：

```bash
export ANTHROPIC_API_KEY="..."
```

验证：

```bash
codex exec -c model_provider=\"anthropic-compatible\" -c model=\"claude-sonnet-4-5\" "Say hello in one sentence."
```

### 10.2 Bearer-style Anthropic proxy

有些代理不是 `x-api-key`，而是 Bearer token：

```toml
model_provider = "claude-bearer-proxy"
model = "claude-sonnet-4-5"

[model_providers.claude-bearer-proxy]
name = "Claude Bearer Proxy"
base_url = "https://your-proxy.example.com/v1"
env_key = "CLAUDE_PROXY_API_KEY"
wire_api = "anthropic_messages"
auth_style = "bearer"
requires_openai_auth = false
supports_websockets = false

[model_providers.claude-bearer-proxy.http_headers]
anthropic-version = "2023-06-01"
```

验证重点：

- 是否接受 `/v1/messages`
- 是否接受 `anthropic-version` header
- 是否支持 streaming
- 是否支持 tool use/tool result 结构

## 11. 兼容性验证矩阵

| Provider | 目标 wire_api | 默认/样例模型 | 预期状态 | 重点风险 |
|---|---|---|---|---|
| OpenAI | responses | gpt-5.1-codex-max | 应可用 | auth/login/API key 配置 |
| Ollama | responses | gpt-oss:20b | 待验证 | 是否支持 `/v1/responses` |
| LM Studio | responses | local model | 待验证 | 是否支持 `/v1/responses` |
| OpenRouter | responses | anthropic/claude-sonnet-4.5 | 高风险待验证 | 可能只支持 Chat Completions |
| DeepSeek | responses | deepseek-chat | 高风险待验证 | 可能只支持 Chat Completions |
| Anthropic-compatible | anthropic_messages | claude-sonnet-4-5 | 中风险待验证 | header/auth/tool use/streaming 差异 |

## 12. 验证命令模板

### 12.1 基础连通性

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Say hello in one sentence."
```

通过标准：

- 命令退出码为 0
- 返回正常自然语言
- 没有 endpoint/auth/schema 错误

### 12.2 简单代码任务

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Write a TypeScript function that adds two numbers."
```

通过标准：

- 返回代码
- 没有 JSON schema/tool call 错误

### 12.3 工具调用任务

在一个临时测试目录内：

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Create a file hello.txt containing hello."
```

通过标准：

- 能触发文件写入相关工具
- 权限审批逻辑正常
- 文件实际创建成功

注意：这个命令会修改本地文件，应该只在临时目录测试。

### 12.4 Shell 工具任务

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Run pwd and summarize the result."
```

通过标准：

- 能触发 shell/local shell 工具
- sandbox/approval 不报异常
- 输出正确总结

### 12.5 长上下文和 streaming

```bash
codex exec -c model_provider=\"PROVIDER_ID\" -c model=\"MODEL_NAME\" "Count from 1 to 100 with commas."
```

通过标准：

- streaming 输出稳定
- 不出现 stream parse error
- 不因 idle timeout 中断

## 13. 错误分类

### 13.1 endpoint 不存在

常见表现：

```text
404 Not Found
unsupported endpoint
/v1/responses not found
```

判断：provider 不支持 Responses API。需要：

1. 换 `wire_api = "anthropic_messages"`，如果 provider 支持 Anthropic Messages；或
2. 使用代理层转换；或
3. 新增 ChatCompletions adapter。

### 13.2 鉴权错误

常见表现：

```text
401 Unauthorized
403 Forbidden
missing API key
env var not set
```

检查：

- `env_key` 是否正确

## 12. UI-first provider backend API 设计

目标：让 app-server 成为模型/provider 管理的后端，未来 Web UI / desktop settings 页只调这个后端，不直接改 TOML。

### 12.1 设计原则

1. UI-first：普通用户在界面完成 provider 配置，不接触 TOML。
2. API key 永不明文回读：后端只接收新 key，只返回是否已配置。
3. provider 配置与连接测试分离：保存配置不等于测试通过。
4. 诊断结构化：前端要区分 auth、network、endpoint、model、streaming、tool use 问题。
5. 支持内置 provider 和 custom provider，但禁止 UI 覆盖保留内置 ID。
6. 项目级覆盖与全局配置分层，后端要明确返回来源层级。

### 12.2 provider 数据模型

建议后端统一返回 ProviderConfigSummary：

```ts
interface ProviderConfigSummary {
  id: string
  displayName: string
  source: "builtin" | "custom"
  enabled: boolean
  builtinKind?: "openai" | "amazon-bedrock" | "ollama" | "lmstudio"
  baseUrl?: string
  wireApi: "responses" | "anthropic_messages"
  authStyle: "bearer" | "x_api_key"
  envKey?: string
  hasApiKey: boolean
  supportsWebsockets: boolean
  requestMaxRetries?: number
  streamMaxRetries?: number
  streamIdleTimeoutMs?: number
  websocketConnectTimeoutMs?: number
  defaultModel?: string
  models?: ProviderModelSummary[]
  httpHeaders?: ProviderHeaderSummary[]
  projectOverride: boolean
  configScope: "global" | "project"
}

interface ProviderModelSummary {
  id: string
  displayName?: string
  available: boolean
  capabilities?: {
    toolCalling?: boolean
    reasoning?: boolean
    vision?: boolean
    streaming?: boolean
  }
}

interface ProviderHeaderSummary {
  name: string
  valueSource: "literal" | "env"
  valuePreview?: string
}
```

说明：
- `hasApiKey` 只表示 key 已配置，不返回真实值。
- `valuePreview` 只允许脱敏值，例如 `sk-...abcd`，默认也可不返回。
- `configScope` 告诉前端这条配置来自全局还是项目级覆盖。

### 12.3 list/read/write/delete/test API

#### A. List providers

```http
GET /api/providers
```

返回：

```json
{
  "providers": [ProviderConfigSummary],
  "defaultProvider": "openai-custom",
  "defaultModel": "gpt-5.4"
}
```

用途：
- 设置页 provider 列表
- 默认 provider/model 下拉框
- 显示 builtin/custom、已启用状态、是否已配置 key

#### B. Read provider details

```http
GET /api/providers/:id
```

返回：

```json
{
  "provider": {
    "id": "openai-custom",
    "displayName": "OpenAI-Compatible Custom",
    "source": "custom",
    "enabled": true,
    "baseUrl": "https://provider.example.com/v1",
    "wireApi": "responses",
    "authStyle": "bearer",
    "envKey": "OPENAI_API_KEY",
    "hasApiKey": true,
    "supportsWebsockets": false,
    "requestMaxRetries": 1,
    "streamMaxRetries": 1,
    "streamIdleTimeoutMs": 30000,
    "httpHeaders": [
      { "name": "X-Title", "valueSource": "literal" }
    ]
  }
}
```

用途：
- 编辑 provider 表单初始化
- 显示高级配置和诊断信息

#### C. Create provider

```http
POST /api/providers
```

请求体：

```json
{
  "id": "openai-custom",
  "displayName": "OpenAI-Compatible Custom",
  "configScope": "global",
  "baseUrl": "https://provider.example.com/v1",
  "wireApi": "responses",
  "authStyle": "bearer",
  "apiKey": "user-input-secret",
  "supportsWebsockets": false,
  "requestMaxRetries": 1,
  "streamMaxRetries": 1,
  "streamIdleTimeoutMs": 30000,
  "defaultModel": "gpt-5.4",
  "httpHeaders": [
    { "name": "X-Title", "valueSource": "literal", "value": "Codex Local" }
  ]
}
```

返回：

```json
{
  "provider": ProviderConfigSummary,
  "warnings": []
}
```

校验：
- `id` 不能为空，且不能占用保留 builtin provider ID。
- `baseUrl` 必须是合法 URL。
- `wireApi` 只能是 `responses` 或 `anthropic_messages`。
- `apiKey` 允许为空，但为空时 `hasApiKey = false`，后续测试应返回结构化缺失错误。

#### D. Update provider

```http
PATCH /api/providers/:id
```

请求体支持部分更新：

```json
{
  "displayName": "OpenAI-Compatible Custom",
  "baseUrl": "https://provider.example.com/v1",
  "apiKey": "new-secret-or-empty-string-to-clear",
  "supportsWebsockets": false,
  "defaultModel": "gpt-5.4"
}
```

规则：
- `apiKey` 缺席 = 不改。
- `apiKey = ""` = 清除现有 key。
- 更新 builtin provider 时，只允许更新安全子集，例如默认 model、base URL 覆盖、是否启用，不允许破坏内置 ID 本身。

#### E. Delete provider

```http
DELETE /api/providers/:id
```

返回：

```json
{
  "deleted": true
}
```

规则：
- custom provider 可删除。
- builtin provider 不真删除，只允许 `enabled=false`。
- 如果被删除项是当前 default provider，返回 warning，让前端要求用户重新选择默认 provider。

#### F. Test connection

```http
POST /api/providers/:id/test
```

或在保存前支持无持久化测试：

```http
POST /api/providers/test
```

请求体：

```json
{
  "provider": {
    "id": "openai-custom",
    "baseUrl": "https://provider.example.com/v1",
    "wireApi": "responses",
    "authStyle": "bearer",
    "apiKey": "user-input-secret",
    "model": "gpt-5.4",
    "supportsWebsockets": false
  },
  "checks": {
    "basic": true,
    "streaming": true,
    "toolCalling": true
  }
}
```

返回：

```json
{
  "ok": true,
  "summary": "Basic request and streaming passed.",
  "results": [
    {
      "check": "basic",
      "ok": true
    },
    {
      "check": "streaming",
      "ok": true
    },
    {
      "check": "toolCalling",
      "ok": false,
      "errorCode": "FAIL_TOOL",
      "message": "Tool call response schema not supported by provider."
    }
  ]
}
```

### 12.4 结构化错误码

后端测试连接和保存校验建议统一错误码：

```text
MISSING_API_KEY
INVALID_BASE_URL
FAIL_AUTH
FAIL_NETWORK
FAIL_DNS
FAIL_TLS
FAIL_ENDPOINT
FAIL_MODEL
FAIL_STREAM
FAIL_SCHEMA
FAIL_TOOL
UNSUPPORTED_WIRE_API
RESERVED_PROVIDER_ID
BUILTIN_PROVIDER_IMMUTABLE
```

用户体验目标：
- 前端不显示“Unknown error”。
- 用户一眼知道是 key 错、URL 错、协议不兼容、还是模型不存在。

### 12.5 API key 存储与脱敏策略

后端不应该把 key 明文写回前端。

建议策略：

1. 写入时：
   - UI 传 `apiKey`。
   - 后端只在保存阶段消费。
2. 读取时：
   - 只返回 `hasApiKey: true/false`。
   - 可选返回 `apiKeyPreview: "sk-...abcd"`，但默认可以不返回。
3. 存储位置优先级：
   - 第一优先：系统 keyring / secure storage
   - 第二优先：环境变量引用
   - 第三优先：本地受限配置文件（仅开发模式，不推荐产品默认）
4. 文档和日志：
   - 只记录 env var 名称，不记录真实 key。
   - 测试报告只写 key 是否存在。

### 12.6 默认 provider/model 与项目级覆盖

建议再补两个接口：

```http
GET /api/provider-preferences
PATCH /api/provider-preferences
```

请求/返回示例：

```json
{
  "defaultProvider": "openai-custom",
  "defaultModel": "gpt-5.4",
  "strongModel": "gpt-5.4",
  "fastModel": "gpt-5.4-mini",
  "configScope": "project"
}
```

这样 UI 可以支持：
- 全局默认模型
- 项目级覆盖
- future strong/fast model split

### 12.7 后端实现建议

推荐先做 app-server protocol + request processor：

```text
codex-rs/app-server-protocol
codex-rs/app-server/src/request_processors
```

第一批最小实现顺序：
1. `GET /api/providers`
2. `GET /api/providers/:id`
3. `POST /api/providers/test`（无持久化）
4. `POST /api/providers`
5. `PATCH /api/providers/:id`
6. `DELETE /api/providers/:id`
7. `GET/PATCH /api/provider-preferences`

### 12.8 与当前实测结果的关系

结合 A003 当前结果，默认建议：
- 对 custom OpenAI-compatible provider，创建向导默认 `supportsWebsockets = false`。
- 先主打 `responses` + HTTPS/SSE 路径。
- 只有用户显式测试通过后，才开放 WebSocket 开关。
- ChatCompletions adapter 暂不进入这批后端 API 最小范围。

这保证 UI-first provider management 可以先落地，不被更大 adapter 改造阻塞。'}]} to=functions.Edit  北京赛车开奖իչ to=functions.Edit  大发彩票官网 to=functions.Edit 񟡋json શું ആണ് this? JSONArray? Let's provide proper JSON. 夜夜啪 to=functions.Edit  天天中彩票怎么json  时时彩后assistant to=functions.Edit commentary  天天中彩票有人  天天中彩票提现 to=functions.Edit  大发快三是国家արգիչ to=functions.Edit 早点加盟 to=functions.Edit  荣富resco  大发快三彩票 to=functions.Edit  天天中彩票无法 to=functions.Edit ￣奇米影视json 无码不卡高清免费 to=functions.Edit _一本道 to=functions.Edit เงินไทยฟรี to=functions.Edit  大发快三的 to=functions.Edit  大发极速 to=functions.Edit 不中反 to=functions.Edit  天天中彩票 to=functions.Edit  东臣 to=functions.Edit  北京赛车的 to=functions.Edit ақәеи to=functions.Edit েজোন code  菲律宾申博json  天天中彩票网站 to=functions.Edit  天天中彩票未commentary to=functions.Edit  心博json 期六合 to=functions.Edit  大发快三彩票 to=functions.Edit әдоуjson  天天中奖彩票 to=functions.Edit  彩神争霸提现json Japgollyassistant to=functions.Edit  天天中彩票大神推荐 to=functions.Edit  久赢json  仲博 to=functions.Edit 凤凰大参考 to=functions.Edit িন্নასუხ to=functions.Edit  天天中彩票网站 to=functions.Edit  彩神争霸有 to=functions.Edit ყვეტ to=functions.Edit 】【：】【“】【file_path_please_fix? Let's recover. The previous tool call is malformed. We need send a clean Edit call. We'll use exact old string from end of file maybe last line. Need read tail context. We have enough maybe last line is `-  env_key 是否正确` near offset 580. Let's read tail lines around end.րցanalysis to=functions.Read  天天中彩票腾讯 մեկն  海南天天中彩票json  彩神争霸高 to=functions.Read 公众号天天中彩票ം 彩票开号 to=functions.Read  大发彩票官网 to=functions.Read 公众号天天中彩票 to=functions.Read  大发快三开奖 to=functions.Read  天天中彩票中 to=functions.Read  һөкүмитиниң to=functions.Read ugburuassistant to=functions.Read 夫妻性生活影片 to=functions.Read  天天中彩票不中返json  重庆时时彩杀 to=functions.Read  亚历山大发 to=functions.Read  天天购彩票 to=functions.Read цҳауеит to=functions.Read RGCTX ＿久久essage  彩神争霸提现json  北京赛车计划 to=functions.Read 930? We'll do.ოქმედary to=functions.Read  天天乐彩票 ﻿출장안마json 955? {
- 环境变量是否已 export
- `auth_style` 是 bearer 还是 x_api_key
- 是否错误设置了 `requires_openai_auth = true`

### 13.3 streaming 解析错误

常见表现：

```text
failed to parse stream event
unexpected event type
connection closed
```

判断：provider 的 SSE 格式与 Codex 预期不一致。需要：

- 调整 wire adapter；或
- 关闭/调整 streaming，如果支持；或
- 新增 provider-specific 兼容层。

### 13.4 tool call schema 错误

常见表现：

```text
failed to parse function arguments
unsupported tool call
invalid tool schema
```

判断：模型或 provider 对 tool calling 格式不完全兼容。

处理方向：

- 降低工具复杂度
- 修改 tool spec 映射
- 增加 provider capability 判断
- 对不支持工具的模型禁用工具调用

## 14. 建议的实测顺序

第一轮只做“最小连通性”：

1. OpenAI 默认 provider
2. Ollama 内置 provider
3. LM Studio 内置 provider
4. Anthropic-compatible with `anthropic_messages`
5. OpenRouter with `responses`
6. DeepSeek with `responses`

第二轮做“工具能力”：

1. read/search task
2. write file task
3. shell task
4. apply patch task
5. multi-turn task

第三轮做“稳定性”：

1. streaming 长输出
2. 大上下文
3. tool call 多轮
4. 错误恢复
5. rate limit / retry

## 15. 后续源码改造建议

### 15.1 低成本改造

优先做：

1. 增加 provider 配置模板文档
2. 增加 `codex provider list`
3. 增加 `codex provider test`
4. 增加更友好的错误提示
5. 增加静态 model catalog 配置
6. TUI 显示当前 provider + model

### 15.2 中成本改造

如果 Anthropic-compatible 路线可行：

1. 强化 `anthropic_messages` adapter
2. 增加 Claude tool use 映射测试
3. 增加 Anthropic-style streaming 兼容测试
4. 增加 provider capabilities：tool calling、vision、reasoning、streaming

### 15.3 高成本改造

如果 OpenRouter/DeepSeek 只支持 Chat Completions，则需要：

```rust
WireApi::ChatCompletions
```

涉及：

```text
codex-rs/model-provider-info/src/lib.rs
codex-rs/core/src/client.rs
codex-rs/codex-api
codex-rs/protocol / response event mapping
models-manager / model catalog capability
工具调用格式转换
streaming delta 转换
```

这应作为单独里程碑，不要和 provider 文档/配置管理混在一起。

## 16. 建议的近期落地计划

### Milestone A：配置验证

产出：

```text
docs/yuqei-provider-compatibility.md
```

完成标准：

- 每个目标 provider 有配置样例
- 每个目标 provider 有验证命令
- 明确风险和错误分类

### Milestone B：本机实测记录

产出：

```text
docs/yuqei-provider-test-results.md
```

记录格式：

```markdown
## OpenRouter

- Date:
- Model:
- Config:
- Basic connectivity: pass/fail
- Tool call: pass/fail
- Shell call: pass/fail
- Error:
- Conclusion:
```

### Milestone C：provider CLI 设计

产出：

```text
docs/yuqei-provider-cli-design.md
```

设计命令：

```text
codex provider list
codex provider show <id>
codex provider add <template>
codex provider test <id>
codex provider select <id>
codex model list
codex model select <model>
```

### Milestone D：是否需要 ChatCompletions adapter 的决策

根据实测结果决定。

如果 OpenRouter 和 DeepSeek 都无法通过 `responses` 跑通，则新增 ChatCompletions adapter 变成高优先级。

## 17. 当前推荐策略

短期：

1. 先使用 OpenAI 默认 provider 保证主线可用。
2. 同时验证 Anthropic-compatible 的 `anthropic_messages` 路径。
3. 验证 Ollama/LM Studio 作为本地模型路径。
4. OpenRouter/DeepSeek 先按 `responses` 尝试，不成功再规划 ChatCompletions adapter。

长期：

1. provider 不应该只是配置表，而应有 capability model。
2. model 不应该只是字符串，而应有能力声明。
3. agent 应根据 provider/model 能力决定是否启用工具、vision、reasoning、web search、parallel tool calls。
4. 第三方模型支持应通过 adapter 层解决，不要污染 core agent loop。
