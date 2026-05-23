# Yuqei Devflow Codex 开发方案

日期：2026-05-18

适用源码路径：`/Users/yuqei/codex-provider`

主要 Rust 工作区：`/Users/yuqei/codex-provider/codex-rs`

关联总方案：`/Users/yuqei/codex-provider/codex-rs/docs/yuqei-devflow-platform-plan.md`

关联 Warp 源码：`/Users/yuqei/warp`

## 1. 开发目标

Codex 在 Yuqei Devflow 中只承担一件事：成为 Warp + Codex 自动化软件开发平台的主控制平面、主 agent runtime 和 Devflow backend。

本方案不再使用 Hermes 和 Claude Code 作为 Devflow 主链路。新的主链路固定为：

```text
Warp UI
  -> Codex app-server / Devflow API
  -> Codex Runtime / tools / subagents / worktree / quality gates
```

Superpowers 和 gstack 可以融合，但只作为增强包：

- Superpowers：`PolicyPack`，提供计划、worktree、TDD、系统化调试、review、完成前验证等流程策略。
- gstack：`CapabilityPack`，提供 browser QA、health、review、benchmark、canary、watchdog 等工程能力。

## 2. Codex 职责边界

Codex 负责：

- Devflow 协议和 schema。
- Project、Task、Run、Worktree、Artifact、QualityGate、Approval 的状态管理。
- Planner、Worker、Reviewer、QA、Integrator 等 Codex 内部角色。
- Codex thread/turn/review/tools 的编排。
- worktree 创建、diff、merge、cleanup。
- shell、patch、文件读写、测试命令和质量门执行。
- approval 请求、响应、策略和事件通知。
- Superpowers PolicyPack 的应用。
- gstack CapabilityPack 的调用和结果记录。
- 给 Warp 提供稳定 app-server v2 API 和事件流。

Codex 不负责：

- 产品 UI。
- Warp 本地导航、看板、diff 展示和交互。
- 直接持有 Warp UI 状态。
- 引入第二套 agent runtime。
- 让 Warp 直接读 Codex 内部私有结构。
- 把 gstack 或 Superpowers 变成独立控制面。

## 3. 当前代码基线

当前仓库已经有 Devflow 相关骨架：

- `codex-rs/app-server-protocol/src/protocol/v2/devflow.rs`
- `codex-rs/app-server/src/request_processors/devflow_processor.rs`
- `codex-rs/app-server/src/request_processors/devflow_project.rs`
- `codex-rs/app-server/src/request_processors/devflow_worktree.rs`
- `codex-rs/app-server/src/request_processors/devflow_quality_gate.rs`
- `codex-rs/app-server/src/request_processors/devflow_approval.rs`
- `codex-rs/app-server/tests/suite/v2/devflow.rs`

需要注意：当前部分 Devflow 代码还保留 Claude / Hermes runtime，以及外部 artifact delivery / agent adapter 设计。新路线中这些外部消息能力应被迁移、废弃或隔离为 legacy；Codex 主路径可以保留本地 handoff / receipt，但不应把外发消息能力当成主链路必需项。

## 4. 核心原则

### 4.1 app-server v2 是唯一产品边界

Warp 只通过 app-server v2 调用 Codex。新增 API 不直接暴露 Rust 私有类型，不让 Warp 解析内部 store 文件。

### 4.2 Codex 是唯一 agent runtime

所有角色都映射为 Codex 内部 role、context pack、tool set、approval policy、model/reasoning 配置或 subagent，而不是映射成外部程序。

### 4.3 不继续膨胀 codex-core

Devflow 是平台编排能力，优先放在 app-server、protocol、store、orchestrator、quality gate、artifact 等边界。只有真正影响 Codex turn loop 的逻辑才进入 `codex-core`。

### 4.4 默认 worktree 隔离

实现型任务默认创建 managed worktree。只读分析、低风险诊断或用户明确指定的小改动可以不创建 worktree。

### 4.5 所有完成声明必须有证据

任务进入 Done 前必须有关联 artifact，例如 diff、test output、review report、QA screenshot、health summary 或 completion report。

## 5. 目标模块划分

### 5.1 Protocol

主要文件：

- `codex-rs/app-server-protocol/src/protocol/v2/devflow.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/mod.rs`
- `codex-rs/app-server-protocol/src/protocol/common.rs`

目标：

