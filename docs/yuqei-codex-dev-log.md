# Codex 自动化开发日志

> 记录 `/Users/yuqei/codex` 长期自动化开发过程中的任务、改动、验证、失败和下一步。

## 2026-05-07 A001/A002

- Task: 建立自动化控制面基础，并填写 provider 实测环境信息。
- Files changed:
  - `docs/yuqei-codex-dev-log.md`
  - `docs/yuqei-provider-test-results.md`
  - `automation/queue/000-provider-phase.yaml`
  - `automation/scripts/run-next-task.sh`
  - `automation/scripts/verify-task.sh`
  - `automation/scripts/update-dev-log.sh`
- Tests run:
  - `cargo check -p codex-core` → PASS, report `automation/reports/test-runs/20260507-101757-A002.log`.
  - `cargo test -p codex-core config` → FAIL after lib config tests passed; integration tests matching `config` require missing `target/debug/test_stdio_server`, report `automation/reports/test-runs/20260507-102117-A002.log`.
- Result: 自动化目录和首批 provider 任务队列已建立；当前 Codex core 可编译。
- Decisions:
  - 先用 YAML 任务队列做本地控制面。
  - 不自动执行需要真实 API key 或本地 provider 服务的外部测试。
  - A002 记录为 done，因为失败原因是测试命令粒度/fixture 二进制问题，已保存证据，不阻塞 provider 实测文档推进。
- Blockers: provider 实测需要用户本机 key 或本地服务状态。
- Next: 检查 provider 前置条件，缺少 key 或本地服务时标记 BLOCKED。

## 2026-05-07 A003-A008

- Task: 执行第一批 provider 兼容性前置检查和 OpenAI smoke test。
- Files changed:
  - `docs/yuqei-provider-test-results.md`
  - `automation/queue/000-provider-phase.yaml`
  - `automation/reports/test-runs/20260507-102309-A003.log`
  - `automation/reports/test-runs/20260507-102343-A003.log`
  - `automation/reports/test-runs/20260507-102416-A003.log`
- Tests run:
  - OpenAI: local binary smoke test with `gpt-5.1` → PARTIAL/blocked by streaming reconnect loop.
  - Ollama: `curl --max-time 2 http://localhost:11434/api/tags` → local service unavailable.
  - LM Studio: `curl --max-time 2 http://localhost:1234/v1/models` → local service unavailable.
  - OpenRouter: env check → `OPENROUTER_API_KEY` missing.
  - DeepSeek: env check → `DEEPSEEK_API_KEY` missing.
- Result: Provider phase is blocked on external prerequisites or streaming/MCP transport investigation.
- Decisions: Do not start local services, guess API keys, or keep a stuck streaming request running indefinitely.
- Blockers:
  - A003 OpenAI: `rmcp::transport::worker` JSON-RPC parse error followed by repeated reconnects.
  - A004 Ollama: service unavailable on localhost:11434.
  - A005 LM Studio: service unavailable on localhost:1234.
  - A006 Anthropic-compatible: no concrete base_url/key configured.
  - A007 OpenRouter: `OPENROUTER_API_KEY` missing.
  - A008 DeepSeek: `DEEPSEEK_API_KEY` missing.
- Next: Investigate OpenAI streaming/MCP transport loop, or provide/start external provider prerequisites and rerun the blocked tasks.

## 2026-05-07 A009

- Task: Summarize ChatCompletions adapter decision.
- Files changed:
  - `docs/yuqei-provider-test-results.md`
  - `automation/queue/000-provider-phase.yaml`
- Tests run: No code validation required for decision record.
- Result: Decision recorded as B, needed but not urgent.
- Decisions: Do not implement ChatCompletions adapter yet. Current evidence points first to OpenAI streaming/MCP transport investigation and provider prerequisite gaps, not confirmed `/v1/responses` incompatibility.
- Blockers: None for A009 after auto-decision in full automation mode.
- Next: Continue to A010 provider UI-first backend API design, while separately investigating the OpenAI streaming loop.

## 2026-05-07 A003 blocker investigation

- Task: Investigate OpenAI provider smoke test reconnect loop instead of skipping it.
- Files changed:
  - `docs/yuqei-provider-test-results.md`
  - `automation/queue/000-provider-phase.yaml`
- Tests run:
  - Built-in `openai` provider via WebSocket → `No route to host (os error 65)` for `wss://api.openai.com/v1/responses`, report `automation/reports/test-runs/20260507-111643-A003.log`.
  - Custom `openai-custom` provider with `supports_websockets=false` → HTTPS/SSE path still failed, report `automation/reports/test-runs/20260507-115314-A003.log`.
  - `curl https://api.openai.com/v1/models` with `OPENAI_API_KEY` → 10s timeout, HTTP 000.
  - `curl https://api.openai.com/v1/responses` with `OPENAI_API_KEY` → 10s timeout, HTTP 000.
- Result: Root blocker is local network reachability to `api.openai.com`, not confirmed Codex adapter incompatibility.
- Decisions: Do not skip A003 or use A003 as evidence for ChatCompletions. Stop and ask user to fix/provide network path before continuing OpenAI provider validation.
- Blockers: Local machine/session cannot reach `api.openai.com` over HTTPS or WebSocket.
- Next: User should enable working network/proxy/VPN for `api.openai.com`, then rerun A003.

## 2026-05-07 A003 resolved

- Task: Rerun OpenAI-compatible custom provider after network/proxy was fixed.
- Files changed:
  - `docs/yuqei-provider-test-results.md`
  - `automation/queue/000-provider-phase.yaml`
- Tests run:
  - `curl $OPENAI_BASE_URL/models` with `OPENAI_API_KEY` → HTTP 200.
  - `curl $OPENAI_BASE_URL/responses` with `model=gpt-5.4` → HTTP 200.
  - Codex smoke test with `openai-custom`, `gpt-5.4`, `wire_api=responses`, `supports_websockets=false` → PASS, returned `Hello.`.
- Result: A003 is done for the actual OpenAI-compatible endpoint in this environment.
- Decisions: Treat the user's actual model endpoint as custom OpenAI-compatible, not official OpenAI. Keep WebSocket disabled for this provider until explicitly tested.
- Blockers: None for basic connectivity.
- Next: Continue provider compatibility tests for code generation, file writing, shell, streaming stability, and tool compatibility.

