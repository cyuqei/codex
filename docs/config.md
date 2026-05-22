# Configuration

For basic configuration instructions, see [this documentation](https://developers.openai.com/codex/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/codex/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/codex/config-reference).

## Third-party Claude-compatible providers

Codex can use a third-party Claude provider through `model_providers`.

For an OpenAI-compatible Claude proxy that supports the Responses API:

```toml
model_provider = "claude_proxy"
model = "claude-sonnet-4-5"

[model_providers.claude_proxy]
name = "Claude Proxy"
base_url = "https://your-proxy.example.com/v1"
env_key = "CLAUDE_PROXY_API_KEY"
wire_api = "responses"
requires_openai_auth = false
```

For a native Anthropic Messages-compatible proxy:

```toml
model_provider = "claude_third_party"
model = "claude-sonnet-4-5"

[model_providers.claude_third_party]
name = "Claude Third Party"
base_url = "https://your-provider.example.com/v1"
env_key = "CLAUDE_THIRD_PARTY_API_KEY"
wire_api = "anthropic_messages"
auth_style = "x_api_key"
requires_openai_auth = false

[model_providers.claude_third_party.http_headers]
anthropic-version = "2023-06-01"
```

Set the matching environment variable before starting Codex, for example `CLAUDE_THIRD_PARTY_API_KEY`.

For an OpenAI Chat Completions-compatible provider such as DeepSeek:

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
```

`chat_completions` config support is the provider selection knob for services that expose `/v1/chat/completions`. The streaming adapter must also be available in the runtime before these providers can complete turns.
