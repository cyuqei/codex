# Yuqei Devflow Warp 开发方案

日期：2026-05-18

适用源码路径：`/Users/yuqei/warp`

关联 Codex 源码：`/Users/yuqei/codex-provider`

关联总方案：`/Users/yuqei/codex-provider/codex-rs/docs/yuqei-devflow-platform-plan.md`

关联 Codex 方案：`/Users/yuqei/codex-provider/codex-rs/docs/yuqei-devflow-codex-development-plan.md`

## 1. 开发目标

Warp 在 Yuqei Devflow 中是用户面对的产品入口和自动化开发工作台。

新的主链路固定为：

```text
用户 -> Warp Devflow UI -> Codex app-server -> Codex Devflow backend
```

Warp 不再把 Hermes 或 Claude Code 作为 Devflow 主路径。Warp 只需要展示和操作：

- Codex 状态。
- Devflow task/run/worktree/diff/gate/review/artifact。
- Superpowers PolicyPack 状态。
- gstack CapabilityPack 状态和证据。
- approval、watchdog alert、失败回派和最终合并/发布 gate。

## 2. Warp 职责边界

Warp 负责：

- Devflow 产品入口。
- 项目选择和本地路径配置。
- Codex app-server 连接、启动、诊断和重连。
- Task 创建、启动、暂停、恢复、取消。
- Run 输出、命令、文件变化、错误摘要展示。
- Worktree、branch、diff、changed files 展示。
- Quality Gate、Review、Artifact Timeline 展示。
- Approval Center。
- PolicyPack / CapabilityPack 启用状态展示。
- gstack browser QA 截图、health 结果、watchdog alert 展示。

Warp 不负责：

- 直接实现代码修改。
- 直接执行 shell/file runtime。
- 自己维护一套 task store。
- 自己合并 worktree。
- 直接读取 gstack 或 Superpowers 内部状态作为事实来源。
- 将 Hermes/Claude Code 放回主链路。

所有事实状态以 Codex app-server 返回为准。Warp 本地只做 UI 缓存和交互状态。

## 3. 当前代码基线

Warp 里已经有 Devflow 和本地 agent 相关骨架：

- `app/src/ai/devflow/protocol.rs`
- `app/src/ai/devflow/app_server.rs`
- `app/src/ai/devflow/app_server_model.rs`
- `app/src/ai/devflow/display.rs`
- `app/src/ai/devflow/notifications.rs`
- `app/src/ai/devflow/approval_actions.rs`
- `app/src/ai/devflow/artifact_actions.rs`
- `app/src/ai/agent_management/agent_management_model.rs`
- `app/src/ai/agent_management/view.rs`
- `app/src/ai/agent_sdk/driver/harness/codex.rs`
- `app/src/workspace/view/codex_modal.rs`

需要注意：当前 Devflow UI 文案和测试中仍有 Hermes delivery 相关内容，Agent Management 也保留 Claude/本地 CLI agent 旧路径。新方案中这些内容应迁移为 legacy 或从 Devflow 主入口移除。

## 4. 产品信息架构

建议新增或收敛到以下入口：

```text
Devflow
  Overview
  Task Board
  Run Detail
  Diff and Review
  Quality Gates
  Approvals
  Artifacts
  Policy Packs
  Capability Packs
  Diagnostics
  Settings
```

MVP 可以先不做完整一级导航，先在现有 AI / Local Agent 入口下增加 Devflow 面板，等单任务闭环稳定后再升级为主入口。

## 5. 本地路径和设置

默认路径：

```text
Codex: /Users/yuqei/codex-provider/codex-rs
Warp: /Users/yuqei/warp
Superpowers: /Users/yuqei/superpowers
gstack: /Users/yuqei/gstack
```

环境覆盖：

```sh
export YUQEI_CODEX_ROOT=/Users/yuqei/codex-provider/codex-rs
export YUQEI_WARP_ROOT=/Users/yuqei/warp
export YUQEI_SUPERPOWERS_ROOT=/Users/yuqei/superpowers
export YUQEI_GSTACK_ROOT=/Users/yuqei/gstack
```

设置页应显示：

- 当前路径。
- 路径来源：default、env、user setting。
- manifest 是否存在。
- build/smoke/start command。
- app-server ready 状态。
- 最近错误。
- 可用 policy/capability 列表。

