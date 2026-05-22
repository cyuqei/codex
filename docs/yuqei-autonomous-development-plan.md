# Codex 全自动化开发执行方案

> 目标：让 Claude 在用户启动时给予高权限后，以尽可能自动化、高效率、可恢复、可审计的方式长期推进 `/Users/yuqei/codex` 升级改造项目，直到阶段性完成。本文不是让 agent 无边界乱改，而是定义“自动执行 + 安全闸门 + 证据验收”的工程系统。

## 1. 结论

可以做到高度自动化，但不建议做“完全无人值守直到全部项目完成”。

正确模式是：

```text
高权限启动
  -> 自动读取任务队列
  -> 自动执行一个阶段/一个任务包
  -> 自动测试和记录
  -> 自动保存上下文
  -> 遇到高风险/产品方向/外部副作用时停下来让用户确认
  -> 确认后继续下一批任务
```

也就是：

```text
自动驾驶，不是无刹车。
```

## 2. 启动方式

如果用户愿意在本机给最大权限，可用：

```bash
claude --dangerously-skip-permissions
```

如果是运行本地 cc-haha 包装命令，则可能是：

```bash
claude-haha --dangerously-skip-permissions
```

具体以当前 CLI 支持的参数为准。

注意：

- `--dangerously-skip-permissions` 是启动时权限模式。
- `/permissions` 主要管理工具 allow/deny 规则，不一定能切到这个模式。
- `skipDangerousModePermissionPrompt` 只是不再二次确认危险模式，不等于自动开启危险模式。

## 3. 自动化模式的边界

### 3.1 可以自动做的事

以下适合自动执行：

- 读取代码和文档
- 创建/更新项目内部方案文档
- 拆分任务
- 改代码
- 增加测试
- 运行格式化、lint、unit test、cargo check
- 记录失败原因
- 根据失败自动修复
- 更新本地 dev log
- 保存 context
- 创建本地 checkpoint commit，前提是用户允许
- 生成下一批任务计划

### 3.2 不应该无确认自动做的事

即使在最大权限启动下，以下仍要人工确认：

- `git push`
- 创建/关闭/评论 PR 或 issue
- 发布 release
- 部署生产环境
- 删除大量文件
- `git reset --hard`
- `git clean -fd`
- force push
- 修改用户全局系统配置
- 写入真实 API key
- 调用付费高成本模型跑大规模任务
- 连接外部账户或远程服务
- 任何不可逆或影响共享系统的操作

## 4. 推荐的全自动执行架构

建议在 Codex 项目内建立一个本地自动化控制层：

```text
docs/
  yuqei-codex-long-term-roadmap.md
  yuqei-codex-architecture-notes.md
  yuqei-provider-compatibility.md
  yuqei-provider-test-results.md
  yuqei-autonomous-development-plan.md

automation/
  queue/
    000-roadmap.yaml
    010-provider-ui.yaml
    020-provider-test.yaml
  scripts/
    run-next-task.sh
    run-task-batch.sh
    verify-task.sh
    update-dev-log.sh
  reports/
    task-runs/
    test-runs/
    checkpoints/
```

其中：

- `docs/` 保存长期方案和决策。
- `automation/queue/` 保存可执行任务队列。
- `automation/scripts/` 保存自动执行脚本。
- `automation/reports/` 保存每次运行证据。

是否实际创建 `automation/` 目录，应作为下一阶段实现任务，不在本文档里直接假设已存在。

## 5. 任务队列格式

建议每个任务使用 YAML 或 Markdown frontmatter。

示例：

```yaml
id: provider-ui-001
title: Add provider management backend APIs
phase: provider-ui
status: pending
priority: P0
risk: medium
owner: claude
requires_human_approval: false
files_expected:
  - codex-rs/app-server/src/request_processors/config.rs
  - codex-rs/app-server-protocol/src/protocol/v2/config.rs
validation:
  - cargo check -p codex-app-server
  - cargo test -p codex-app-server config_manager
exit_criteria:
  - UI/app-server can list configured model providers
  - API returns provider id, display name, base_url, auth style, wire_api, enabled status
notes:
  - Do not expose API key values in responses
```