- 保留现有 Devflow Project、Task、Run、Worktree、Artifact、QualityGate、Approval 类型。
- 新增 `DevflowPolicyPack`、`DevflowCapabilityPack`、`DevflowWatchdogAlert`。
- 将新的主线 runtime 约束为 Codex。
- 新增或调整 schema fixtures 和 TypeScript 生成物。

建议 API：

```text
devflowProject/list
devflowProject/open
devflowProject/read
devflowProject/diagnose
devflowTask/create
devflowTask/plan
devflowTask/dispatch
devflowTask/start
devflowTask/pause
devflowTask/resume
devflowTask/cancel
devflowTask/read
devflowTask/list
devflowWorktree/create
devflowWorktree/read
devflowWorktree/list
devflowWorktree/diff
devflowWorktree/merge
devflowWorktree/cleanup
devflowQualityGate/list
devflowQualityGate/run
devflowQualityGate/read
devflowQualityGate/rerun
devflowQualityGate/waive
devflowPolicyPack/list
devflowPolicyPack/read
devflowPolicyPack/apply
devflowCapabilityPack/list
devflowCapabilityPack/read
devflowCapabilityPack/run
devflowApproval/list
devflowApproval/respond
devflowApproval/policy/read
devflowApproval/policy/update
devflowArtifact/list
devflowArtifact/read
devflowArtifact/open
devflowArtifact/export
devflowWatchdog/read
devflowWatchdog/alerts
```

### 5.2 Store

目标：

- 统一保存 Project、Task、Run、Worktree、Artifact、QualityGate、Approval、Pack、Watchdog 状态。
- 先支持 in-memory store，后续可落盘到 Codex home 或项目 `.codex/devflow/`。
- 所有状态变更都生成 event，供 Warp UI 恢复。

关键要求：

- Task 和 Worktree 必须可反查。
- Run 必须记录 command、output summary、file changes、exit reason。
- QualityGate 必须关联 task、run、worktree、commit。
- Artifact 必须可打开、导出、追溯来源。
- Pack 失败不能破坏 task 状态，只能形成 gate failure 或 alert。

### 5.3 Orchestrator

目标：

- 将用户目标变成任务图。
- 选择 Codex role。
- 生成 Context Pack。
- 启动 Codex thread/turn。
- 处理 pause/resume/cancel。
- 处理失败回派和重试。

默认角色：

- Planner：需求澄清、任务图、Context Pack。
- Worker：代码实现、局部测试、修复。
- Reviewer：diff review、风险检查、测试覆盖检查。
- QA：format、lint、typecheck、test、build、gstack gate。
- Integrator：合并 worktree、最终 gate、commit/PR 文案。

### 5.4 Worktree Manager

目标：

- 为实现型任务创建 managed worktree。
- 生成 branch name。
- 绑定 task/worktree/base commit/head commit。
- 提供 diff、merge、cleanup。

安全策略：

- 不清理 primary worktree。
- 不清理 unknown owner worktree。
- dirty worktree 不静默删除。
- unmanaged worktree 不清理。
- merge 前必须有 diff、gate、review artifact。

### 5.5 Quality Gate Engine

目标：

- 运行格式化、lint、typecheck、targeted test、integration test、snapshot、build。
- 支持 gstack health/browser QA/benchmark/canary 作为 gate。
- Gate 失败时生成失败摘要和 artifact。
- Gate waive 必须走 approval。

第一批 gate：

- `format`
- `lint`
- `typecheck`
- `targetedTest`
- `build`
- `review`
- `gstackHealth`
- `gstackBrowserQa`
- `gstackWatchdog`

### 5.6 Policy Pack Engine

来源：`/Users/yuqei/superpowers`

第一批策略：

- `writingPlans`
- `worktreeIsolation`
- `systematicDebugging`
- `verificationBeforeCompletion`
- `requestingCodeReview`
- `finishBranch`

行为：

- 低风险任务启用轻量策略。
- 中高风险任务必须生成计划、worktree、验证证据和 review gate。
- bug 修复任务必须先记录 root cause。
- 没有验证 artifact 的任务不能 Done。

### 5.7 Capability Pack Engine

来源：`/Users/yuqei/gstack`

第一批能力：

- `health`
- `browseQa`
- `review`
- `benchmark`
- `canary`
- `watchdogQueue`

行为：

- 由 Codex 统一调用和记录。
- Warp 只看 Codex 返回的 artifact/event。
- 不让 Warp 直接读 gstack 状态文件。
- 浏览器 daemon 的启动、权限、日志和回收由 Codex 管理。