## 6. Codex app-server 连接层

目标：Warp 能稳定连接和驱动 `/Users/yuqei/codex-provider/codex-rs` 的 app-server。

优先连接方式：

1. Codex app-server 已支持的本地连接方式。
2. Unix socket。
3. stdio proxy。
4. localhost websocket。

连接层需要管理：

- app-server 是否运行。
- 启动命令。
- initialize/initialized handshake。
- server version。
- experimental API 能力。
- 心跳或 ready check。
- 断线重连。
- schema/method 不兼容错误。

错误展示要区分：

- Codex 路径不存在。
- 构建产物不存在。
- 启动命令失败。
- app-server 未 ready。
- initialize 失败。
- method 不存在。
- schema 不兼容。
- 权限不足。
- socket/port 被占用。

每类错误都应有：

- 简短摘要。
- 原始错误展开。
- 推荐修复动作。
- copy command。
- open checkout。

## 7. Devflow Overview

第一屏应显示：

- 当前项目。
- Codex app-server 状态。
- 最近 task。
- 新建任务输入框。
- 当前启用的 PolicyPack。
- 当前启用的 CapabilityPack。
- 待审批数量。
- 最近失败 gate。
- watchdog alert 数量。

MVP 首屏目标：

```text
选择项目 -> 输入需求 -> 创建 task -> 启动 Codex run -> 看到输出
```

## 8. Project Selector

Warp 应允许用户选择 Devflow 目标项目。

显示：

- 项目路径。
- git branch。
- remote。
- dirty state。
- trust state。
- detected docs。
- detected test commands。
- active tasks。

操作：

- Open project。
- Trust project。
- Run diagnose。
- Refresh git state。
- Configure test commands。

MVP 可以先使用当前 Warp workspace/cwd 作为项目来源。

## 9. Task Creation Flow

输入字段：

- objective。
- project。
- task kind。
- risk level。
- allow worktree。
- enabled policy packs。
- enabled capability packs。
- test command preference。
- approval profile。

MVP 简化字段：

- objective。
- project。
- mode：implementation / review / diagnostic。

创建后行为：

- 调用 `devflowTask/create`。
- 展示 Context Pack 摘要。
- 展示 policy/capability 将如何应用。
- 用户点击 Start 或按设置自动 start 低风险任务。

## 10. Task Board

列设计：

- Planned。
- Running。
- Blocked。
- Reviewing。
- Testing。
- Ready to Merge。
- Done。
- Failed。

Task card 显示：

- title。
- kind。
- status。
- assigned Codex role。
- risk level。
- worktree path。
- branch。
- latest event。
- quality gate state。
- policy pack state。
- capability pack state。
- artifact count。
- watchdog alert badge。

操作：

- Start。
- Pause。
- Resume。
- Cancel。
- Open run detail。
- Open diff。
- Run gate。
- Send to review。
- Return for fix。
- Accept / ready to merge。

所有操作都调用 Codex app-server，不能只在 Warp 本地改状态。

## 11. Run Detail

Run Detail 是用户判断自动化是否靠谱的核心界面。

显示：

- 任务目标。
- Codex role。
- started/completed time。
- status。
- stream output。
- command list。
- file changes。
- errors。
- quality gate results。
- artifacts。
- token/cost。
- exit reason。

输出流应结构化：

- Agent message。
- Plan updated。
- Command started。
- Command output。
- File changed。
- Approval requested。
- Gate completed。
- Artifact created。
- Watchdog alerted。

操作：

- Interrupt。
- Add instruction。
- Return for fix。
- Create follow-up task。
- Generate report。

## 12. Diff and Review

Diff 面板显示：

- worktree。
- branch。
- changed files。
- diff。
- summary。
- risk markers。
- test coverage markers。

Review 面板显示：

- Findings。
- Risks。
- Missing tests。
- Behavior changes。
- Suggested fixes。
- Final recommendation。

操作：

- Open file。
- Open worktree。
- Send to Codex review。
- Return for fix。
- Accept。
- Discard task。

如果 review 通过，明确显示 Ready to Merge。如果不通过，提供 Return for fix。

## 13. Quality Gates

Gate 列表显示：

- gate name。
- source：Codex / Superpowers / gstack。
- command。
- cwd。
- status。
- exit code。
- duration。
- summary。
- artifact。

操作：