字段含义：

| 字段 | 含义 |
|---|---|
| id | 稳定任务 ID |
| title | 人类可读标题 |
| phase | 所属阶段 |
| status | pending / in_progress / blocked / done |
| priority | P0/P1/P2 |
| risk | low/medium/high |
| requires_human_approval | 是否必须人工确认 |
| files_expected | 预计修改文件 |
| validation | 验证命令 |
| exit_criteria | 完成标准 |
| notes | 约束和风险 |

## 6. 自动化执行循环

每次启动后执行如下循环：

```text
1. 读取 docs/yuqei-autonomous-development-plan.md
2. 读取 docs/yuqei-codex-long-term-roadmap.md
3. 读取 automation/queue 中第一个 pending 任务
4. 检查 git 状态
5. 如果有用户未保存改动，先记录并避免覆盖
6. 将任务标记为 in_progress
7. 读取相关源码
8. 制定微计划
9. 修改代码
10. 运行 validation
11. 如果失败，自动修复，最多 3 轮
12. 仍失败则标记 blocked，写原因
13. 成功则更新文档和 dev log
14. 可选创建 checkpoint commit
15. 标记任务 done
16. 进入下一个任务
```

伪代码：

```bash
while true; do
  task=$(next_pending_task)
  [ -z "$task" ] && break

  prepare_workspace "$task"
  implement_task "$task"
  verify_task "$task"

  if verified; then
    update_docs "$task"
    checkpoint_if_enabled "$task"
    mark_done "$task"
  else
    mark_blocked "$task"
    save_failure_report "$task"
    break
  fi

done
```

## 7. 自动化运行的推荐命令接口

未来可以做这些命令：

```text
codex-auto status
codex-auto next
codex-auto run-next
codex-auto run-phase provider-ui
codex-auto run-until-blocked
codex-auto verify
codex-auto report
codex-auto checkpoint
codex-auto resume
```

其中最重要的是：

```bash
codex-auto run-until-blocked
```

含义：持续执行任务，直到：

- 没有任务
- 测试连续失败无法修复
- 需要人工决策
- 触发高风险操作
- 上下文不足
- 成本过高

## 8. 当前 Codex 改造项目的自动化阶段

### Phase A：项目控制面

目标：先建立自动化项目管理基础。

任务：

1. 建立 `automation/queue` 任务队列
2. 建立 dev log
3. 建立 test result 文档更新规则
4. 建立 checkpoint 策略
5. 建立 blocked task 格式
6. 建立人工审批规则

完成标准：

- 有可读任务队列
- 有明确状态流转
- 有验证结果沉淀

### Phase B：Provider 兼容性实测

目标：确定哪些模型供应商可直接用，哪些需要源码 adapter。

任务：

1. 实测 OpenAI
2. 实测 Anthropic-compatible
3. 实测 OpenRouter
4. 实测 DeepSeek
5. 更新 `docs/yuqei-provider-test-results.md`
6. 决定是否需要 ChatCompletions adapter

完成标准：

- 每个 provider 有 PASS/PARTIAL/FAIL 结论
- 有明确 adapter 决策

### Phase C：Provider 管理后端

目标：让 app-server 能读写 provider 配置，为 UI-first 做后端。

任务：

1. 设计 provider list/read/write/test protocol
2. 在 app-server protocol 中加入 provider config API
3. 在 app-server request processor 中实现
4. 支持隐藏 API key
5. 支持测试连接
6. 加测试

完成标准：

- UI 或 client 可通过 app-server 管理 provider
- API key 不泄露
- 测试连接返回结构化诊断

### Phase D：Provider 管理 UI

目标：普通用户能在界面里配置模型接口。

任务：