## 2026-05-07 A003 full verification

- Task: Finish the remaining OpenAI-compatible custom provider verification matrix.
- Files changed:
  - `docs/yuqei-provider-test-results.md`
- Tests run:
  - Direct codegen prompt → PASS, report `automation/reports/test-runs/20260507-180224-A003-codegen-direct.log`.
  - Shell `pwd` prompt → PASS, report `automation/reports/test-runs/20260507-180255-A003-shell.log`.
  - Streaming count-to-100 prompt → PASS, report `automation/reports/test-runs/20260507-180354-A003-stream.log`.
  - File write in writable temp repo → PASS, report `automation/reports/test-runs/20260507-180434-A003-filewrite.log`.
  - Tool compatibility directory listing/README detection → PASS, report `automation/reports/test-runs/20260507-180513-A003-tool.log`.
- Result: A003 full matrix passed for the actual GPT-5.4 OpenAI-compatible endpoint.
- Decisions: For this environment, provider verification should target the custom OpenAI-compatible endpoint, not official OpenAI. Keep `supports_websockets=false` in current test config because Responses/SSE is already sufficient and passing.
- Blockers: None for A003.
- Next: Continue to the next unblocked provider task in the queue.

## 2026-05-07 A009 resolved

- Task: Reassess ChatCompletions adapter decision after A003 full verification.
- Files changed:
  - `docs/yuqei-provider-test-results.md`
  - `automation/queue/000-provider-phase.yaml`
- Tests run: No new code execution required; based on completed provider evidence.
- Result: A009 is now done with decision B.
- Decisions: Keep ChatCompletions adapter at B (needed but not urgent). The actual GPT-5.4 OpenAI-compatible endpoint fully passes `responses`, so current pressure is not from OpenAI-compatible support.
- Blockers: Still need more evidence from OpenRouter/DeepSeek/Ollama/LM Studio before promoting adapter work.
- Next: Finish A010 and continue provider checks that have become actionable.

## 2026-05-07 A010

- Task: Design UI-first provider backend API.
- Files changed:
  - `docs/yuqei-provider-compatibility.md`
- Tests run: Documentation/design task only.
- Result: Added backend API design covering provider list/read/write/delete/test endpoints, preferences endpoints, provider summary model, error codes, API key redaction, storage policy, and implementation order.
- Decisions: Future UI should talk only to app-server APIs for provider management, not write TOML directly. Default custom OpenAI-compatible providers should start with `supportsWebsockets=false` until explicitly tested.
- Blockers: None for A010 itself.
- Next: Continue blocked provider tasks if prerequisites are now present.

## 2026-05-07 A006 refined blocker

- Task: Recheck whether Anthropic-compatible provider is runnable now that env vars exist.
- Files changed:
  - `docs/yuqei-provider-test-results.md`
  - `automation/queue/000-provider-phase.yaml`
- Tests run:
  - `curl $ANTHROPIC_BASE_URL/messages` with `x-api-key` and `anthropic-version` → HTTP 404, `REMOTE_IP=127.0.0.1`.
- Result: A006 blocker changed from missing config to `FAIL_ENDPOINT`.
- Decisions: Do not treat A006 as a missing-env blocker anymore. It now needs the correct Anthropic-compatible base URL/path.
- Blockers: Current `ANTHROPIC_BASE_URL` does not expose a working `/messages` endpoint for Codex `anthropic_messages`.
- Next: Ask user for the correct Anthropic-compatible endpoint only when we choose to unblock A006.

## 2026-05-07 run-until-blocked status

- Task: Continue autonomous queue execution after A003.
- Files changed:
  - `docs/yuqei-codex-dev-log.md`
- Tests run: Rechecked Ollama, LM Studio, OpenRouter, DeepSeek, Anthropic-compatible prerequisites.
- Result: A009 done, A010 done. A004/A005/A007/A008 remain blocked by missing local services or keys. A006 is blocked by a concrete endpoint mismatch.
- Decisions: We should stop here and ask only for the next unblock input, not skip to unrelated work.
- Blockers:
  - A004 Ollama unavailable on localhost:11434.
  - A005 LM Studio unavailable on localhost:1234.
  - A006 current `ANTHROPIC_BASE_URL/messages` returns 404.
  - A007 `OPENROUTER_API_KEY` missing.
  - A008 `DEEPSEEK_API_KEY` missing.
- Next: Wait for the user to choose which provider blocker to unblock next.

## 2026-05-08 A008 resolved

- Task: Test DeepSeek provider compatibility with user-provided key.
- Files changed:
  - `docs/yuqei-provider-test-results.md`
  - `automation/queue/000-provider-phase.yaml`
- Tests run:
  - Raw HTTP `POST https://api.deepseek.com/v1/responses` with `deepseek-chat` → HTTP 404.
  - Raw HTTP `POST https://api.deepseek.com/v1/chat/completions` with `deepseek-chat` → HTTP 200, returned Chat Completions-shaped JSON with `choices`.
- Result: A008 is done as `NEEDS_ADAPTER` / `FAIL_ENDPOINT` for current Codex.
- Decisions: DeepSeek should not be tested further through current Codex `wire_api = "responses"`; it needs `WireApi::ChatCompletions` or a conversion proxy before codegen/shell/streaming/tool matrix can run.
- Blockers: Codex currently has no Chat Completions wire adapter.
- Next: Revisit A009. DeepSeek is now concrete evidence for promoting ChatCompletions adapter priority if OpenRouter or local providers show the same pattern.

## 2026-05-08 A011

- Task: Design Chat Completions adapter after DeepSeek compatibility result.
- Files changed:
  - `docs/yuqei-chat-completions-adapter-design.md`
  - `automation/queue/000-provider-phase.yaml`
  - `docs/yuqei-codex-dev-log.md`
- Tests run: Documentation/design task only. Read current provider/client code paths to ground the design.
- Result: Added adapter design covering `WireApi::ChatCompletions`, `/chat/completions` endpoint client, request/response structs, prompt-to-chat-message conversion, streaming delta mapping, tool call aggregation, DeepSeek provider config, implementation order, and regression/real-provider test plan.
- Decisions: First implementation target should be DeepSeek `deepseek-chat`; MVP should support text streaming and then tool calls, while deferring vision, WebSocket, full reasoning metadata, and complete parallel tool call semantics.
- Blockers: None for design. Implementation still requires code changes across model-provider-info, codex-api, core client, tool schema mapping, and tests.
- Next: If user approves implementation, start with schema/config support for `wire_api = "chat_completions"`, then text-only streaming before tool-call support.

