# Yuqei Devflow Platform 完整方案

日期：2026-05-15

适用源码路径：

- Codex：`/Users/yuqei/codex-provider/codex-rs`
- Warp：`/Users/yuqei/warp`

可融合辅助源码路径：

- Superpowers：`/Users/yuqei/superpowers`
- gstack：`/Users/yuqei/gstack`

## 1. 总体目标

目标是把 Codex 和 Warp 改造成一个本地优先的软件自动化开发工作平台。平台只使用两层核心组件：

- Warp：用户面对的产品入口和开发工作台。
- Codex：主控制平面、主 agent runtime、工具执行层和 Devflow backend。

同时可以融合 Superpowers 和 gstack，但它们不作为新的主 runtime，也不改变 “Warp + Codex” 的主架构。它们应作为 Codex Devflow 的可配置增强包：

- Superpowers：流程策略包，负责把自动化开发约束到需求澄清、计划、worktree、TDD、系统化调试、review、验证和完成分支的可靠流程中。
- gstack：工程能力包，负责补强浏览器 QA、health、review、ship、benchmark、canary、watchdog 和长任务监督能力。

这个平台不是简单地让 Warp 调用一个 CLI，而是把 Codex 的 app-server、thread/turn、工具、权限、review、worktree、质量门和 artifact 能力产品化到 Warp 中，让用户可以在一个产品界面里完成：

- 需求输入
- 自动拆解任务
- Codex 内部 agent / subagent 并行开发
- worktree 隔离
- 自动测试和质量门禁
- 自动 review
- 自动 QA、浏览器验证、健康检查和发布前审计
- 合并、提交、推送、发布准备
- 会话、决策、日志、diff、artifact 的可恢复记录

理想状态是人工只参与少数关键决策：

- 需求目标确认
- 高风险操作审批
- 架构方向确认
- 合并和发布 gate
- 失败时的策略选择

其余重复性工程动作应由平台自动完成。

## 2. 本地组件的职责定位

### 2.1 Warp：产品入口和开发工作台

Warp 应作为最终用户主要面对的产品。

核心职责：

- 终端和工作区管理
- 项目列表和当前仓库上下文
- Codex 管理和启动入口
- 任务看板
- Codex turn、subagent、任务和质量门状态流展示
- worktree、branch、diff、测试结果、review 结果展示
- 权限审批 UI
- artifact、日志、历史任务、恢复入口
- 本地配置、模型配置、诊断页、安装引导

Warp 不应该承担复杂 agent 推理逻辑，也不应该直接实现所有开发流程。它应该调用统一控制层，并把状态以清晰、可恢复的方式展示给用户。

### 2.2 Codex：主控制平面和主 agent runtime

Codex 应作为平台的主 agent runtime 和 orchestration backend。

核心职责：

- 线程和回合管理
- agent turn 执行
- 工具调用
- 文件编辑
- shell 执行
- 权限和 sandbox
- review
- skills
- memory
- worktree 管理
- task/activity/agent graph API
- 自动质量门禁的命令执行
- 与 Warp 的 JSON-RPC 通信

Codex 当前已有 `app-server`，它天然适合作为 Warp 的本地 backend。后续新增平台能力时，应优先通过 `app-server v2` 暴露给 Warp，而不是让 Warp 直接依赖 Codex 内部模块。

### 2.3 Superpowers：流程策略包

Superpowers 不作为外部 agent runtime。它的价值在于把成熟的软件开发方法沉淀成 Codex 可执行的 Devflow 策略。

应融合的能力：

- brainstorming：在需求不清晰时先澄清目标、约束和成功标准。
- writing-plans：把需求转成可执行计划、文件范围、验证步骤和回滚点。
- using-git-worktrees：把实现任务默认放进独立 worktree。
- test-driven-development：对高风险核心逻辑启用红绿重构流程。
- systematic-debugging：遇到异常时先定位根因，再允许修改。
- subagent-driven-development：把大任务拆成独立 Codex worker，并配套 review。
- requesting-code-review / receiving-code-review：让 review 变成必须处理的质量门，而不是形式化报告。
- verification-before-completion：禁止没有新鲜验证证据就宣称完成。
- finishing-a-development-branch：统一收尾、测试、合并、提交和 PR 准备。

落地方式：

- 在 Codex Devflow 中定义 `PolicyPack`。
- 将 Superpowers 的流程规则转成 task policy、role prompt、quality gate 和 artifact 要求。
- Warp 只展示策略是否启用、当前卡在哪个流程阶段、下一步需要什么证据。
- 简单任务可以走轻量策略；中高风险任务默认启用完整策略。

### 2.4 gstack：工程能力包

gstack 不作为主控制层，也不替代 Codex app-server。它的价值在于提供自动化工程能力和长期任务监督模型。