1. Provider 列表页
2. 新增 provider 向导
3. 编辑 provider 表单
4. API key 安全输入
5. base_url 校验
6. wire_api 选择
7. headers 高级设置
8. 测试连接按钮
9. 默认 provider/model 选择
10. 项目级覆盖

完成标准：

- 不手改配置文件也能完成主要 provider 配置
- 错误诊断清晰

### Phase E：Model Catalog 和模型选择

目标：模型不再只是字符串。

任务：

1. 支持静态 model catalog
2. 支持 provider 拉取 model list
3. 支持模型能力声明
4. TUI/UI 显示模型能力
5. 支持默认模型、强模型、快模型分层

完成标准：

- 用户能在 UI 选择模型
- agent 能根据模型能力决定工具策略

### Phase F：ChatCompletions adapter，条件执行

仅当 Phase B 证明 OpenRouter/DeepSeek/本地 provider 不能通过 Responses/AnthropicMessages 跑通时执行。

任务：

1. 新增 `WireApi::ChatCompletions`
2. 新增 API client
3. 完成 message 转换
4. 完成 streaming delta 转换
5. 完成 tool call 转换
6. 加 provider compatibility tests

完成标准：

- OpenRouter/DeepSeek 至少一个通过工具调用测试
- 不破坏现有 Responses/AnthropicMessages

### Phase G：自动化 workflow/skills

目标：把日常开发工作流产品化。

任务：

1. investigate workflow
2. review workflow
3. commit workflow
4. context save/restore
5. QA workflow
6. provider setup workflow

完成标准：

- 用户能通过命令或 UI 启动工作流
- 每个 workflow 有可测试状态机

## 9. 自动化脚本层设计

### 9.1 run-next-task.sh

职责：

- 找到下一个 pending task
- 输出任务 JSON/YAML
- 不直接改代码

### 9.2 run-task-batch.sh

职责：

- 批量执行同一 phase 的任务
- 每个任务结束后运行验证
- 遇到 blocked 停止

### 9.3 verify-task.sh

职责：

- 根据任务中的 validation 执行验证
- 保存 stdout/stderr
- 输出结构化结果

### 9.4 update-dev-log.sh

职责：

- 写入 `docs/yuqei-codex-dev-log.md`
- 记录任务、改动、测试、失败、决策

### 9.5 checkpoint.sh

职责：

- 检查 git status
- 只 stage 本任务相关文件
- 创建 WIP checkpoint commit
- 不 push

## 10. 人工审批节点

即使全自动模式，也必须设置审批节点。

### 10.1 必须审批

- 选定产品方向
- 是否新增 ChatCompletions adapter
- 是否引入新依赖
- 是否创建大规模 UI 子项目
- 是否删除旧架构
- 是否推送远端
- 是否发布版本
- 是否写入真实密钥

### 10.2 可自动决策

- 小范围重构
- 测试补充
- 文档更新
- 错误信息优化
- 类型修复
- lint/format 修复
- 局部 API 命名一致化

### 10.3 审批格式

每个审批请求应包含：

```text
Decision: 是否执行 X
Why now: 为什么当前需要
Options: A/B/C
Recommendation: 推荐选项
Risk: 失败影响
Rollback: 如何回滚
```

## 11. 安全策略

### 11.1 权限分层

```text
Level 0: read-only 自动调研
Level 1: docs-only 自动写文档
Level 2: code-write 自动改代码
Level 3: test-run 自动运行测试
Level 4: checkpoint 自动本地提交
Level 5: external-side-effect 需要人工确认
```

### 11.2 命令策略

自动允许：

```text
git status
git diff
git log
cargo check
cargo test
cargo fmt
npm test
pnpm test
bun test
```

需要确认：

```text
git commit
npm install
pnpm install
cargo update
brew install
```

必须人工确认：

```text
git push
git reset --hard
git clean -fd
rm -rf
force push
release/deploy
```

### 11.3 密钥策略