## 2026-05-08 A011 implementation step 1

- Task: Add schema/config support for Chat Completions provider selection.
- Files changed:
  - `codex-rs/model-provider-info/src/lib.rs`
  - `codex-rs/model-provider-info/src/model_provider_info_tests.rs`
  - `codex-rs/core/config.schema.json`
  - `codex-rs/core/src/client.rs`
  - `docs/config.md`
- Tests run:
  - `cargo test --manifest-path /Users/yuqei/codex/codex-rs/Cargo.toml -p codex-model-provider-info chat_completions` → PASS.
  - `cargo check --manifest-path /Users/yuqei/codex/codex-rs/Cargo.toml -p codex-core` → PASS.
- Result: `wire_api = "chat_completions"` now deserializes, displays as `chat_completions`, appears in JSON schema/docs, and fails at runtime with a clear unsupported-operation message until the streaming adapter lands.
- Decisions: Keep this as a safe config/schema step only; do not pretend DeepSeek is runnable until `/chat/completions` streaming support is implemented.
- Blockers: Chat Completions request/response structs and SSE adapter still need implementation.
- Next: Add minimal Chat Completions request types and text-only SSE mapping, then wire `ModelClientSession::stream` to `/chat/completions`.

## 2026-05-08 A011 implementation step 2

- Task: Add text-only Chat Completions streaming adapter.
- Files changed:
  - `codex-rs/codex-api/src/common.rs`
  - `codex-rs/codex-api/src/endpoint/chat_completions.rs`
  - `codex-rs/codex-api/src/endpoint/mod.rs`
  - `codex-rs/codex-api/src/lib.rs`
  - `codex-rs/codex-api/src/sse/chat_completions.rs`
  - `codex-rs/codex-api/src/sse/mod.rs`
  - `codex-rs/core/src/client.rs`
- Tests run:
  - `cargo test --manifest-path /Users/yuqei/codex/codex-rs/Cargo.toml -p codex-api chat_completions` → PASS.
  - `cargo check --manifest-path /Users/yuqei/codex/codex-rs/Cargo.toml -p codex-core` → PASS.
- Result: `WireApi::ChatCompletions` now routes through `/chat/completions` with minimal request structs, prompt-to-chat message conversion, text delta SSE parsing, provider error handling, and completion mapping.
- Decisions: This step is text-only for streaming output. Tool call deltas are represented in request/history/tool schema but streaming tool-call aggregation is still deferred.
- Blockers: Need live DeepSeek smoke test and then tool-call streaming aggregation before marking DeepSeek fully usable.
- Next: Run DeepSeek text-only smoke if `DEEPSEEK_API_KEY` is available; otherwise stop at external-prerequisite blocker.

## 2026-05-08 run-until-blocked status after A011 validation

- Task: Continue autonomous execution after the Chat Completions adapter work and verify the current local implementation before attempting live provider smoke tests.
- Files changed:
  - `automation/queue/000-provider-phase.yaml`
  - `docs/yuqei-codex-dev-log.md`
- Tests run:
  - `cargo test -p codex-api chat_completions` → PASS.
  - `cargo test -p codex-model-provider-info chat_completions` → PASS.
  - `cargo check -p codex-core` → PASS.
- Result: The current working tree already contains Chat Completions tool-call SSE aggregation and related request/history mapping. Local validation passed for the parser, provider schema, and `codex-core` compile chain.
- Decisions:
  - Do not add more local adapter code in this turn because the next critical-path proof is live provider execution, not speculative refactoring.
  - Correct the stale queue state: A010 is done, and the new next task is a live DeepSeek smoke test through the `chat_completions` wire path.
- Blockers:
  - `DEEPSEEK_API_KEY` is missing in the current shell, so the live DeepSeek smoke test cannot run.
  - `OPENROUTER_API_KEY` is missing.
  - Ollama is still unavailable on `localhost:11434`.
  - LM Studio is still unavailable on `localhost:1234`.
  - Anthropic-compatible env vars are absent in the current shell.
- Next: Resume from the new blocked task A012 once `DEEPSEEK_API_KEY` is available, then run the live `codex exec` smoke test before any further Chat Completions adapter expansion.

## 2026-05-08 A012 local regression coverage

- Task: Continue autonomous execution without live provider credentials by tightening local `codex-core` regression coverage for the Chat Completions adapter.
- Files changed:
  - `codex-rs/core/tests/suite/client.rs`
  - `automation/queue/000-provider-phase.yaml`
  - `docs/yuqei-codex-dev-log.md`
- Tests run:
  - `just fmt` → PASS.
  - `cargo test -p codex-core chat_completions` → PASS.
- Result: Added `codex-core` regression tests that verify `/v1/chat/completions` is used, assistant tool-call history is serialized into Chat Completions `messages`, tool results are replayed as `role = "tool"` with `tool_call_id`, and image input is rejected locally before any HTTP request.
- Decisions:
  - Keep A012 blocked rather than inventing more adapter work; the next highest-value proof is still a live DeepSeek smoke test.
  - Use the new `codex-core` tests as the local guardrail for future `chat_completions` changes.
- Blockers:
  - `DEEPSEEK_API_KEY` is still missing in the current shell, so the live DeepSeek smoke test remains blocked.
- Next: When `DEEPSEEK_API_KEY` becomes available, rerun A012 from the live `codex exec -c model_provider="deepseek" -c model="deepseek-chat"` smoke command.

## 2026-05-08 A012 local tool-loop coverage

- Task: Keep autonomous execution moving without external credentials by proving the Chat Completions adapter can complete a local tool loop, not just serialize requests.
- Files changed:
  - `codex-rs/core/tests/suite/client.rs`
  - `automation/queue/000-provider-phase.yaml`
  - `docs/yuqei-codex-dev-log.md`
- Tests run:
  - `just fmt` → PASS.
  - `cargo test -p codex-core chat_completions_executes_tool_call_and_replays_tool_result` → PASS.
  - `cargo test -p codex-core chat_completions` → PASS.