应融合的能力：

- browse / qa：浏览器自动化、截图证据、响应式检查、前端交互验证。
- review：合并前结构性风险检查。
- health：聚合 typecheck、lint、test、dead code、shell lint 等项目健康分。
- benchmark：性能基线和回归检测。
- canary：发布后健康观察。
- cso：安全风险扫描和供应链检查。
- ship / land-and-deploy：提交、PR、发布前后的标准交付步骤。
- 4+6 orchestration / watchdog：队列、worker、审计、心跳、无进展检测和自动补位。

落地方式：

- 在 Codex Devflow 中定义 `CapabilityPack`。
- 将 gstack 的能力封装为 Codex 可调用的工具模板、quality gate 类型、QA artifact 和 watchdog 规则。
- 需要浏览器 daemon 时，由 Codex 统一启动、授权、记录和回收；Warp 不直接读写 gstack 状态文件。
- gstack 的 4+6 调度设计只作为 Devflow queue/watchdog 的参考，不引入 Hermes 或 Claude Code 作为必需依赖。

## 3. 核心设计原则

### 3.1 统一状态，不做双写状态

Warp 和 Codex 之间必须共享一个平台级任务状态模型。

不能让 Warp UI 和 Codex backend 各自维护一套互不一致的任务状态，否则自动化开发后会出现：

- 任务已经完成但 UI 不知道
- Codex 还在写旧 worktree
- review 的 diff 不是最终 diff
- 测试结果无法对应到具体 commit
- resume 时恢复到错误上下文

平台必须有统一的 `Project`、`Task`、`Run`、`AgentSession`、`Artifact`、`QualityGate` 状态。

### 3.2 控制层统一，能力层内聚到 Codex

Codex 是唯一的 agent runtime 和执行控制层。不同角色不再映射到不同外部程序，而是映射到 Codex 内部的能力配置、subagent、tools、approval policy、context pack 和 turn mode。

平台层只关心：

- Codex 是否可用
- 当前任务适合用哪种 Codex role / subagent
- 如何启动
- 如何传入上下文
- 如何接收流式输出
- 如何获得 diff、测试结果、最终结论
- 如何中断、恢复、重试

平台层不应再引入额外本地 agent runtime。需要方案、报告、review、测试分析、长期任务恢复时，都应优先通过 Codex 的 thread/turn、review、skills、memory 和 Devflow API 完成。

Superpowers 和 gstack 的融合也必须遵守这一条：只吸收策略、工具模板、质量门、浏览器验证和 watchdog 思路，不把它们提升为第二套任务状态或第二套 agent 控制面。

### 3.3 默认 worktree 隔离

所有自动实现任务都应默认使用独立 git worktree。

原因：

- Codex subagent 并行时减少写冲突
- 用户当前工作区不被污染
- 失败任务可以直接清理
- 每个任务的 diff 更容易 review
- 可以按任务独立测试、合并、回滚

只有非常小的只读任务、纯分析任务、或用户明确指定时，才允许直接在主工作区执行。

### 3.4 人工 gate 放在风险点

平台不应该每一步都问用户，否则自动化没有意义。

但这些动作必须进入人工审批或强策略审批：

- 删除大量文件
- 修改依赖锁文件
- 数据库迁移
- 影响认证、权限、支付、密钥、部署的代码
- force push
- 发布生产环境
- 写出 workspace 之外路径
- 执行不可逆 shell 命令
- 关闭测试或绕过质量门禁

低风险动作可以自动执行，例如读文件、创建 worktree、运行格式化、运行测试、生成 diff、写本任务目录内的代码。

### 3.5 先闭环，再扩大自治

不要一开始追求全自动大型项目开发。

正确路线是：

1. 单 agent 闭环
2. agent + reviewer 闭环
3. 多 worktree 并行
4. 自动合并和质量门
5. 受控发布
6. 长任务恢复和项目记忆

每阶段都必须有可验收的产品状态，而不是只堆功能。

### 3.6 策略包和能力包可插拔

Superpowers 和 gstack 应作为可启用、可禁用、可替换的 Devflow pack，而不是写死在 Warp 或 Codex 核心流程里。

原则：

- 策略包决定 “应该怎么做”，例如是否必须先写计划、是否必须 TDD、是否必须 review。
- 能力包提供 “可以调用什么”，例如浏览器 QA、health 检查、benchmark、canary、watchdog。
- Codex app-server 统一持久化 Project、Task、Run、Artifact、QualityGate 和事件流。
- Warp 只展示 pack 带来的流程节点、证据、告警和用户可操作项。
- pack 失败不能破坏主任务状态，必须以 QualityGate failure、Artifact 或 Alert 形式记录。

## 4. 推荐总体架构