### 5.8 Watchdog / Queue

目标：

- 借鉴 gstack 4+6 orchestration 的 queue、worker、auditor、watchdog 思路。
- 维护 running worker 心跳。
- 检测 no progress、timed out、false running、zero worker。
- 生成 alert，交给 Warp 展示或回派。

状态：

- `idle`
- `running`
- `noProgress`
- `timedOut`
- `recovering`
- `quarantined`

## 6. 分阶段开发计划

### Phase 0：路线校准和旧 runtime 隔离

目标：把当前 Devflow 从旧的多 runtime 设想校准到 Warp + Codex 主线。

任务：

- 盘点 `DevflowAgentRuntime::Claude` / `Hermes` 使用点。
- 将新文档和协议明确为 Codex-only 主路径。
- 标记外部 artifact delivery / external agent adapter 为 legacy 或后续可选功能；本地 handoff/receipt 仍可保留在 Codex 主路径内。
- 定义 Superpowers/gstack 的 pack schema。
- 更新 `app-server/README.md` 中的 Devflow 说明。

验收：

- 文档和协议不再要求 Hermes/Claude Code 参与主流程。
- Warp 可识别 Codex、Superpowers、gstack 三类本地路径。
- 当前旧代码不会阻塞 Codex-only MVP。

### Phase 1：Protocol 和 store 最小闭环

目标：Warp 能通过 app-server 创建、读取和观察 Devflow task。

任务：

- 补齐 Devflow v2 request/response/notification 类型。
- 增加 PolicyPack、CapabilityPack、Watchdog 类型。
- 完成 task create/read/list/statusChanged。
- 完成 artifact created/list/read。
- 生成 schema 和 TypeScript 类型。

验收：

- `devflowTask/create` 可以创建任务。
- `devflowTask/read` 可以读取任务。
- `devflowArtifact/list` 可以列出任务 artifact。
- Warp 可以订阅基础事件。

建议验证：

```sh
just write-app-server-schema
cargo test -p codex-app-server-protocol
```

### Phase 2：Codex 单任务执行闭环

目标：从 Warp 输入需求，到 Codex 完成一个真实 coding turn。

任务：

- `devflowTask/start` 创建 Run。
- 生成最小 Context Pack。
- 调用 Codex thread/turn。
- 将输出映射为 `devflowRun/outputDelta`。
- 记录 command started/completed。
- 生成 run summary artifact。

验收：

- Warp 输入一个小需求后，Codex 能执行并返回流式输出。
- Task 从 planned 到 running，再到 readyForReview 或 failed。
- Run 有明确 exit reason。

### Phase 3：worktree、diff、cleanup

目标：实现型任务默认隔离。

任务：

- 实现 managed worktree create/read/list/diff。
- Task 绑定 worktree。
- Context Pack 明确可写范围。
- cleanup fail closed。
- diff 生成 artifact。

验收：

- 每个实现任务有独立 branch/worktree。
- Warp 能读取 worktree path、branch、diff。
- 失败任务不会污染主工作区。

### Phase 4：Superpowers PolicyPack

目标：把流程纪律变成可执行 gate。

任务：

- `devflowPolicyPack/list/read/apply`。（已接入：`apply` 会按 task/risk 计算 plan/worktree/diff/verification/integrationTest/snapshot/review/rootCause/report 等 required artifacts；绑定已存在 task 时会写入可恢复的 policy application report artifact，并推送 `devflowArtifact/created`。）
- Planner 输出 plan artifact。（已接入：`devflowTask/plan` 为中高风险 implementation task 写入 planner report artifact，并挂到任务 artifact 列表。）
- bug/diagnostic 任务要求 root cause artifact。（已接入：`Diagnostic` task 和标题/目标像 bug 的 implementation task 会把 run summary/report 规范化为 `Root cause state` artifact，ReleasePrep 会阻断缺失、无状态或 `missing/unknown` 根因的任务。）
- review gate 必须处理 findings。（已接入：Codex Reviewer 的原生 `reviewOutput` 会随 v2 `exitedReviewMode` 透出，review artifact 优先消费这份语义结构并写入 finding state，包含 severity、filePath、line、status、resolution、followUp；没有原生结构时才回退到 Markdown/directive 解析。open findings 会阻断自动 Integrator merge 和 ReleasePrep；resolved/waived/follow-up 视为已处理。）
- completion gate 检查验证证据。（ReleasePrep 已接入：implementation task 必须有 passed/waived gate 且 gate artifact id 能解析到 artifact store。）