- Result: Added and passed a `codex-core` end-to-end regression test covering one full mock `chat_completions` tool cycle: first response emits a `shell` tool call, Codex executes it locally, the second request replays both the assistant tool call and the `role = "tool"` result, and the final assistant response completes the turn.
- Decisions:
  - This is the last meaningful local confidence layer before live DeepSeek validation.
  - Do not continue adding speculative adapter code while the remaining blocker is external credential availability.
- Blockers:
  - `DEEPSEEK_API_KEY` is still missing in the current shell, so the live DeepSeek smoke test remains blocked.
- Next: Resume A012 from the live DeepSeek smoke command once `DEEPSEEK_API_KEY` is available.

## 2026-05-09 A012 live DeepSeek smoke

- Task: Resume A012 after `DEEPSEEK_API_KEY` became available and prove the Chat Completions adapter against the real DeepSeek provider.
- Files changed:
  - `automation/queue/000-provider-phase.yaml`
  - `docs/yuqei-provider-test-results.md`
  - `docs/yuqei-codex-dev-log.md`
- Validation:
  - `printenv DEEPSEEK_API_KEY` → set.
  - `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home cargo run -p codex-cli -- exec --skip-git-repo-check "Say hello in one sentence."` → PASS.
- Result:
  - DeepSeek `deepseek-chat` successfully completed a real Codex turn through `wire_api = "chat_completions"` and returned `Hey! 👋`.
  - A012 is no longer blocked and can be marked done.
- Decisions:
  - Use a temporary `CODEX_HOME` with a minimal DeepSeek provider config so the test does not mutate the user's real `~/.codex/config.toml`.
  - Treat the source-built `codex-cli` as the authoritative validation target for this branch, because the bundled `/Applications/Codex.app` CLI still rejects `wire_api = "chat_completions"`.
- Blockers:
  - No external blocker remains for A012.
- Next:
  - Move to the next provider-compatibility task that exercises real file-write or tool-call workflows against Chat Completions providers instead of adding more text-only coverage.

## 2026-05-09 A007 OpenRouter provider test

- Task: Resume A007 after `OPENROUTER_API_KEY` became available and verify whether OpenRouter can serve as a Codex `responses` provider.
- Files changed:
  - `automation/queue/000-provider-phase.yaml`
  - `docs/yuqei-provider-test-results.md`
  - `docs/yuqei-codex-dev-log.md`
- Validation:
  - `printenv OPENROUTER_API_KEY` → set.
  - `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check "Say hello in one sentence."` → PARTIAL (`403 Forbidden: This model is not available in your region.` for `anthropic/claude-sonnet-4.5`).
  - Raw `POST https://openrouter.ai/api/v1/responses` with the same model → same `403 Forbidden` region error.
  - `curl https://openrouter.ai/api/v1/models` with auth → PASS, returned model catalog data.
  - `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec -c model="openrouter/owl-alpha" --skip-git-repo-check "Say hello in one sentence."` → PASS.
- Result:
  - OpenRouter provider compatibility is `PARTIAL`: the provider and `/v1/responses` path are usable from Codex, but the requested default model is region-blocked for this account.
  - Alternate model `openrouter/owl-alpha` proved that the same provider config can complete a full text smoke test.
- Decisions:
  - Keep the provider verdict separate from model availability. This is not a `wire_api` incompatibility.
  - Use a temporary `CODEX_HOME` for OpenRouter tests, mirroring the DeepSeek flow, so the user's real config stays untouched.
- Blockers:
  - No provider-level blocker remains for A007, but stronger OpenRouter validation still depends on choosing a model that is both available in-region and suitable for tool-call tests.
- Next:
  - Either leave A007 at `PARTIAL`, or add a follow-up task for an OpenRouter model-selection pass plus real tool-call validation.

## 2026-05-09 A013 DeepSeek workflow matrix

- Task: Extend DeepSeek validation beyond text smoke and prove the Chat Completions adapter on real coding-agent workflow tasks.
- Files changed:
  - `automation/queue/000-provider-phase.yaml`
  - `docs/yuqei-provider-test-results.md`
  - `docs/yuqei-codex-dev-log.md`
- Validation:
  - `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check "Reply with only TypeScript code for a function add(a: number, b: number): number that returns their sum. Do not inspect files. Do not use tools."` → PASS, report `automation/reports/test-runs/20260509-083511-A013-deepseek-codegen.log`.
  - `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check --sandbox read-only "Run pwd and summarize the result."` → PASS, report `automation/reports/test-runs/20260509-011225-A013-deepseek-shell.log`.
  - `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check --sandbox workspace-write "Create a file hello.txt containing hello."` → PASS, report `automation/reports/test-runs/20260509-011245-A013-deepseek-filewrite.log`.
  - `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check "Count from 1 to 30 with commas."` → PASS, report `automation/reports/test-runs/20260509-011225-A013-deepseek-stream.log`.
  - `CODEX_HOME=/tmp/codex-provider-tests/deepseek-chat-home /Users/yuqei/codex/codex-rs/target/debug/codex exec --skip-git-repo-check --sandbox read-only "List the files in the current directory and tell me which one looks like the README."` → PASS, report `automation/reports/test-runs/20260509-011245-A013-deepseek-tool.log`.
- Result:
  - DeepSeek `deepseek-chat` now has real-provider evidence for code generation, streaming, shell, file writing, and directory inspection through `wire_api = "chat_completions"`.
  - The adapter is no longer just text-smoke validated; it is workflow validated for the MVP task matrix.
- Decisions:
  - Use a temporary writable workdir under `/tmp/codex-provider-tests/deepseek-workflow` so file-write tests do not touch the repository.
- Blockers:
  - No external blocker remains for DeepSeek MVP workflow validation.
- Next:
  - If deeper confidence is needed, add a real multi-step tool-call workflow rather than more single-step prompts.

## 2026-05-09 A014 OpenRouter alternate-model workflow matrix

- Task: Strengthen A007 by proving OpenRouter on a region-available model instead of stopping at a text-only smoke test.
- Files changed:
  - `automation/queue/000-provider-phase.yaml`
  - `docs/yuqei-provider-test-results.md`
  - `docs/yuqei-codex-dev-log.md`