```text
┌────────────────────────────────────────────────────────────┐
│                         Warp App                           │
│                                                            │
│  Project UI  Task Board  Codex Panel  Diff  Logs  Review   │
│  QA Evidence  Pack Status  Watchdog Alerts  Approval Center │
└───────────────────────────┬────────────────────────────────┘
                            │
                            │ JSON-RPC / WebSocket / Unix Socket
                            │
┌───────────────────────────▼────────────────────────────────┐
│              Codex app-server / Devflow API                │
│                                                            │
│  Project Manager     Task Graph       Approval Engine       │
│  Agent Registry      Worktree Manager Quality Gate Engine   │
│  Artifact Store      Memory Layer      Event Bus            │
│  Policy Packs        Capability Packs  Watchdog / Queue     │
└───────────────────────────┬────────────────────────────────┘
                            │
                            │ Codex thread / turn / review / tools
                            │
┌───────────────────────────▼────────────────────────────────┐
│                         Codex Runtime                      │
│                                                            │
│  Model Provider  Tools  MCP  Skills  Memory  Sandbox        │
│  Shell / Patch   Review  Subagents  Git / Worktree          │
│  Superpowers Policies   gstack Capabilities                 │
└───────────────────────────┬────────────────────────────────┘
                            │
┌──────────────────────────────▼──────────────────────────────┐
│                    Local Developer Machine                  │
│                                                            │
│  Git repos  Worktrees  Build tools  Tests  Browser  Logs    │
└────────────────────────────────────────────────────────────┘
```

## 5. 平台核心对象模型

### 5.1 Project

表示一个本地软件项目。

关键字段：

- `id`
- `name`
- `rootPath`
- `gitRemote`
- `defaultBranch`
- `language`
- `testCommands`
- `qualityGates`
- `trustedAgents`
- `workspacePolicy`

### 5.2 Agent

表示一个可调度 agent。

关键字段：

- `id`
- `name`
- `runtime`：固定为 `codex`
- `role`：`planner`、`worker`、`reviewer`、`qa`、`integrator`
- `launchCommand`
- `cwd`
- `model`
- `permissionProfile`
- `capabilities`
- `status`

### 5.3 Task

表示一个可执行开发任务。

关键字段：

- `id`
- `projectId`
- `title`
- `objective`
- `status`
- `parentTaskId`
- `dependencies`
- `assignedAgentId`
- `worktreePath`
- `branchName`
- `scope`
- `contextPackId`
- `riskLevel`
- `qualityGateIds`
- `artifacts`

### 5.4 Run

表示一次任务执行。

关键字段：

- `id`
- `taskId`
- `agentId`
- `startedAt`
- `completedAt`
- `status`
- `input`
- `streamEvents`
- `commands`
- `fileChanges`
- `tokenUsage`
- `exitReason`

### 5.5 Context Pack

表示发给 agent 的最小上下文包。

内容包括：

- 用户需求
- 任务目标
- 可编辑范围
- 禁止事项
- 相关文件
- 架构摘要
- 测试命令
- 质量门
- 输出格式要求

Context Pack 是 Codex 内部并行任务成败的核心。上下文给多了会浪费 token 和引入干扰；给少了会导致 Codex 猜测。

### 5.6 Artifact

表示任务产生的可追踪产物。

类型包括：

- plan
- patch
- diff
- command output
- test report
- review report
- screenshot
- build log
- release note
- rollback plan

### 5.7 Quality Gate

表示自动化质量检查。

类型包括：

- format
- lint
- typecheck
- unit test
- integration test
- snapshot test
- build
- security scan
- dependency check
- custom command

每个 gate 都必须记录：

- 命令
- cwd
- exit code
- stdout/stderr 摘要
- 开始时间
- 结束时间
- 关联 task/run/commit

### 5.8 Policy Pack

表示可应用到任务流的流程策略。

第一批内置策略来自 Superpowers：

- `brainstorming`
- `writingPlans`
- `worktreeIsolation`
- `tdd`
- `systematicDebugging`
- `subagentDrivenDevelopment`
- `requestCodeReview`
- `verificationBeforeCompletion`
- `finishBranch`

关键字段：

- `id`
- `name`
- `sourcePath`
- `enabled`
- `appliesToRiskLevels`
- `requiredArtifacts`
- `requiredGates`
- `roleInstructions`
- `waiverPolicy`

### 5.9 Capability Pack

表示可被 Devflow 调用的工程能力集合。

第一批内置能力来自 gstack：

- `browserQa`
- `healthCheck`
- `preLandingReview`
- `benchmark`
- `canary`
- `securityReview`
- `shipWorkflow`
- `watchdogQueue`

关键字段：

