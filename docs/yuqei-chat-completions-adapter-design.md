# A011 Chat Completions Adapter 设计

> 目标：让 Codex 能直接接入只支持 OpenAI Chat Completions 的第三方模型服务。第一目标 provider 是 DeepSeek。

## 1. 背景

当前 Codex provider 只支持两种 `wire_api`：

- `responses`：OpenAI Responses API，路径通常是 `/v1/responses`
- `anthropic_messages`：Anthropic Messages API，路径通常是 `/v1/messages`

DeepSeek 实测结果：

| Endpoint | 结果 |
|---|---|
| `POST https://api.deepseek.com/v1/responses` | HTTP 404 |
| `POST https://api.deepseek.com/v1/chat/completions` | HTTP 200 |

结论：DeepSeek key 和模型可用，但协议是 Chat Completions。Codex 要直接支持 DeepSeek，需要新增 `WireApi::ChatCompletions`，或要求用户使用转换代理。长期产品目标是多 provider 原生支持，所以应设计 adapter。

## 2. 范围

### 2.1 MVP 范围

第一版只做 DeepSeek 可用所需的最小闭环：

1. 新增 provider 配置值：
   ```toml
   wire_api = "chat_completions"
   ```
2. HTTP streaming 请求到：
   ```text
   {base_url}/chat/completions
   ```
3. 支持基础文本输出。
4. 支持普通 shell / 文件工具调用需要的 function calling 映射。
5. 支持 tool result 回灌到后续 Chat Completions messages。
6. 支持错误分类：auth、endpoint、model、schema、stream。
7. 支持 DeepSeek `deepseek-chat`。

### 2.2 暂不做

第一版不做这些，避免 adapter 变成大泥球：

- Vision / image input
- OpenAI Responses 专属 reasoning summary 等价映射
- Parallel tool calls 完整语义保证
- WebSocket
- Built-in web search / computer-use 特殊 hosted tools
- 多模态 file input
- DeepSeek reasoner 的完整推理内容结构化展示

这些可以在 Chat Completions 基础闭环通过后按能力模型补。

## 3. 需要改的主要位置

### 3.1 provider schema

文件：

```text
codex-rs/model-provider-info/src/lib.rs
codex-rs/core/config.schema.json
```

改动：

- `WireApi` 新增：
  ```rust
  ChatCompletions
  ```
- serde 名称：
  ```text
  chat_completions
  ```
- `Display` 输出 `chat_completions`
- `Deserialize` 接受 `chat_completions`
- 是否继续拒绝旧 `chat`：保留拒绝，提示用户使用 `chat_completions`

建议错误文案：

```text
`wire_api = "chat"` is no longer supported. Use `wire_api = "chat_completions"` for OpenAI Chat Completions-compatible providers.
```

### 3.2 codex-api endpoint

新增文件：

```text
codex-rs/codex-api/src/endpoint/chat_completions.rs
codex-rs/codex-api/src/sse/chat_completions.rs
```

导出位置：

```text
codex-rs/codex-api/src/endpoint/mod.rs
codex-rs/codex-api/src/sse/mod.rs
codex-rs/codex-api/src/lib.rs
```

新增 client：

```rust
ChatCompletionsClient
ChatCompletionsOptions
```

endpoint：

```text
/chat/completions
```

### 3.3 common request/response types

文件：

```text
codex-rs/codex-api/src/common.rs
```

新增最小类型：

```rust
ChatCompletionsRequest {
  model: String,
  messages: Vec<ChatMessage>,
  tools: Option<Vec<ChatTool>>,
  tool_choice: Option<String>,
  stream: bool,
  temperature: Option<f64>,
  max_tokens: Option<u32>,
}

ChatMessage {
  role: String, // system | user | assistant | tool
  content: Option<String>,
  tool_calls: Option<Vec<ChatToolCall>>,
  tool_call_id: Option<String>,
}

ChatTool {
  type: String, // "function"
  function: ChatFunctionTool,
}
```

第一版可以只实现 Codex 需要的字段，不追求完整 OpenAI schema。

### 3.4 core client 分发

文件：

```text
codex-rs/core/src/client.rs
```

新增常量：

```rust
const CHAT_COMPLETIONS_ENDPOINT: &str = "/chat/completions";
```

新增构建函数：

```rust
fn build_chat_completions_request(
  &self,
  prompt: &Prompt,
  model_info: &ModelInfo,
) -> Result<ChatCompletionsRequest>
```

新增 streaming 分支：

```rust
WireApi::ChatCompletions => {
  self.stream_chat_completions_api(...).await
}
```

WebSocket 不参与 Chat Completions。

## 4. Prompt 到 Chat messages 的映射

### 4.1 system

`prompt.base_instructions.text` 映射为第一条 system message：

```json
{ "role": "system", "content": "..." }
```

### 4.2 user / assistant 历史

Codex 现在内部走 Responses-style `ResponseItem` / `ContentItem`。Chat adapter 需要把 `prompt.get_formatted_input()` 转成 Chat messages。

建议新建转换函数：

```rust
chat_messages_from_input(input: Vec<ResponseItem>, system: String) -> Result<Vec<ChatMessage>>
```

基本映射：

| Codex / Responses item | Chat Completions message |
|---|---|
| user text | `{ role: "user", content }` |
| assistant text | `{ role: "assistant", content }` |
| function call | assistant message with `tool_calls` |
| function call output | `{ role: "tool", tool_call_id, content }` |

### 4.3 多段 content

MVP 只支持 text content。遇到图片、文件、多模态：

- 返回 `FAIL_SCHEMA` 风格错误
- 错误文案说明 provider 不支持当前输入类型