验收：

- 中高风险任务没有 plan 不能 start。（已接入：`devflowTask/start`/`devflowTask/dispatch` 缺 plan 时 fail-closed 为 `blocked`，不会创建 run/worktree/thread。）
- 没有 verification artifact 不能 Done。（ReleasePrep 已接入：缺少 verification artifact 的 implementation task 会阻断发布准备；implementation 运行态现在以 `ready_to_merge` 表示 review/gate 证据已闭环。）
- Review finding 必须 resolve/waive/follow-up。（已接入：review finding state 为 open 或缺少 finding-state metadata 时阻断发布准备；Codex 原生 `reviewOutput` 中的 priority/file/line range 会直接投影为结构化 severity/location，后续 UI 和回派不再需要只靠 Markdown 文本解析。）

### Phase 5：Quality Gate 和 review 闭环

目标：Codex 不只写代码，还能验证和修复。

任务：

- `devflowQualityGate/run/rerun/waive`。
- 自动发现 targeted test。
- gate output 写 artifact。
- Codex Reviewer 生成 review artifact。
- gate failed 时回派 Worker 修复至少一轮。

当前实现进展：

- `devflowQualityGate/run` 已支持显式 `kind`，第一批通用 gate（`format`、`lint`、`typecheck`、`targetedTest`、`integrationTest`、`snapshot`、`build`、`review`）会记录为对应类型；`rerun` 会沿用原 gate 的 command 和 kind。
- 未传 `commandOverride` 时，Rust 项目默认走受控 cargo 命令，非 Rust 项目优先使用存在的 package script，并以 `git diff --check` 作为轻量兜底；`gstack_*` gate 仍由 `devflowCapabilityPack/run` 生成。
- Review artifact 的 finding state 已升级为 schema v2：后端会从 Reviewer 的结构化 bullet 或 `::code-comment` 中提取 severity、filePath、line、status、resolution、followUp，并继续保留旧 summary 前缀供 ReleasePrep/Integrator fail-closed 判断。
- `integrationTest` / `snapshot` 已从“可选 gate kind”升级为策略证据：高风险 implementation task 在 ReleasePrep 前必须有 passed/waived integration-test gate artifact；标题/目标/触发源显示 UI、TUI、React、visual、snapshot、screenshot、dialog、form、settings 等快照敏感信号的 implementation task 必须有 passed/waived snapshot gate artifact。
- 自动 gate 链已接入执行闭环：implementation turn 完成后先跑 `targetedTest`，通过后按策略自动串行补跑 `integrationTest` / `snapshot`，所有必需 gate 均有 artifact 后才启动 Codex Reviewer；任一自动 gate 失败仍沿用 Worker 自动修复一轮。

验收：

- Gate 失败有命令、exit code、摘要和 artifact。
- Waive 必须产生 approval。
- Review 通过且 finding 已清空/resolve/waive/follow-up 后，implementation Task 进入显式 `ready_to_merge`；高风险和快照敏感任务缺少对应 gate artifact 时 ReleasePrep fail-closed。

### Phase 6：gstack CapabilityPack

目标：补齐工程 QA 和长任务监督能力。

任务：

- `devflowCapabilityPack/list/read/run`。
- 接入 gstack health。
- 接入 gstack browse / qa，保存截图或报告 artifact。
- 接入 gstack review。
- 接入 gstack watchdog queue。
- benchmark/canary 先作为可选 gate。

当前实现进展：

- `health`、`browseQa`、`review`、`watchdogQueue` 已接入 Codex-owned runner。
- `health`、`browseQa`、`review`、`benchmark`、`canary` 的 completed/failed 结果都会投影为 task-scoped quality gate，并把 report artifact 作为 gate evidence；`watchdogQueue` 保持只读 dashboard artifact，不伪造成验证 gate。
- `benchmark` 已先落成静态资产预算 runner：不启动 shell/browser/package script/network，只读取本地静态资产元数据并写 report artifact；预算失败会生成 `gstack_benchmark` quality gate 和 Watchdog alert。
- `canary` 已先落成本地安全探针 runner：自动选择 localhost/127.0.0.1 或静态 file target，不启动部署、不启动 package script、不打外网；浏览器 daemon 生命周期交给 gstack browse，goto/screenshot 失败、超时或截图缺失会生成 `gstack_canary` quality gate 和 Watchdog alert。

验收：