- Validation:
  - `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec -c model="openrouter/owl-alpha" --skip-git-repo-check --sandbox read-only "Run pwd and summarize the result."` → PASS, report `automation/reports/test-runs/20260509-011326-A014-openrouter-shell.log`.
  - `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec -c model="openrouter/owl-alpha" --skip-git-repo-check "Count from 1 to 30 with commas."` → PASS, report `automation/reports/test-runs/20260509-011326-A014-openrouter-stream.log`.
  - `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec -c model="openrouter/owl-alpha" --skip-git-repo-check --sandbox workspace-write "Create a file hello.txt containing hello."` → PASS, report `automation/reports/test-runs/20260509-011407-A014-openrouter-filewrite.log`.
  - `CODEX_HOME=/tmp/codex-provider-tests/openrouter-home /Users/yuqei/codex/codex-rs/target/debug/codex exec -c model="openrouter/owl-alpha" --skip-git-repo-check --sandbox read-only "List the files in the current directory and tell me which one looks like the README."` → PASS, report `automation/reports/test-runs/20260509-011407-A014-openrouter-tool.log`.
- Result:
  - OpenRouter `openrouter/owl-alpha` completed real shell, streaming, file-write, and directory-inspection tasks through the same `responses` provider config.
  - One tool-compatibility run auto-reconnected once before finishing, so the provider looks usable but not perfectly smooth.
- Decisions:
  - Keep the provider-level summary at `PARTIAL` because the originally targeted `anthropic/claude-sonnet-4.5` model is still region-blocked for this account.
  - Separate provider compatibility from model availability; the former is now largely proven.
- Blockers:
  - No provider-level blocker remains, but a stronger product-facing OpenRouter recommendation still depends on selecting a region-available model that matches the intended coding workload.
- Next:
  - Stop provider expansion here unless more local providers come online; the next highest-value work is provider UI/backend implementation or a real multi-step tool-call workflow.

## 2026-05-09 A015 provider list/read backend API

- Task: Start Phase C implementation with the smallest UI-facing backend slice: read-only provider APIs for list/read.
- Files changed:
  - `codex-rs/app-server-protocol/src/protocol/common.rs`
  - `codex-rs/app-server-protocol/src/protocol/v2/mod.rs`
  - `codex-rs/app-server-protocol/src/protocol/v2/provider.rs`
  - `codex-rs/app-server-protocol/schema/json/*`
  - `codex-rs/app-server-protocol/schema/typescript/*`
  - `codex-rs/app-server/src/request_processors/config_processor.rs`
  - `codex-rs/app-server/src/message_processor.rs`
  - `codex-rs/app-server/tests/common/mcp_process.rs`
  - `codex-rs/app-server/tests/suite/v2/mod.rs`
  - `codex-rs/app-server/tests/suite/v2/provider_read.rs`
  - `codex-rs/app-server/README.md`
- Validation:
  - `cargo test -p codex-app-server-protocol` → PASS.
  - `cargo test -p codex-app-server provider_read` → PASS.
  - `just write-app-server-schema` → PASS.
  - `just fmt` → PASS.
  - `just fix -p codex-app-server -p codex-app-server-protocol` → PASS.
- Result:
  - Added `provider/list` and `provider/read` RPCs in app-server v2.
  - Responses now expose effective provider summaries for builtin and custom providers, including default-provider selection, wire API, auth style, env-key name, whether auth is configured, websocket support, retry settings, and header metadata without leaking secret values.
  - Added integration coverage for listing builtin+custom providers, reading a custom provider, and rejecting unknown provider ids.
- Decisions:
  - Start with read-only APIs instead of write/test-connection to keep the first UI-facing slice conservative and easy to verify.
  - Represent provider header metadata structurally, but return only literal values and env-var names, never actual secret values.
- Blockers:
  - No immediate blocker on A015.
- Next:
  - Proceed to A016 for provider create/update/delete plus structured test-connection diagnostics.

## 2026-05-09 A016 provider write/test-connection backend API

- Task: Continue Phase C by adding the first writable provider-management RPCs plus a structured connection-testing path for both saved and draft provider configs.
- Files changed:
  - `codex-rs/app-server-protocol/src/protocol/common.rs`
  - `codex-rs/app-server-protocol/src/protocol/v2/provider.rs`
  - `codex-rs/app-server-protocol/schema/json/*`
  - `codex-rs/app-server-protocol/schema/typescript/*`
  - `codex-rs/app-server/src/request_processors/provider_processor.rs`
  - `codex-rs/app-server/src/request_processors.rs`
  - `codex-rs/app-server/src/message_processor.rs`
  - `codex-rs/app-server/tests/common/mcp_process.rs`
  - `codex-rs/app-server/tests/suite/v2/mod.rs`
  - `codex-rs/app-server/tests/suite/v2/provider_write.rs`
  - `codex-rs/app-server/README.md`
  - `codex-rs/app-server/Cargo.toml`
- Validation:
  - `cargo test -p codex-app-server provider_write` → PASS.
  - `cargo test -p codex-app-server provider_read` → PASS earlier in the same feature slice and remained compatible with A016 changes.
  - `just write-app-server-schema` → PASS.
  - `cargo test -p codex-app-server-protocol` was re-run during the slice and only drifted on vendored schema fixtures before regeneration; after schema regeneration the remaining expected step is final recheck.
- Result:
  - Added `provider/create`, `provider/update`, `provider/delete`, and `provider/testConnection` RPCs.
  - Custom providers can now be created, updated, deleted, and optionally selected as default without exposing secret values on readback.
  - `provider/testConnection` supports two targets:
    - `saved`: test an already persisted provider id with its stored auth.
    - `draft`: test an unsaved provider form without mutating the real config first.
  - Connection diagnostics now return structured per-check results for `basic`, `streaming`, and `toolCalling`, with error codes such as `MISSING_API_KEY`, `FAIL_ENDPOINT`, `FAIL_MODEL`, `FAIL_STREAM`, and `FAIL_TOOL`.
- Decisions:
  - Keep builtin provider ids immutable in this first writable slice.
  - Reuse the existing `experimental_bearer_token` config path as the minimal persistence path for UI-submitted API keys, while continuing to hide secret values from all read APIs.
  - Treat test-connection as a provider-level protocol probe: for `toolCalling`, require that the provider accepts a minimal tool schema, but do not require a full real tool loop in settings-page validation.