- `id`
- `name`
- `sourcePath`
- `enabled`
- `capabilities`
- `commands`
- `requiredPermissions`
- `artifactTypes`
- `eventTypes`
- `failurePolicy`

## 6. 通信协议设计

建议定义一层 `Yuqei Devflow Protocol`，由 Warp 调用 Codex app-server。Codex 负责协议实现、任务状态持久化、事件流、工具执行、review、worktree、质量门、策略包和能力包，不再通过额外本地 runtime adapter 转发。

### 6.1 Project API

```text
project/list
project/open
project/read
project/diagnose
project/trust
project/testCommands/list
```

### 6.2 Agent API

```text
agent/list
agent/detect
agent/read
agent/start
agent/stop
agent/restart
agent/diagnose
agent/capabilities
```

### 6.3 Task API

```text
task/create
task/plan
task/start
task/pause
task/resume
task/cancel
task/read
task/list
task/dependencies/update
task/assign
```

### 6.4 Worktree API

```text
worktree/create
worktree/read
worktree/list
worktree/diff
worktree/merge
worktree/cleanup
```

### 6.5 Quality Gate API

```text
qualityGate/list
qualityGate/run
qualityGate/read
qualityGate/rerun
qualityGate/waive
```

`qualityGate/waive` 必须需要明确审批，并记录原因。

### 6.6 Policy Pack API

```text
policyPack/list
policyPack/read
policyPack/enable
policyPack/disable
policyPack/apply
```

### 6.7 Capability Pack API

```text
capabilityPack/list
capabilityPack/read
capabilityPack/enable
capabilityPack/disable
capabilityPack/run
```

### 6.8 Approval API

```text
approval/request
approval/respond
approval/list
approval/policy/read
approval/policy/update
```

### 6.9 Artifact API

```text
artifact/list
artifact/read
artifact/open
artifact/export
```

### 6.10 Event Notifications

```text
task/statusChanged
agent/statusChanged
run/outputDelta
run/commandStarted
run/commandCompleted
file/changeDetected
worktree/diffUpdated
qualityGate/completed
approval/requested
artifact/created
policyPack/applied
capabilityPack/completed
watchdog/alerted
```

这些事件应直接驱动 Warp UI。

## 7. Agent 调度策略

### 7.1 推荐默认角色

第一阶段建议使用以下 agent 角色：

- `Planner`：拆任务、识别依赖、生成 Context Pack。
- `Worker`：在隔离 worktree 中实现具体任务。
- `Reviewer`：检查 diff、风险、测试、API 兼容性。
- `QA`：运行测试、复现失败、整理证据。
- `Integrator`：合并多个 worktree，处理冲突，生成最终提交。

### 7.2 Codex 内部角色分工建议

Codex 默认承担所有 agent 角色。不同职责通过 role、context pack、tool set、approval policy、model/reasoning 配置和 subagent 隔离来区分。

Codex Planner 默认承担：

- 需求整理
- 任务拆解
- 依赖识别
- 风险标记
- Context Pack 生成

Codex Worker 默认承担：

- 复杂代码实现
- 命令执行和文件编辑
- 局部测试和失败修复
- patch/diff artifact 生成

Codex Reviewer 默认承担：

- diff review
- API 兼容性检查
- 测试覆盖检查
- 安全风险检查
- 回派修复建议

Codex QA 默认承担：

- format / lint / typecheck / test / build
- 失败摘要
- 复现证据
- gate 历史记录

Codex Integrator 默认承担：

- 多 worktree 合并
- 冲突处理
- 最终质量门
- commit message / PR body / release note 草稿

### 7.3 并行规则

可以并行：

- 不同模块的实现任务
- 一个实现任务和一个只读分析任务
- 多个 review 任务
- 测试任务和文档更新任务

不建议并行：

- 多个 agent 修改同一个高耦合核心文件
- 多个 agent 同时修改依赖
- 多个 agent 同时改数据库 schema
- 多个 agent 同时改公共 API
- 多个 agent 同时更新同一批 snapshot

## 8. 标准自动化开发流

### 8.1 需求输入

用户在 Warp 输入：

```text
实现 XXX 功能，要求 YYY，完成后运行测试并生成 PR。
```

Warp 将需求发送给控制层。

### 8.2 计划生成

Planner 执行：

- 读取仓库结构
- 读取项目文档
- 识别相关模块
- 应用 Superpowers `brainstorming` / `writing-plans` 策略
- 生成任务图
- 标记风险
- 生成每个任务的可编辑范围
- 生成必须产出的 artifact 和验证证据
- 推荐测试命令

输出任务计划给 Warp。

### 8.3 人工确认

用户确认：

- 目标是否正确
- 是否允许创建 worktree
- 哪些 agent 可用
- 启用哪些 Superpowers policy
- 启用哪些 gstack capability
- 哪些高风险动作需要审批