- Run。
- Rerun。
- Open output。
- Copy command。
- Waive。

`Waive` 必须进入 Codex approval flow。

第一批 UI gate：

- format。
- lint。
- typecheck。
- targeted test。
- build。
- review。
- gstack health。
- gstack browser QA。
- gstack watchdog。

## 14. Approval Center

审批类型：

- shell command。
- file write outside scope。
- dependency change。
- git commit。
- git push。
- delete operation。
- long-running process。
- quality gate waive。
- external network action。

审批动作：

- Allow once。
- Deny。
- Allow for task。
- Allow by project policy。

每个审批必须展示：

- 请求来源。
- task。
- run。
- Codex role。
- 命令或动作。
- 风险说明。
- 原始 payload。
- 推荐决策。

## 15. Artifact Timeline

Timeline 是 Devflow 的审计和恢复入口。

显示：

- task created。
- policy pack applied。
- context pack created。
- worktree created。
- run started。
- command started/completed。
- diff updated。
- gate completed。
- review completed。
- gstack browser QA evidence。
- watchdog alert。
- approval requested/responded。
- merge completed。

每个 item 可展开 artifact。长输出默认转 artifact，不塞进 UI 大文本块。

## 16. Policy Packs UI

来源：Superpowers。

显示：

- 当前项目启用的 policy。
- 每个 task 实际触发的 policy。
- policy 是否通过。
- 缺少哪个 artifact 或 gate。
- 是否被 waive。

第一批 policy：

- writing-plans。
- using-git-worktrees。
- systematic-debugging。
- verification-before-completion。
- requesting-code-review。
- finishing-a-development-branch。

UI 行为：

- 中高风险任务没有计划时显示阻塞原因。
- 没有验证证据时不能显示 Done。
- review feedback 未处理时显示 Return for fix。

## 17. Capability Packs UI

来源：gstack。

显示：

- 当前项目启用的 capability。
- capability 运行状态。
- 最近结果。
- artifact 链接。
- 失败摘要。

第一批 capability：

- health。
- browse / qa。
- review。
- benchmark。
- canary。
- watchdog queue。

gstack browser QA 需要展示：

- 截图。
- 访问 URL。
- 视口。
- 失败步骤。
- console/network 错误摘要。

watchdog 需要展示：

- 当前 running task。
- 心跳时间。
- no progress。
- timed out。
- false running。
- zero worker。
- recovery action。

## 18. Diagnostics

全局诊断：

- Codex root。
- Codex app-server。
- Superpowers root。
- gstack root。
- git。
- rg。
- cargo。
- bun/node。
- network/proxy if needed。

项目诊断：

- git repo。
- branch。
- dirty state。
- remote。
- trust。
- AGENTS.md。
- test commands。
- build tools。
- writable roots。

诊断结果必须可操作：

- status。
- summary。
- details。
- fix command。
- open path。
- copy logs。

## 19. Settings

Agent / pack paths：

- Codex root。
- Superpowers root。
- gstack root。
- Codex app-server launch command。

Permission profile：

- default approval mode。
- auto start low-risk task。
- allow managed worktree creation。
- allow local tests。
- require approval for git commit。
- require approval for git push。
- require approval for external network action。

Quality gates：

- default fmt command。
- default lint command。
- default test command。
- full test command。
- build command。
- snapshot command。
- gstack health command。
- browser QA profile。

## 20. 分阶段开发计划

### Phase 0：连接和路径校准

目标：Warp 看见正确的本地组件。

任务：

- 将默认 Codex 路径改为 `/Users/yuqei/codex-provider/codex-rs`。
- 增加 Superpowers 和 gstack 路径配置。
- 移除 Devflow 主入口中的 Hermes/Claude Code 必需项。
- 连接 Codex app-server。
- 显示 initialize / version / capability。

验收：

- Devflow 面板能显示 Codex 可用性。
- 能显示 Superpowers/gstack 路径是否存在。
- Codex 连接失败时有明确错误和修复命令。

### Phase 1：单任务创建和运行

目标：用户能从 Warp 发起 Codex 开发任务。

任务：

- Task creation form。
- 调用 `devflowTask/create`。
- 调用 `devflowTask/start`。
- 订阅 run output。
- 显示 task status。
- 显示 run detail。

验收：

- 用户输入需求后能创建 task。
- task 能进入 running。
- Codex 输出能实时显示。
- 完成后有 final status。