- 浏览器 QA 结果能进入 Artifact Timeline。
- health 结果能作为 QualityGate。
- watchdog alert 能推送给 Warp。

### Phase 7：Codex subagent 并行和 Integrator

目标：支持中型需求多任务并行。

任务：

- Planner 生成任务图。
- 每个 task 独立 worktree。
- Codex subagent 或独立 Codex run 并行。
- Integrator 合并无冲突 worktree。
- 冲突任务进入 blocked。

验收：

- 一个中型需求可拆成多个并行 task。
- 无冲突任务可自动合并。
- 有冲突时 Warp 能看到阻塞原因和相关 diff。

当前进度：

- `devflowTask/plan` 已生成可恢复 Planner DAG artifact：implementation workstream 分配给 `codex-worker`，fan-in review task 分配给 `codex-reviewer`，artifact 中明确 `codex-main`/`codex-worker`/`codex-reviewer`/`codex-integrator` 的 main-lane role map、nodes、edges 和 nextAction。
- `devflowTask/dispatch` 已批量启动 ready implementation workstream，并写入 Integrator dispatch report；dispatch 启动的 implementation task 现在会在 `ready_to_merge` 且 diff、quality gate、review artifact 都齐备后自动走 `devflowWorktree/merge` 同一条 Integrator 路径。
- implementation workstream 成功合并后，若所有 Planner DAG 依赖均满足，后续 project dispatch 会启动 fan-in `codex-reviewer` task，并把 Planner DAG artifact 与 dependency artifacts 注入 Review prompt；review task 完成后会写入自己的规范化 `ReviewReport` artifact，形成 Codex-owned 复审证据闭环。
- 有冲突时 Integrator 仍将 task 标记为 `blocked` 并写入 conflict report；primary worktree 修复后，显式 `devflowTask/dispatch` 指定该 task id 会启动 Codex conflict-repair run，先把 managed worktree 的 `baseCommit` 对齐到当前 primary HEAD，再写入 repair diff artifact，随后重新进入 quality gate、review 和 Integrator merge。
- `devflowWatchdog/reconcile` 已补上 bounded recovery action：它会从项目队列里找出带 Integrator conflict report 且依赖已解开的 blocked implementation task，再通过显式 conflict-repair dispatch 重启它们；`watchdogQueue` 仍然只负责只读投影。
- 普通 `devflowTask/start` 仍保留显式 merge 语义，避免单任务验证和 cleanup 场景被后台自动改动 primary worktree。

### Phase 8：发布准备和硬化

目标：形成可长期使用的本地自动化开发 backend。

任务：

- finish branch gate。
- commit message / PR body / release note artifact。
- support bundle 和 diagnostics。
- store 持久化和恢复。
- 大输出归档。
- 权限策略稳定。

当前实现进展：

- `devflowReleasePrep/create`、support bundle、大输出归档已接入 app-server v2；release prep 的 commit message / PR body / release note artifacts 以及长输出 `output_archive` 已纳入可重启恢复的 artifact store。
- `devflowReleasePrep/submit` 已补上发布执行层：它会先复用 release-prep gate，再在 `release_publish` 审批通过后执行 `git add` / `git commit`，并在 `commit_and_push` 模式下继续 `git push origin <currentBranch>`；`devflowReleasePrep/create` 仍然只做只读 gate，不直接 mutate Git。
- release prep 的 finish-branch gate 已接入 Integrator 证据和 Devflow store persistence 健康检查：有 managed worktree 的 implementation task 必须存在成功 merge report，当前 store snapshot load/persist error 会 fail-closed 阻止发布准备，PR body 会输出 Integrator、Persistence 和 finish-branch blockers，展示已合并/待合并任务以及发布前持久化健康状态。
- support bundle 已输出 release prep 复现入口、release prep artifact 元数据、Integrator merge evidence 摘要，以及 persistence health 区块；该区块会列出 snapshot 路径、snapshot 文件 metadata、load/persist error、可恢复索引和仍保持进程内语义的易失状态。带 task scope 的 bundle 也会登记为可恢复的 `report` artifact 并推送 `devflowArtifact/created`，方便把发布阻塞原因导出给 Warp 或在 app-server 重启后继续排障。
- task/run/quality gate/approval audit/artifact/watchdog runtime store 已快照到 `CODEX_HOME/devflow/store/state.json`，app-server 重启时会恢复这些索引。
- 如果 `CODEX_HOME/devflow/store/state.json` 损坏或不可读，app-server 会以空 Devflow store 启动，同时写入 critical `recovering` Watchdog alert，并通过 support bundle 导出 store snapshot load error，避免恢复失败变成静默丢状态。
- 如果后续 `state.json` best-effort 写入失败，app-server 会记住最近一次 store snapshot persist error，并将其作为非持久化 critical `recovering` Watchdog alert 投射到 read/list/queue/support bundle 视图，提示近期 task/run/gate/artifact 状态可能无法跨重启保留；后续快照写入恢复成功后，该错误会自动清除，避免 support bundle/UI 持续展示过期故障。
- `watchdogQueue` 已把 `recovering` 纳入 counts、queue item 和 dashboard dimension；全局恢复告警和 store snapshot persist failure 合成告警也会进入项目级 queue report，避免 dashboard 摘要漏掉启动恢复失败或持久化风险。
- 不能重连的 queued/running run 或 gate 采用 fail-closed 恢复语义：run/gate 标记 failed，仍在 running 的 task 标记 blocked，等待用户重新 dispatch/start/rerun。
- 重启时仍为 pending 的 approval 会恢复成 responded/cancelled 审计记录；approval grant、活跃 approval callback、活跃 thread subscription 仍保持进程内语义，不在重启后伪造安全授权。
- Devflow approval policy 已通过 `CODEX_HOME/devflow/approval-policy.json` 持久化，重启后继续按 task risk level 生效，并随 support bundle 导出用于排障；损坏或不可读的策略文件会 fail-closed，避免静默退回默认策略。