- Blockers:
  - No product blocker inside A016.
  - Auxiliary Bazel lock maintenance (`just bazel-lock-update` / `just bazel-lock-check`) may take a long time in this environment and should be treated as environment-sensitive validation rather than core feature logic.
- Next:
  - Proceed to A017 for provider preferences (`defaultProvider`, `defaultModel`) so the UI can complete the settings flow without hand-editing config.

## 2026-05-09 A017 provider preferences backend API

- Task: Finish the Phase C backend by adding explicit provider-preferences RPCs for default provider/model selection.
- Files changed:
  - `codex-rs/app-server-protocol/src/protocol/common.rs`
  - `codex-rs/app-server-protocol/src/protocol/v2/provider.rs`
  - `codex-rs/app-server-protocol/schema/json/*`
  - `codex-rs/app-server-protocol/schema/typescript/*`
  - `codex-rs/config/src/state.rs`
  - `codex-rs/app-server/src/config_manager_service.rs`
  - `codex-rs/app-server/src/request_processors/provider_processor.rs`
  - `codex-rs/app-server/src/message_processor.rs`
  - `codex-rs/app-server/tests/common/mcp_process.rs`
  - `codex-rs/app-server/tests/suite/v2/mod.rs`
  - `codex-rs/app-server/tests/suite/v2/provider_preferences.rs`
  - `codex-rs/app-server/tests/suite/v2/provider_write.rs`
  - `codex-rs/app-server/README.md`
- Validation:
  - `just write-app-server-schema` → PASS.
  - `cargo test -p codex-app-server-protocol` → PASS.
  - `cargo test -p codex-app-server provider_` → PASS.
- Result:
  - Added `providerPreferences/read` and `providerPreferences/update` RPCs.
  - The backend now exposes `defaultProvider`, `defaultModel`, and `configScope`.
  - Preferences can be written to either the global user config or a project `.codex/config.toml` chosen by `cwd`.
  - To support project-scope preferences, `ConfigManagerService` now allows controlled writes to project config layers instead of only the user config layer.
- Decisions:
  - Keep the preferences slice intentionally small: only default provider/model and scope, without introducing strong/fast-model split yet.
  - Unset proxy environment variables in provider connection tests so localhost wiremock probes are not polluted by the host proxy session.
- Blockers:
  - No blocker remains inside Phase C backend work.
- Next:
  - The next substantial step is Phase D UI integration against the new provider backend APIs.

## 2026-05-09 A018 minimal TUI provider preferences integration

- Task: Start Phase D by wiring the new provider-preferences backend into an existing user-visible TUI surface instead of inventing a larger settings page from scratch.
- Files changed:
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app_server_session.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
- Validation:
  - `cargo test -p codex-tui popups_and_settings` → PASS.
  - `cargo test -p codex-app-server provider_` → PASS after ensuring provider probe tests do not inherit the host proxy for localhost wiremock requests.
- Result:
  - The TUI `Settings` popup now includes a `Default provider` entry.
  - Selecting it opens a provider picker built from the effective configured providers.
  - Choosing a provider persists the default provider through `providerPreferences/update` and shows a confirmation message.
- Decisions:
  - Keep this first UI slice conservative: changing the default provider only affects future sessions, not the currently running thread.
  - Reuse the existing `Settings` popup rather than introducing a dedicated provider management page yet.
- Blockers:
  - No immediate blocker remains for the minimal UI slice.
- Next:
  - A fuller provider management UI would need a denser surface for provider CRUD and test-connection details, not just default-provider selection.

## 2026-05-09 A019 TUI provider list/detail/test flow

- Task: Continue Phase D from the minimal default-provider selector to a more explicit provider-management flow in TUI.
- Files changed:
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app_server_session.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/chatwidget/slash_dispatch.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
- Validation:
  - `cargo test -p codex-tui popups_and_settings` → PASS.
  - `cargo test -p codex-app-server provider_` → PASS.
- Result:
  - The TUI `Settings` popup now exposes a `Providers` entry instead of only a direct default-provider toggle.
  - Opening that entry shows a provider list page.
  - Selecting a provider opens a detail page that exposes:
    - `Set as default`
    - `Test connection`
  - `Test connection` now calls the backend `provider/testConnection` RPC against the saved provider and prints structured PASS/FAIL lines into history.
- Decisions:
  - Keep this first provider-management UI slice read/manage oriented, without attempting provider create/edit/delete forms yet.
  - Reuse the current model for the settings-page test-connection probe so the flow stays one-click, even though some providers may ultimately want a provider-specific model choice later.
- Blockers:
  - No immediate code blocker remains for this slice.
- Next:
  - The next larger Phase D step would be custom-provider create/edit/delete form flows, which is a separate UI surface from the list/detail/test flow completed here.

## 2026-05-09 A020 custom provider create/edit/delete TUI flow

- Task: Complete the Phase D provider-management loop by adding custom-provider create, edit, and delete flows to TUI.
- Files changed:
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app_server_session.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/chatwidget/slash_dispatch.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
- Validation:
  - `cargo test -p codex-tui popups_and_settings` → PASS.
  - `cargo test -p codex-app-server provider_` → PASS.
- Result:
  - The TUI provider list now has an `Add custom provider` action.
  - Custom providers open a detail page with `Set as default`, `Test connection`, `Edit provider`, and `Delete provider`.
  - Create and edit use a prompt-based TOML form built on `CustomPromptView`.
  - Delete uses an explicit confirmation popup.
  - Successful create/update/delete actions refresh the in-memory config and request a thread-side user-config reload.
- Decisions:
  - Use prompt-based TOML forms instead of inventing a new structured form widget.
  - Keep builtin providers immutable in the TUI flow.
  - Preserve the existing `Test connection` action as a lightweight health probe, while CRUD remains a separate interaction path.
- Blockers:
  - No blocker remains in the provider UI flow itself.
- Next:
  - Provider management is now functionally closed in TUI. The next meaningful area would be richer provider forms or broader model-catalog UX, not missing provider CRUD plumbing.

## 2026-05-09 A021 provider-aware model catalog browsing