### Phase 2：diff、worktree、artifact

目标：用户能看懂 Codex 做了什么。

任务：

- Worktree summary。
- Changed files list。
- Diff panel。
- Artifact list。
- Artifact detail。
- Timeline。

验收：

- task 完成后能看到 diff。
- 能看到 worktree path 和 branch。
- 能打开 test output artifact。
- 能打开 review report artifact。

### Phase 3：quality gate、review、approval

目标：用户能判断任务能否合并。

任务：

- Quality Gates panel。
- Gate status badge。
- Gate output viewer。
- Review report view。
- Approval Center。
- Return for fix action。
- Rerun gate action。
- Waive gate approval flow。

验收：

- 测试失败能明确展示。
- review finding 能明确展示。
- 用户能一键回派修复。
- waive 必须产生审批记录。

### Phase 4：PolicyPack 和 CapabilityPack

目标：把 Superpowers/gstack 融合结果产品化。

任务：

- Policy Packs tab。
- Capability Packs tab。
- Task card 显示 pack 状态。
- gstack health result viewer。
- browser QA evidence viewer。
- watchdog alert viewer。

验收：

- 用户能看到任务被哪些 Superpowers policy 约束。
- 用户能看到 gstack 输出的 health/browser QA/watchdog artifact。
- Pack 失败能进入 gate failure 或 alert，而不是让 UI 状态错乱。

### Phase 5：Task Board 和多任务

目标：支持中型需求和多任务并行。

任务：

- Task Board columns。
- Dependency visualization。
- Codex role assignment view。
- Parallel run status。
- Blocked state。
- Merge readiness。

验收：

- 多个 task 可以同时展示。
- 每个 task 有独立状态。
- 有依赖的 task 不会误显示为可执行。
- 冲突或 gate 失败能进入 blocked/failed。

### Phase 6：完成分支和交付准备

目标：支持从需求到 PR 准备的低人工干预闭环。

任务：

- Ready to merge view。
- Final gate summary。
- Commit message artifact。
- PR body artifact。
- Release note artifact。
- Export support bundle。

验收：

- 用户能看到完整证据链后批准 merge/commit/push。
- 所有高风险动作都走 approval。
- 支持失败时导出诊断材料。

## 21. 测试要求

优先测试：

- `app/src/ai/devflow/protocol.rs`
- `app/src/ai/devflow/app_server_model.rs`
- `app/src/ai/devflow/display.rs`
- `app/src/ai/devflow/notifications.rs`
- `app/src/ai/devflow/approval_actions.rs`
- `app/src/ai/devflow/artifact_actions.rs`
- `app/src/ai/agent_management/*`

建议覆盖：

- Codex 连接失败分类。
- Task create/start 状态流。
- output delta 展示。
- gate completed 展示。
- approval requested/responded 展示。
- artifact timeline 排序。
- policy/capability pack 状态。
- watchdog alert 展示。

## 22. Warp 侧最终验收

Warp 方案完成后应满足：

- 能连接 `/Users/yuqei/codex-provider/codex-rs` 的 Codex app-server。
- 能创建 Devflow task。
- 能启动 Codex run。
- 能实时显示 run output。
- 能显示 worktree、branch、diff。
- 能显示 quality gate 和 review artifact。
- 能处理 approval。
- 能展示 Superpowers PolicyPack 状态。
- 能展示 gstack CapabilityPack 证据。
- 能展示 watchdog alert。
- 能回派修复。
- 能展示最终 ready-to-merge 证据链。
- 主链路不依赖 Hermes 或 Claude Code。

## 23. 优先级结论

Warp 开发顺序：

1. Codex app-server 连接和诊断。
2. 单任务创建、启动、输出。
3. run detail、diff、artifact timeline。
4. quality gate、review、approval。
5. Superpowers PolicyPack UI。
6. gstack CapabilityPack UI。
7. task board、多任务和 watchdog。
8. 最终 merge/commit/PR 准备。

第一阶段不要先做复杂看板。先把这条闭环做稳：

```text
用户输入需求
  -> Warp 创建 task
  -> Codex 执行
  -> Warp 显示输出、diff、test、review、artifact
  -> 用户批准或回派修复
```

这条链路稳定后，再扩大到多任务并行、browser QA、watchdog 和发布准备。