- 不把 API key 写入文档
- 不把 API key 写入 git
- UI 输入的密钥优先进入 keyring 或本地安全存储
- 文档只记录环境变量名
- 自动测试发现密钥缺失时标记 BLOCKED，不猜测、不生成

## 12. 上下文保存策略

每个任务完成后写：

```text
docs/yuqei-codex-dev-log.md
```

内容：

```markdown
## YYYY-MM-DD task-id

- Task:
- Files changed:
- Tests run:
- Result:
- Decisions:
- Blockers:
- Next:
```

长会话结束前执行 context-save 或写 checkpoint：

```text
当前完成到哪个 phase
哪些任务 done
哪些任务 blocked
下一步从哪里恢复
```

## 13. 自动化验收标准

每个任务完成必须满足：

1. 代码能编译或明确说明为什么暂不能编译
2. 相关测试通过或失败原因已记录
3. 文档已更新
4. 没有泄露密钥
5. 没有未解释的大范围 diff
6. 有 rollback 思路
7. 任务状态已更新

每个 phase 完成必须满足：

1. phase summary 已写入文档
2. 所有任务 done 或明确 deferred
3. 至少一条验证证据
4. 下一 phase 的入口任务明确

## 14. 推荐的实际工作流

### 14.1 每次启动

用户启动：

```bash
claude --dangerously-skip-permissions
```

用户输入：

```text
继续 Codex 自动化开发，按 docs/yuqei-autonomous-development-plan.md 执行，run-until-blocked。
```

Claude 执行：

1. 读自动化方案
2. 读 dev log
3. 读任务队列
4. 执行下一个任务包
5. 测试
6. 记录
7. 遇到 blocker 才问用户

### 14.2 每个阶段开始

Claude 输出：

```text
开始 Phase C: Provider 管理后端。
本阶段目标：app-server 能读写 provider 配置。
预计修改：app-server-protocol、app-server request processor、config manager tests。
高风险点：API key 不得泄露。
```

### 14.3 每个阶段结束

Claude 输出：

```text
Phase C 完成。
验证：cargo test -p codex-app-server provider_config 通过。
文档：已更新 dev log。
下一步：Phase D Provider 管理 UI。
```

## 15. 为什么不能完全无人值守到全部完成

原因：

1. 这是长期产品，不是单个 bug fix。
2. 中途会出现产品取舍，例如 UI-first 到底先做 TUI 还是 Web UI。
3. 会遇到外部 provider 兼容性差异，需要真实 API key 和成本控制。
4. 可能需要新增 adapter，这是架构决策。
5. 大规模自动修改如果没有 checkpoint，失败后难恢复。

所以最优方案是：

```text
自动执行到 blocked
人工只处理关键决策
然后继续自动执行
```

## 16. 近期落地建议

下一步不是马上让 Claude “自己一直写”，而是先建立自动执行骨架：

1. 创建 `docs/yuqei-codex-dev-log.md`
2. 创建 `automation/queue/000-provider-phase.yaml`
3. 创建 `automation/reports/`
4. 写第一批任务：Provider 兼容性实测
5. 跑第一轮自动测试
6. 根据结果决定 Phase C 或 ChatCompletions adapter

## 17. 第一批任务建议

```text
A001: 填写 provider test results 环境信息
A002: 检查当前 Codex 构建/测试命令
A003: 实测 OpenAI provider
A004: 实测 Anthropic-compatible，如有 key/base_url
A005: 实测 OpenRouter，如有 key
A006: 实测 DeepSeek，如有 key
A007: 汇总 adapter 决策
A008: 设计 provider UI-first 后端 API
```

## 18. 最终推荐

使用三层自动化：

```text
Layer 1: Claude 自动执行任务
Layer 2: 本地脚本管理队列、验证、报告
Layer 3: 人工审批关键产品/安全/外部副作用节点
```

不要追求“永远不问用户”。

应该追求：

```text
99% 编码、测试、文档自动化；
1% 产品方向、安全风险、外部副作用人工决策。
```

这才适合一个未来要产品化的长期 AI 开发工具项目。