- Task: Start Phase E by making `model/list` truly provider-aware and exposing that provider-specific catalog in TUI.
- Files changed:
  - `codex-rs/app-server/src/models.rs`
  - `codex-rs/app-server/src/request_processors/catalog_processor.rs`
  - `codex-rs/app-server/tests/suite/v2/model_list.rs`
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app_server_session.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
  - `codex-rs/app-server/README.md`
- Validation:
  - `cargo test -p codex-app-server model_list` → PASS.
  - `cargo test -p codex-tui popups_and_settings` → PASS after accepting the intentional snapshot updates for the provider detail popup and the new provider model catalog popup.
- Result:
  - `model/list` with `providerId` now returns the targeted provider's raw catalog instead of mixing remote provider models with bundled defaults.
  - The underlying fix bypasses the merged `OpenAiModelsManager` path for provider-specific app-server catalog reads and instead fetches that provider's raw `/models` catalog, then rebuilds picker-visible presets from the raw response.
  - TUI provider detail pages now include:
    - `Browse models` for non-default providers, opening a read-only provider catalog popup with capability metadata.
    - `Choose model` for the current default provider, reusing the existing interactive model picker.
- Decisions:
  - Keep non-default-provider catalog browsing read-only for now so the UI does not persist a mismatched provider/model pair.
  - Reuse the current model picker only when the provider already matches the current default provider.
- Blockers:
  - No immediate blocker remains for this slice.
- Next:
  - The next Phase E step should be interactive cross-provider provider+model selection, so a user can switch provider and choose a matching model in one persisted flow.

## 2026-05-09 A022 combined provider+model preference selection

- Task: Continue Phase E by turning the provider-specific model catalog from a read-only browser into a real provider+model default-selection flow.
- Files changed:
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
- Validation:
  - `cargo test -p codex-tui popups_and_settings` → PASS after accepting the intentional snapshot updates for the provider detail popup and provider model catalog popup.
- Result:
  - Non-default provider detail pages now expose `Choose default model` instead of a read-only browse action.
  - The provider model catalog popup is now interactive.
  - Selecting a model persists `defaultProvider` and `defaultModel` together through `providerPreferences/update`.
  - The flow still updates future-session defaults only and does not hot-swap the currently running thread's provider.
- Decisions:
  - Keep provider+model selection separate from the current-thread model picker semantics to avoid mutating the running thread into an invalid provider/model combination.
  - Leave provider-specific reasoning selection out of this slice; only the active provider continues to use the full model+reasoning picker flow.
- Blockers:
  - No immediate blocker remains for this slice.
- Next:
  - The next meaningful Phase E step is provider-aware reasoning/service-tier selection and a denser catalog experience, not basic provider/model preference plumbing.

## 2026-05-09 A023 provider-aware reasoning selection

- Task: Continue Phase E by extending the non-default-provider catalog flow to handle model-specific reasoning choices instead of always persisting the default reasoning effort.
- Files changed:
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
- Validation:
  - `cargo test -p codex-tui popups_and_settings` → PASS.
- Result:
  - Non-default provider model catalog rows now behave like this:
    - single reasoning option: persist provider + model + effort directly
    - multiple reasoning options: open a provider-aware reasoning picker first
  - The provider-aware reasoning picker reuses the existing reasoning popup UX, but its persistence target is future-session defaults rather than the currently running thread.
  - Persistence now happens in two existing lanes:
    - `providerPreferences/update` persists default provider + default model
    - `ConfigEditsBuilder::set_model(...)` persists model + reasoning effort
- Decisions:
  - Keep the current-thread model picker and the provider-preference picker separate so a settings action cannot hot-swap the live thread into a mismatched provider/model state.
  - Do not widen the app-server protocol for reasoning effort in this slice; reuse the existing config edit path instead.
- Blockers:
  - No immediate blocker remains for this slice.
- Next:
  - The next Phase E step should be provider-aware service-tier selection or a denser catalog presentation with stronger/fast/default grouping, not more provider/model plumbing.

## 2026-05-10 A024 provider-aware service-tier selection

- Task: Continue Phase E by extending provider-specific future-default selection to cover Fast-mode capable models.
- Files changed:
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
- Validation:
  - `cargo test -p codex-tui popups_and_settings` → PASS after accepting the new provider service-tier popup snapshot.
- Result:
  - Fast-capable provider models now branch into a provider-specific service-tier popup.
  - The popup offers `Standard` and `Fast`, then persists provider + model + reasoning + service tier together for future sessions.
  - Provider-specific persistence still reuses two existing lanes:
    - `providerPreferences/update` for default provider + default model
    - `ConfigEditsBuilder` for reasoning effort + service tier
- Decisions:
  - Keep service-tier preference future-session-only in this flow; do not override the currently running thread.
  - Do not widen app-server protocol payloads for service tier here; reuse existing config persistence.
- Blockers:
  - No immediate blocker remains for this slice.
- Next:
  - The remaining Phase E gap is catalog presentation density, not more provider/model preference plumbing.

## 2026-05-10 A025 layered model catalog grouping

- Task: Finish the Phase E catalog UX line by grouping model lists into more scannable sections.
- Files changed:
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
- Validation:
  - `cargo test -p codex-tui popups_and_settings` → PASS after accepting the updated snapshots for:
    - `model_picker_filters_hidden_models`
    - `model_selection_popup`
    - `provider_model_catalog_popup`
- Result:
  - TUI model catalogs now render grouped sections:
    - `Default`
    - `Fast`
    - `Strong reasoning`
    - `Other`
  - The grouping now appears both in provider-specific model catalogs and in the full `Select Model and Effort` popup.
- Decisions:
  - Keep grouping purely presentational; do not change the selection semantics or persistence paths.
  - Preserve input ordering within each group rather than adding a second sort policy.
- Blockers:
  - No immediate blocker remains for this slice.
- Next:
  - Phase E is now effectively closed for the current plan. The next meaningful line of work is Phase G workflow productization, unless provider live-service blockers (`A004` Ollama, `A005` LM Studio) become unblocked externally.

## 2026-05-10 A026 provider setup workflow command