验收：

- 从需求到 PR 准备可以低人工干预完成。
- 所有关键动作有可恢复记录。
- 错误能被诊断、导出和复现。

## 7. 代码改造重点

优先修改：

- `codex-rs/app-server-protocol/src/protocol/v2/devflow.rs`
- `codex-rs/app-server/src/request_processors/devflow_processor.rs`
- `codex-rs/app-server/src/request_processors/devflow_project.rs`
- `codex-rs/app-server/src/request_processors/devflow_worktree.rs`
- `codex-rs/app-server/src/request_processors/devflow_quality_gate.rs`
- `codex-rs/app-server/src/request_processors/devflow_approval.rs`
- `codex-rs/app-server/tests/suite/v2/devflow.rs`
- `codex-rs/app-server/README.md`

建议新增：

- `codex-rs/app-server/src/request_processors/devflow_policy_pack.rs`
- `codex-rs/app-server/src/request_processors/devflow_capability_pack.rs`
- `codex-rs/app-server/src/request_processors/devflow_watchdog.rs`

如果 `devflow_processor.rs` 继续变大，应把 pack、watchdog、artifact、quality gate 的逻辑拆出去，避免形成新的巨型模块。

## 8. 测试要求

协议变更：

```sh
just write-app-server-schema
cargo test -p codex-app-server-protocol
```

app-server Devflow：

```sh
cargo test -p codex-app-server --test all devflow
```

具体功能建议拆成：

- devflow task create/read/list
- devflow task start
- devflow worktree create/diff/cleanup
- devflow approval requested/respond
- devflow quality gate run/rerun/waive
- devflow policy pack apply
- devflow capability pack run
- devflow watchdog alert

## 9. Codex 侧最终验收

Codex 方案完成后应满足：

- Warp 能连接 Codex app-server。
- Warp 能创建 Devflow task。
- Codex 能创建 managed worktree。
- Codex 能执行真实 coding turn。
- Codex 能返回 diff、run summary、test output、review report。
- Superpowers PolicyPack 能阻止无计划、无验证证据的高风险任务完成。
- gstack CapabilityPack 能产生 health、browser QA 或 watchdog artifact。
- 所有审批、gate、artifact、alert 都通过 app-server 事件流给 Warp。
- 主链路不依赖 Hermes 或 Claude Code。

## 10. 优先级结论

Codex 先做 backend 主干，不先追求全自动大型项目：

1. Codex-only Devflow 协议和 store。
2. 单任务 task/start/run/output/artifact。
3. worktree/diff/cleanup。
4. quality gate/review/approval。
5. Superpowers PolicyPack。
6. gstack CapabilityPack。
7. subagent 并行、graph dispatch 和 Integrator。
8. 发布准备、持久化、诊断和恢复。

第一阶段只要跑通：

```text
Warp -> Codex app-server -> Devflow task -> Codex run -> diff -> targeted test -> review -> artifact
```

整个自动化开发平台就有了稳定主干。