### 8.4 任务分派

控制层为每个任务：

- 创建 branch
- 创建 worktree
- 生成 Context Pack
- 绑定 Superpowers 流程策略
- 绑定 gstack 质量门和 QA 能力
- 选择 agent
- 启动 Run

### 8.5 自动实现

Worker 在自己的 worktree 中：

- 按计划和可编辑范围执行
- 修改代码
- 运行局部测试
- 修复失败
- 输出实现说明
- 生成 patch/diff artifact
- 对高风险任务遵守 TDD 或系统化调试策略

### 8.6 自动 review

Reviewer 检查：

- 是否满足需求
- 是否破坏 API
- 是否有明显 bug
- 是否缺测试
- 是否违反项目规范
- 是否存在安全风险
- 是否有过度设计

如果失败，任务回派给 Worker。

Superpowers 的 requesting-code-review / receiving-code-review 策略应转成明确的 review gate：每条 review 反馈都要被验证、采纳、拒绝或转为后续任务，不能只在报告里停留。

### 8.7 质量门

QA 执行：

- format
- lint
- typecheck
- targeted tests
- integration tests
- snapshot tests
- build
- gstack health
- gstack browser QA
- gstack benchmark / canary（发布前后按需启用）

失败时自动生成失败摘要，并回派修复。

### 8.8 集成合并

Integrator 执行：

- 合并各 worktree diff
- 处理冲突
- 运行全局质量门
- 生成最终变更说明
- 生成 PR body 或 commit message

### 8.9 人工发布 gate

用户最终确认：

- 是否提交
- 是否推送
- 是否创建 PR
- 是否发布

平台记录完整证据链。

### 8.10 融合后的增强闭环

融合 Superpowers 和 gstack 后，标准闭环应升级为：

```text
Warp 输入需求
  -> Codex Devflow 创建 task
  -> Superpowers Policy Pack 约束计划、worktree、TDD、debug、review 和完成标准
  -> Codex Planner 生成任务图和 Context Pack
  -> Codex Worker / subagent 在 worktree 中实现
  -> Codex Reviewer 检查 diff 和风险
  -> gstack Capability Pack 运行 health、browser QA、benchmark 或 canary
  -> Codex Integrator 汇总 diff、gate、artifact 和最终说明
  -> Warp 展示证据链、审批点和 watchdog 告警
  -> 用户批准合并、提交、推送或发布
```

## 9. Warp 产品界面规划

### 9.1 Agent Management

显示：

- Codex 可用状态
- 启动路径
- 版本
- 模型
- 权限等级
- smoke test
- resume 状态
- Codex app-server 状态
- Codex 内部 subagent / task role 状态
- 最近错误

操作：

- 启动
- 停止
- 重启
- 打开源码目录
- 复制修复命令
- 运行诊断

### 9.2 Devflow Task Board

列：

- Planned
- Running
- Blocked
- Reviewing
- Testing
- Ready to Merge
- Done

每个 task card 显示：

- 任务目标
- agent
- worktree
- branch
- 状态
- 最近输出
- risk level
- gate 状态

### 9.3 Run Detail

显示：

- 流式 agent 输出
- 命令列表
- 文件变更
- 错误摘要
- 测试结果
- token/cost
- artifact 列表

### 9.4 Diff and Review

显示：

- 当前 task diff
- 所属 worktree
- reviewer 结论
- 风险项
- 测试覆盖
- 一键回派修复
- 一键接受

### 9.5 Approval Center

显示所有待审批动作：

- 命令
- 文件写入
- 依赖变更
- 删除操作
- 发布操作
- 越权路径访问

审批需要支持：

- 允许一次
- 拒绝
- 本任务内允许
- 本项目内允许某类低风险命令

### 9.6 Artifact Timeline

按时间显示：

- plan
- context pack
- agent run
- command
- diff
- test
- review
- merge
- commit
- deploy

这个 timeline 是长期任务恢复和 debug 自动化失败的关键。

### 9.7 Pack and Watchdog Status

显示：

- 当前项目启用的 Superpowers policies
- 当前项目启用的 gstack capabilities
- 每个 task 触发了哪些 policy / capability
- gstack browser QA 截图和失败证据
- health / benchmark / canary 结果
- watchdog 心跳、无进展告警、假运行状态和自动补位记录

操作：

- 启用或暂停某个 pack
- 对某个 gate 重新运行
- 查看失败 artifact
- 将 watchdog 告警回派给 Planner、Worker、Reviewer 或 QA

## 10. Codex 改造路线

### 10.1 优先使用 app-server v2

所有给 Warp 使用的新能力优先通过 app-server v2 暴露。

优先 API：