- Task: Start Phase G by giving the provider-management flow a dedicated slash-command launcher instead of relying only on the Settings menu.
- Files changed:
  - `codex-rs/tui/src/slash_command.rs`
  - `codex-rs/tui/src/chatwidget/slash_dispatch.rs`
  - `codex-rs/tui/src/bottom_pane/command_popup.rs`
  - `codex-rs/tui/src/bottom_pane/chat_composer.rs`
  - `codex-rs/tui/src/chatwidget/tests/slash_commands.rs`
  - `codex-rs/tui/src/bottom_pane/snapshots/*`
- Validation:
  - `cargo test -p codex-tui providers` → PASS after accepting the new `/pro` command-popup snapshot.
- Result:
  - TUI now supports `/providers` as a workflow entrypoint.
  - Supported variants:
    - `/providers`
    - `/providers new`
    - `/providers current`
    - `/providers <provider-id>`
    - `/providers models <provider-id|current>`
    - `/providers test <provider-id|current>`
  - The command popup now exposes `/providers` and selects it for the `/pro` prefix.
- Decisions:
  - Reuse the existing provider-management UI instead of inventing a separate provider-setup surface.
  - Keep `/providers` scoped to workflow launching; it does not add a new backend state machine.
- Blockers:
  - No immediate blocker remains for this slice.
- Next:
  - The next Phase G slice should be another command-first workflow with missing entrypoints, most likely `commit workflow` or `context save/restore`.

## 2026-05-10 A027 commit drafting workflow command

- Task: Continue Phase G by adding a command-first commit workflow that packages git context into an editable drafting prompt.
- Files changed:
  - `codex-rs/tui/src/slash_command.rs`
  - `codex-rs/tui/src/chatwidget/slash_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/get_git_status.rs`
  - `codex-rs/tui/src/chatwidget/tests/slash_commands.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/bottom_pane/command_popup.rs`
  - `codex-rs/tui/src/bottom_pane/chat_composer.rs`
  - `codex-rs/tui/src/bottom_pane/snapshots/*`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
- Validation:
  - `cargo test -p codex-tui commit --no-fail-fast` → PASS after accepting:
    - `slash_popup_co`
    - `commit_workflow_prompt`
- Result:
  - TUI now supports `/commit` as a workflow launcher.
  - The workflow gathers:
    - `git status --short --branch --untracked-files=all`
    - current git diff
  - After collection completes, TUI opens an editable `Commit workflow` prompt instead of auto-running `git commit`.
  - Submitting that prompt routes through the normal user-message flow, so the agent can draft a commit message without direct git side effects.
  - Inline `/commit ...` args are appended as additional drafting instructions.
- Decisions:
  - Keep this first commit workflow draft-only; do not run `git commit`, stage files, or touch the repository automatically.
  - Reuse `CustomPromptView` so the user can edit or narrow the drafting request before sending it to the agent.
- Blockers:
  - No immediate blocker remains for this slice.
- Next:
  - The next Phase G slice should be `context save/restore` or a guarded commit-execution flow, not more provider/model setup work.

## 2026-05-10 A028 context save/restore workflow command

- Task: Continue Phase G by turning existing rollout/resume mechanics into a single command-first context workflow.
- Files changed:
  - `codex-rs/tui/src/slash_command.rs`
  - `codex-rs/tui/src/chatwidget/slash_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/chatwidget/tests/slash_commands.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/bottom_pane/command_popup.rs`
  - `codex-rs/tui/src/bottom_pane/chat_composer.rs`
  - `codex-rs/tui/src/bottom_pane/snapshots/*`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
  - `codex-rs/tui/src/chatwidget/tests/snapshots/*`
- Validation:
  - `cargo test -p codex-tui context --no-fail-fast` → PASS after accepting:
    - `slash_popup_con`
    - `context_workflow_popup`
    - `context_save_summary`
- Result:
  - TUI now supports `/context` as a workflow launcher.
  - Supported variants:
    - `/context`
    - `/context save`
    - `/context restore`
    - `/context restore <thread-id-or-name>`
  - `/context save` now emits a resumable-context summary that includes thread, model, cwd, rollout path, and a concrete resume command.
  - `/context restore` reuses the existing resume picker, while `/context restore <id-or-name>` jumps directly through the existing named-session resume path.
- Decisions:
  - Reuse the existing rollout/session persistence model instead of inventing a second checkpoint artifact.
  - Keep `save` informational in this slice: it surfaces the resume handles that already exist rather than writing a separate exported file.
- Blockers:
  - No immediate blocker remains for this slice.
- Next:
  - The next Phase G slice should be a guarded commit-execution flow or a more structured review/QA workflow entrypoint.

## 2026-05-10 A027 commit drafting workflow command

- Task: Continue Phase G by adding a command-first commit workflow that packages git context into an editable drafting prompt.
- Files changed:
  - `codex-rs/tui/src/slash_command.rs`
  - `codex-rs/tui/src/chatwidget/slash_dispatch.rs`
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/app_event.rs`
  - `codex-rs/tui/src/app/event_dispatch.rs`
  - `codex-rs/tui/src/get_git_status.rs`
  - `codex-rs/tui/src/chatwidget/tests/slash_commands.rs`
  - `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`
  - `codex-rs/tui/src/bottom_pane/command_popup.rs`
  - `codex-rs/tui/src/bottom_pane/chat_composer.rs`
  - `codex-rs/tui/src/bottom_pane/snapshots/*`
  - `codex-rs/tui/src/chatwidget/snapshots/*`
- Validation:
  - `cargo test -p codex-tui commit --no-fail-fast` → PASS after accepting the new `/co` popup snapshot and the new commit-workflow prompt snapshot.
- Result:
  - TUI now supports `/commit` as a workflow launcher.
  - The command gathers:
    - `git status --short --branch --untracked-files=all`
    - current git diff via the existing diff helper
  - After collection completes, TUI opens an editable `Commit workflow` prompt rather than auto-submitting a commit or shell command.
  - Submitting that prompt routes through the normal user-message flow, so the agent can draft a commit message without direct git side effects.
  - Inline args on `/commit ...` are appended as additional drafting instructions.
- Decisions:
  - Keep the workflow draft-only in this slice; do not run `git commit`, stage files, or touch the repository automatically.
  - Reuse `CustomPromptView` so the user can edit or narrow the drafting request before sending it to the agent.
- Blockers:
  - No immediate blocker remains for this slice.
- Next:
  - The next Phase G slice should be `context save/restore` or a guarded `commit execution` flow, not more provider/model setup work.