## 5. Tools 映射

Codex 已有：

```rust
create_tools_json_for_responses_api(&prompt.tools)
```

Chat Completions 需要：

```json
{
  "type": "function",
  "function": {
    "name": "...",
    "description": "...",
    "parameters": { ... }
  }
}
```

建议新增：

```rust
create_tools_json_for_chat_completions(&prompt.tools)
```

不要复用 Responses 的工具 JSON 后临时 patch 字段。直接生成 Chat schema 更清晰，也方便未来按 provider capability 降级。

## 6. Streaming 事件映射

Chat Completions streaming 是 SSE，常见事件：

```text
data: {"choices":[{"delta":{"content":"..."}}]}
data: {"choices":[{"delta":{"tool_calls":[...]}}]}
data: [DONE]
```

Codex core 需要输出统一的 `ResponseEvent`。

### 6.1 文本增量

Chat delta：

```json
{"choices":[{"delta":{"content":"hello"}}]}
```

映射到 Responses-style：

- `response.output_text.delta`

### 6.2 assistant 完成

`finish_reason = "stop"` 或 `[DONE]` 后：

- 发出 `response.completed`

### 6.3 tool call 增量

Chat delta 里的 tool call 通常分片返回：

```json
{
  "choices": [{
    "delta": {
      "tool_calls": [{
        "index": 0,
        "id": "call_x",
        "type": "function",
        "function": {
          "name": "shell",
          "arguments": "{...partial..."
        }
      }]
    }
  }]
}
```

adapter 要聚合：

- 按 `index` 聚合 id/name/arguments
- arguments 字符串拼接到完整 JSON
- `finish_reason = "tool_calls"` 时发出完整 function call item

第一版不要在 arguments 未完整时发工具调用。

### 6.4 错误处理

- SSE 非 JSON：stream parse error
- HTTP 401/403：auth error
- HTTP 404：endpoint error
- provider 返回 error object：保留 message，但不能泄露 key
- `[DONE]` 前没有 terminal event：映射为 stream dropped

## 7. DeepSeek provider 配置样例

```toml
model_provider = "deepseek"
model = "deepseek-chat"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
env_key = "DEEPSEEK_API_KEY"
wire_api = "chat_completions"
auth_style = "bearer"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 4
stream_max_retries = 5
stream_idle_timeout_ms = 300000
```

可选：

```toml
model = "deepseek-reasoner"
```

但 reasoner 的推理内容先不承诺完整结构化。

## 8. 测试计划

### 8.1 单元测试

新增：

```text
codex-rs/model-provider-info/src/model_provider_info_tests.rs
```

覆盖：

- `wire_api = "chat_completions"` 可反序列化
- `WireApi::ChatCompletions.to_string()` 正确
- 旧 `wire_api = "chat"` 仍给出迁移提示

新增：

```text
codex-rs/codex-api/src/sse/chat_completions.rs
```

覆盖 fixture：

- 文本 delta → ResponseEvent text delta
- `[DONE]` → completed
- tool_calls 分片 → 完整 function call
- error object → ApiError

新增：

```text
codex-rs/core/tests/suite/client_chat_completions.rs
```

覆盖：

- 请求路径是 `/chat/completions`
- body 包含 `messages`
- tools 是 Chat Completions function schema
- tool result 能回灌为 `role = "tool"`

### 8.2 真实 provider 测试

第一批只测 DeepSeek：

1. raw HTTP `/chat/completions` 非 streaming
2. raw HTTP streaming
3. Codex smoke：一句话 hello
4. Codex codegen：只返回 TS add 函数
5. Codex shell：运行 `pwd` 并总结
6. Codex file write：临时目录写 `hello.txt`
7. Codex tool compatibility：列目录识别 README

测试报告写入：

```text
docs/yuqei-provider-test-results.md
automation/reports/test-runs/
```

不写入 API key。

## 9. 实现顺序

推荐小步提交：

1. `WireApi::ChatCompletions` schema + config tests
2. Chat request/response structs
3. Chat SSE parser，先只支持 text
4. `ModelClientSession::stream` 分支接入 text-only
5. DeepSeek smoke test
6. 加 tool schema 映射
7. 加 tool call streaming 聚合
8. 工具调用真实测试
9. 更新 docs/config.md 和 provider docs

## 10. 风险

### 10.1 Responses 语义不等价

Responses API 的 item/event 模型比 Chat Completions 更丰富。adapter 必须明确降级范围，不要假装完全等价。

### 10.2 Tool call streaming 最容易出错

DeepSeek / OpenAI-compatible provider 可能分片方式不同。必须用 fixture 和真实 provider 双测。

### 10.3 多轮 tool result 顺序

Chat Completions 要求 tool result 紧跟对应 assistant tool call，且 `tool_call_id` 匹配。Codex 内部历史转换时必须保持顺序。

### 10.4 推理字段兼容

DeepSeek reasoner 可能返回推理相关字段。MVP 先忽略或作为 provider-specific 后续能力，不阻塞 `deepseek-chat`。

## 11. 完成标准

A011 设计完成后，进入实现前应满足：

- DeepSeek 不支持 `/responses`、支持 `/chat/completions` 的证据已记录。
- Chat adapter MVP 范围明确。
- 代码改动点明确。
- Streaming 和 tool call 映射明确。
- 回归测试文件和真实 provider 验证顺序明确。

实现完成标准：

- DeepSeek `deepseek-chat` 在 Codex 中至少通过：基础、代码生成、streaming。
- 工具调用如无法第一版全过，必须清楚标记 `PARTIAL`，并说明缺口。
- 不影响现有 `responses` 和 `anthropic_messages` provider 测试。