- thread/start
- turn/start
- review/start
- command/exec
- fs/readFile
- fs/writeFile
- skills/list
- provider/list
- task/start
- worktree/diff
- worktree/cleanup
- policyPack/list
- capabilityPack/list
- activity/list
- agent graph

### 10.2 避免继续膨胀 codex-core

新增平台能力优先放到更合适的 crate：

- task orchestration
- worktree management
- quality gates
- artifact store
- Codex task/session orchestration
- app-server protocol

只有真正属于 Codex 核心 turn loop 的能力才进入 `codex-core`。

### 10.3 新增 Devflow Adapter 层

建议新增或扩展：

- `codex-devflow-protocol`
- `codex-devflow-store`
- `codex-devflow-orchestrator`
- `codex-devflow-quality-gates`
- `codex-devflow-artifacts`
- `codex-devflow-policy-packs`
- `codex-devflow-capability-packs`
- `codex-devflow-watchdog`

职责：

- 统一任务图
- agent registry
- context pack 生成
- artifact 记录
- quality gate 执行
- policy pack 应用
- capability pack 调用
- watchdog / queue 状态维护
- event 转发

### 10.4 Codex 侧验收

Codex 侧最小验收：

- 能被 Warp 启动 app-server
- 能创建和恢复 thread
- 能执行一个真实 coding turn
- 能创建 worktree
- 能返回 diff
- 能运行 review
- 能运行 command gate
- 能应用 Superpowers policy pack
- 能调用 gstack capability pack
- 能记录 watchdog alert
- 能把事件流稳定传给 Warp

## 11. Warp 改造路线

### 11.1 核心产品化方向

Warp 要从“带 AI 能力的 terminal”升级成“本地 agent 开发工作台”。

新增主入口：

- Devflow
- Agent Management
- Task Board
- Policy Packs
- Capability Packs
- Approvals
- Artifacts
- Diagnostics

### 11.2 本地组件配置

默认路径：

- Codex：`/Users/yuqei/codex-provider/codex-rs`
- Warp：`/Users/yuqei/warp`
- Superpowers：`/Users/yuqei/superpowers`
- gstack：`/Users/yuqei/gstack`

环境覆盖：

```sh
export YUQEI_CODEX_ROOT=/Users/yuqei/codex-provider/codex-rs
export YUQEI_WARP_ROOT=/Users/yuqei/warp
export YUQEI_SUPERPOWERS_ROOT=/Users/yuqei/superpowers
export YUQEI_GSTACK_ROOT=/Users/yuqei/gstack
```

### 11.3 Warp 侧验收

Warp 侧最小验收：

- 能检测 Codex 和 Warp 本地配置
- 能检测 Superpowers 和 gstack 本地配置
- 能显示可用状态和错误
- 能启动 Codex app-server
- 能创建一个 Devflow task
- 能显示启用的 policy pack 和 capability pack
- 能显示 agent 流式输出
- 能显示 diff
- 能显示测试结果
- 能展示审批请求

## 12. 权限和安全模型

### 12.1 权限等级

建议定义四档：

- `readOnly`：只能读文件和运行只读命令
- `workspaceWrite`：只能写当前项目和对应 worktree
- `projectAutomation`：允许运行测试、格式化、构建、git add/commit
- `releaseControl`：允许 push、创建 PR、部署，但需要人工 gate

### 12.2 命令策略

默认允许：

- `rg`
- `find`
- `ls`
- `sed`
- `git status`
- `git diff`
- `cargo test`
- `npm test`
- `bun test`
- `just fmt`
- 项目配置中的测试命令

默认需要审批：

- `rm`
- `git reset`
- `git clean`
- `git push`
- `git commit`
- `cargo install`
- 修改全局配置
- 写出项目目录
- 启动长期后台服务

默认拒绝：

- 删除 home 目录
- 写系统目录
- 未授权读取密钥
- 未授权外发文件
- 绕过 quality gate 的发布动作

### 12.3 Secret 管理

所有密钥都应走 secret store 或 keychain。

agent 输出中必须过滤：

- API key
- token
- cookie
- SSH private key
- `.env` 内容
- 认证 header

## 13. 记忆和知识沉淀

平台需要三层记忆：

### 13.1 Project Memory

保存项目长期事实：

- 架构约定
- 测试命令
- 发布流程
- 常见错误
- 模块边界
- API 约束

### 13.2 Task Memory

保存任务执行记录：

- 为什么这样改
- 哪些方案被拒绝
- 哪些测试失败过
- 哪些文件是关键点
- 后续遗留问题

### 13.3 Agent Skill Memory

保存可复用流程：

- 如何修某类测试
- 如何更新 schema
- 如何跑 snapshot
- 如何发布
- 如何诊断 provider 问题

记忆必须可检查、可删除、可导出，不能成为不可见黑箱。

## 14. 最小可运行版本

MVP 只做一条完整闭环：

```text
Warp 输入需求
  -> Codex app-server 创建任务
  -> Codex 创建 worktree
  -> Codex 执行实现
  -> Codex 返回 diff
  -> Codex 运行 targeted test
  -> Codex review
  -> Warp 展示结果
  -> 用户批准合并或回派修复
```

MVP 不做：

- 复杂远程协作
- 云端账号系统
- 企业多租户
- 自动生产部署
- 大量插件 marketplace
- 全自动跨仓库发布

## 15. 分阶段路线图

### Phase 0：盘点和协议冻结

目标：确定边界和统一协议。

任务：

- 盘点 Codex app-server API
- 盘点 Warp 本地 Codex 集成能力
- 盘点 Codex thread/turn/review/tool/worktree/quality gate 能力
- 盘点 Superpowers 可转成 policy pack 的流程规则
- 盘点 gstack 可转成 capability pack 的工程能力
- 定义 Yuqei Devflow Protocol v0
- 定义 Project/Task/Run/Artifact schema
- 定义 PolicyPack / CapabilityPack schema
- 定义权限策略
- 定义默认测试命令发现规则

验收：

- 有一份协议文档
- Codex 和 Warp 本地配置都能被 detect
- Superpowers 和 gstack 本地路径能被 detect
- Warp 能显示基础诊断

### Phase 1：Codex + Warp 单任务闭环

目标：完成第一个真实自动开发闭环。

任务：

- Warp 连接 Codex app-server
- 创建 Devflow task
- 启动 Codex thread/turn
- 显示流式输出
- 显示命令和文件变更
- 返回 diff
- 运行 targeted test
- 保存 artifact

验收：

- 用户能在 Warp 里输入一个小需求
- Codex 完成代码修改
- Warp 展示 diff 和测试结果
- 用户可以接受或回派修复

### Phase 2：worktree 隔离和任务图

目标：让自动化开发不污染主工作区。

任务：

- Codex 创建 worktree
- Task Board 展示 worktree 状态
- 支持 worktree diff
- 支持 cleanup
- 支持任务依赖
- 支持失败重试

验收：

- 每个实现任务在独立 worktree 中完成
- 失败任务可清理
- 成功任务可合并

### Phase 3：Codex 内部角色和 review 闭环

目标：把方案、实现、review、QA 和集成都收敛到 Codex 内部角色，让 Warp 能清晰展示每个角色的产物和状态。

任务：

- 支持 Codex Planner 生成任务图和 Context Pack
- 支持 Codex Worker 执行实现型任务
- 支持 Codex Reviewer 检查 diff、风险和测试覆盖
- 支持 Codex QA 运行 gate 并生成失败摘要
- 支持 Codex Integrator 生成最终合并说明
- 支持 Superpowers policy pack 约束计划、worktree、TDD、debug、review 和 completion gate
- Warp 显示 planner/worker/reviewer/qa/integrator 的状态和 artifact

验收：

- 复杂代码实现、文字分析、review 报告和发布说明都由 Codex 完成
- Warp 能统一显示 Codex 各角色状态和输出 artifact
- 失败任务能回派给同一个 Codex task 的 worker 继续修复
- 没有验证证据的任务不能被标记为 Done

### Phase 4：Codex subagent 并行开发

目标：支持大型任务拆分并行执行。

任务：

- Planner 生成任务图
- 根据模块边界分配 Codex subagent 或独立 Codex run
- 多 worktree 并行
- 冲突检测
- 引入 gstack 4+6 orchestration 的 queue / watchdog / auditor 思路
- 自动集成
- Integrator 合并

验收：

- 一个中型需求可拆成多个并行任务
- 平台能自动合并无冲突任务
- 有冲突时能清楚展示给用户
- watchdog 能发现无进展、假运行和 worker 掉线
- Warp 能展示每个 Codex subagent 的 worktree、diff、测试和最终状态

### Phase 5：质量门和自动修复循环

目标：失败自动回派修复，并用 gstack 能力补齐浏览器 QA、健康检查和发布前后验证。

任务：

- 质量门配置
- 接入 gstack health
- 接入 gstack browse / qa
- 接入 gstack review
- 接入 gstack benchmark / canary
- 测试失败摘要
- 自动回派 worker
- reviewer 再检查
- gate 历史记录
- waiver 审批

验收：

- 测试失败后 agent 能自动修复至少一轮
- 所有 gate 都有记录
- 浏览器 QA、health、benchmark 或 canary 产出可追踪 artifact
- 人工可以看到失败原因和修复证据

### Phase 6：发布和产品化

目标：形成可长期使用的本地开发平台。

任务：

- PR/commit 生成
- changelog 生成
- release note
- rollback plan
- gstack ship / land-and-deploy 能力转成 Codex 交付 gate
- 日志导出
- 诊断包
- 配置导入导出
- 离线安装包
- 内网升级源

验收：

- 从需求到 PR 可以低人工干预完成
- 平台具备恢复、审计、诊断、交付能力
- Superpowers 和 gstack 都作为 pack 被 Codex Devflow 调用，而不是作为额外主 runtime 运行

## 16. 风险和处理策略

### 16.1 Codex 并行任务互相踩代码

处理：

- 默认 worktree 隔离
- scope 写入边界
- 任务图依赖
- 自动冲突检测

### 16.2 自动化修改质量不稳定

处理：

- Codex reviewer
- targeted test
- full gate
- 失败回派
- 高风险人工 gate

### 16.3 上下文过大导致 agent 失准

处理：

- Context Pack
- 代码索引
- 文件相关性排序
- 只给必要上下文

### 16.4 两个源码项目改造边界不清

处理：

- 先 Codex + Warp 闭环
- 先把 Devflow 控制面放在 Codex app-server
- Warp 只依赖 Codex 暴露的稳定协议
- 控制协议稳定后再产品化 UI

### 16.5 状态恢复困难

处理：

- 所有 Run 事件持久化
- artifact timeline
- thread/session 映射
- worktree 与 task 绑定
- 每个任务有清晰 exit reason

### 16.6 融合后边界漂移

处理：

- 保持 Warp + Codex 为唯一主架构
- Superpowers 只做 policy pack
- gstack 只做 capability pack
- 不把 Hermes 或 Claude Code 重新引入主链路
- 不让 Warp 直接读写 gstack 或 Superpowers 的内部状态作为事实来源
- 所有事实状态仍以 Codex Devflow store 为准

## 17. 建议立即执行的下一步

### 第一步：创建 Devflow v0 文档和 schema

在 Codex 仓库中新增：

```text
docs/yuqei-devflow-platform-plan.md
docs/yuqei-devflow-protocol-v0.md
```

### 第二步：做本地组件 detect

检测：

```text
/Users/yuqei/codex-provider/codex-rs
/Users/yuqei/warp
/Users/yuqei/superpowers
/Users/yuqei/gstack
```

输出：

- path exists
- manifest exists
- build command
- run command
- smoke command
- version
- last error
- pack type
- available policies / capabilities

### 第三步：定义 Superpowers 和 gstack 的 pack 映射

先做一份最小映射：

```text
Superpowers -> PolicyPack
gstack -> CapabilityPack
```

Superpowers 第一批启用：

- writing-plans
- using-git-worktrees
- systematic-debugging
- verification-before-completion
- requesting-code-review

gstack 第一批启用：

- health
- browse / qa
- review
- watchdog queue

### 第四步：Warp 连接 Codex app-server

先跑最小链路：

```text
Warp -> Codex app-server initialize -> thread/start -> turn/start -> events
```

### 第五步：实现单任务闭环

选一个小需求，完成：

```text
需求 -> task -> policy pack -> codex run -> diff -> test -> review -> artifact
```

### 第六步：接入 Codex 内部角色

当 Codex + Warp 单任务闭环稳定后，把 Codex 内部角色产品化：

- Planner 负责任务图和 Context Pack
- Worker 负责代码实现
- Reviewer 负责 diff、风险和测试覆盖
- QA 负责质量门和失败摘要
- Integrator 负责合并说明和最终 artifact

### 第七步：接入 gstack 工程能力

当基础角色闭环稳定后，优先接入：

- health gate
- browser QA evidence
- review gate
- watchdog alert

### 第八步：接入 Codex subagent 并行

当 Codex 内部角色闭环稳定后，把 Codex subagent 并行开发加入 Devflow：

- 按模块拆分任务
- 每个任务独立 worktree
- 并行执行、并行测试和冲突检测
- Integrator 汇总 diff、gate 和最终说明

## 18. 最终成功状态

最终产品形态应是：

- 用户打开 Warp
- 选择本地项目
- 输入开发目标
- 平台自动拆任务
- Superpowers 约束计划、worktree、调试、review 和完成标准
- Codex 负责方案、实现、测试、review、报告和集成
- gstack 提供浏览器 QA、health、watchdog、benchmark 和 canary 能力
- Warp 负责展示任务、状态、diff、artifact、审批和恢复入口
- 每个任务有隔离 worktree
- 每个变更有 diff、测试、review、日志
- 失败能自动修复或清晰交给用户
- 成功后生成 PR/commit/release note
- 用户只审批高风险动作和最终发布

这才是真正适合大型软件项目的 Warp + Codex 低人工干预开发模式。
