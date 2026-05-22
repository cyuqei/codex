# Codex 长期改造路线图

> 目标：以 `/Users/yuqei/codex` 为长期主线，把 Codex 改造成自己的 AI 开发工具内核；以 `/Users/yuqei/cc-haha` 为功能和产品体验参考。

## 一、总目标

把 `/Users/yuqei/codex` 从 OpenAI Codex CLI，逐步改造成一个：

1. 可长期维护的个人 AI 开发工具
2. 支持多模型、多供应商的 coding agent
3. 有 CLI、TUI、桌面端、远程控制等多入口
4. 支持 memory、skills、MCP、multi-agent、插件化扩展
5. 未来可抽象为通用 AI Agent 应用平台

最终形态可以是：个人 AI 开发操作系统 / AI Coding Workbench / Agent Runtime Platform。

## 二、核心原则

### 1. Codex 做内核，cc-haha 做参考

不要直接把 cc-haha 当长期主干。cc-haha 有来源和授权风险，不适合长期商业化或公开产品化。

正确方式：

- Codex：主代码库
- cc-haha：参考设计、功能、交互、文档
- 自己重写关键功能
- 不复制敏感来源代码

### 2. 先打通日常使用，再做大平台

优先级：

1. 自己每天能用
2. 用起来比官方 Codex 更顺手
3. 能替代 Claude Code / Codex CLI 的大部分日常场景
4. 再做桌面端、远程控制、多 agent、插件市场

### 3. 所有长期能力都要插件化

长期功能应逐步形成：

```text
core runtime
  ├─ model providers
  ├─ tools
  ├─ skills
  ├─ memory backends
  ├─ UI adapters
  ├─ remote adapters
  └─ workflow plugins
```

## 三、总体架构目标

```text
AI Dev System
│
├─ Core Agent Runtime
│  ├─ conversation loop
│  ├─ tool execution
│  ├─ permission system
│  ├─ sandbox
│  ├─ context manager
│  ├─ model router
│  └─ event bus
│
├─ Model Layer
│  ├─ OpenAI
│  ├─ Anthropic-compatible
│  ├─ OpenRouter
│  ├─ DeepSeek
│  ├─ Gemini
│  └─ local models
│
├─ Tool Layer
│  ├─ file tools
│  ├─ shell tools
│  ├─ git tools
│  ├─ browser tools
│  ├─ computer-use tools
│  ├─ MCP tools
│  └─ custom tools
│
├─ Memory Layer
│  ├─ user memory
│  ├─ project memory
│  ├─ task memory
│  ├─ vector search
│  └─ session restore
│
├─ Workflow Layer
│  ├─ skills
│  ├─ slash commands
│  ├─ QA workflow
│  ├─ review workflow
│  ├─ ship workflow
│  └─ custom automation
│
├─ Interface Layer
│  ├─ CLI
│  ├─ TUI
│  ├─ desktop app
│  ├─ web app
│  ├─ VS Code / JetBrains extension
│  └─ mobile / IM control
│
└─ Platform Layer
   ├─ config
   ├─ plugin registry
   ├─ telemetry
   ├─ auth
   ├─ update system
   └─ deployment packaging
```

## 四、阶段路线图

### Phase 0：熟悉和冻结基线

目标：先理解 Codex，不急着大改。

要做的事：

1. 跑通 Codex 本地开发环境
2. 跑通测试
3. 找到核心入口
4. 找到模型调用链路
5. 找到工具调用链路
6. 找到 TUI/CLI 入口
7. 建立自己的开发分支
8. 建立改造文档

重点目录：

```text
codex-rs/core
codex-rs/tui
codex-rs/cli
codex-rs/exec
codex-rs/model-provider
codex-rs/mcp-server
codex-rs/skills
codex-rs/plugin
codex-rs/sandboxing
codex-rs/thread-store
codex-rs/app-server
codex-rs/app-server-protocol
```

建议产出：

```text
docs/yuqei-codex-long-term-roadmap.md
docs/yuqei-codex-architecture-notes.md
docs/yuqei-codex-dev-log.md
```

### Phase 1：模型供应商改造

目标：让 Codex 不只绑定 OpenAI，而是变成多模型 coding agent。

优先供应商：

1. OpenAI 官方
2. OpenRouter
3. Anthropic-compatible API
4. DeepSeek
5. Gemini
6. 自定义 base_url + api_key

理想配置形式：

```toml
[model]
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4.5"

[providers.openai]
kind = "openai"
api_key_env = "OPENAI_API_KEY"

[providers.openrouter]
kind = "openai-compatible"
base_url = "https://openrouter.ai/api/v1"
api_key_env = "OPENROUTER_API_KEY"

[providers.deepseek]
kind = "openai-compatible"
base_url = "https://api.deepseek.com/v1"
api_key_env = "DEEPSEEK_API_KEY"

[providers.ollama]
kind = "ollama"
base_url = "http://localhost:11434"

[providers.lmstudio]
kind = "openai-compatible"
base_url = "http://localhost:1234/v1"
```

要实现的能力：

- provider registry
- model registry
- 默认模型配置
- 每个项目单独模型配置
- 每次会话临时切换模型
- slash command 切模型
- 自动识别模型能力：tool calling、reasoning、vision、long context、structured output
- fallback model
- cheap model / strong model 分层

产品化要求：

- 大模型接口配置必须 UI-first，不应要求普通用户手改配置文件。
- 配置文件只作为高级用户/调试/导入导出机制保留。
- UI 中需要支持 provider 新增、编辑、删除、启用、测试连接、选择默认模型、项目级覆盖。
- API key/base URL/header/auth style/wire API/model list 等都应有表单和校验。
- 用户保存前应能点击“测试连接”，明确看到成功、鉴权失败、endpoint 不兼容、模型不存在等诊断结果。
- 未来桌面端/Web UI 的设置页应把“模型供应商管理”作为核心一级功能。

长期模型路由：

```text
simple edit        -> cheap fast model
large refactor     -> strong coding model
planning           -> reasoning model
code review        -> strong model
summarization      -> cheap model
embedding/search   -> embedding model
```

### Phase 2：配置系统和中文化体验

目标：把 Codex 改成长期舒服使用的工具。

配置层：

```text
global config
project config
session config
environment variables
runtime overrides
```

建议：

```text
~/.your-codex/config.toml
项目/.codex/config.toml
```

配置内容：

- 默认模型
- provider
- 默认 permission mode
- 默认 sandbox 模式
- 是否允许网络
- 是否自动读取项目说明
- memory 开关
- language preference
- output style
- tools allowlist
- tools denylist
- hooks
- skills
- UI 主题

中文体验：

```toml
[ui]
language = "zh-CN"
response_language = "zh-CN"
code_comments_language = "en"
```

支持中文帮助、错误提示、onboarding、模型配置向导、slash commands 别名。

### Phase 3：日常开发工作流增强

目标：让它成为每天写项目的主工具。

必备工作流：

1. 计划模式：先读代码、输出方案、用户确认、再改代码
2. 调查 bug 模式：先复现、找根因、再修复、再测试
3. 代码审查模式：看 diff，找安全、架构、测试问题
4. 自动测试模式：识别项目测试命令，运行相关测试，失败后定位
5. 提交/PR 模式：看 git status/diff，生成 commit message 和 PR 描述

可参考 cc-haha/gstack 的 workflow 思路：investigate、review、ship、qa、design-review、context-save、context-restore。

### Phase 4：Memory 系统

目标：让工具长期理解用户、项目和工作习惯。

Memory 类型：

```text
user memory
project memory
feedback memory
reference memory
```

第一版文件系统：

```text
~/.your-codex/memory/
  MEMORY.md
  user/
  project/
  feedback/
  reference/
```

项目级：

```text
项目/.codex/memory/
```

第二版可加 SQLite、LanceDB、Qdrant、Tantivy、pgvector。

调用原则：用户说“记住”才必须保存；用户说“别记”不能保存；推荐前验证当前代码；memory 不存可从代码读取的结构。

### Phase 5：Skills / Workflow 插件系统

每个 skill 可以是一个目录：

```text
skills/
  investigate/
    skill.toml
    prompt.md
    hooks.toml
    scripts/
  review/
    skill.toml
    prompt.md
  ship/
    skill.toml
    prompt.md
```

skill.toml 示例：

```toml
name = "investigate"
description = "Systematic debugging workflow"
triggers = ["bug", "error", "debug", "why broken", "修 bug", "报错"]

[permissions]
allow_bash = true
allow_edit = true
require_plan = false

[workflow]
steps = [
  "reproduce",
  "inspect",
  "hypothesize",
  "fix",
  "verify"
]
```

第一批 skills：investigate、review、commit、ship、context-save、context-restore、qa-only、qa-fix、docs-update、refactor。

### Phase 6：工具系统增强

基础工具：read file、edit file、write file、search file、grep content、bash、git、task list、ask user、plan mode。

高级工具：browser automation、screenshot reading、desktop computer use、database tool、API testing tool、log viewer、profiler、dependency analyzer、secret scanner、architecture graph、repo indexer。

工具权限分层：safe read-only tools、local reversible tools、risky local tools、external side-effect tools、destructive tools。

### Phase 7：权限、安全和沙箱

权限等级：

```text
read-only
workspace-write
full-access
approval-required
dangerous-confirm
```

命令分类：

```text
safe:
  - git status
  - git diff
  - cargo check
  - npm test

needs approval:
  - git commit
  - git push
  - npm install
  - cargo update

dangerous:
  - rm -rf
  - git reset --hard
  - git clean -fd
  - force push
  - drop database
  - kubectl delete
```

长期目标：文件系统沙箱、网络沙箱、shell 策略、secret 防泄漏、prompt injection 防护、MCP server 信任等级、plugin 签名/权限、操作审计日志。

### Phase 8：App Server 和桌面端

Codex 已有：

```text
codex-rs/app-server
codex-rs/app-server-protocol
codex-rs/app-server-client
```

第一版桌面端目标：项目列表、会话列表、聊天界面、工具调用展示、diff 展示、权限确认弹窗、模型选择、设置页面、运行状态、terminal output 展示。

建议路线：先做 Web UI + local app-server，稳定后再 Tauri。

### Phase 9：远程控制

优先级：Web dashboard、Telegram bot、飞书 bot、微信/企业微信、钉钉、Mobile PWA。

远程控制必须支持：查看项目、查看会话、发送 prompt、接收回复、审批权限、查看 diff、允许/拒绝 shell、停止任务、切换模型、查看状态。

安全要求：登录、device pairing、token、审计、权限审批、项目 allowlist、命令 denylist。

### Phase 10：多 Agent 系统

第一版：

```text
main agent
  ├─ research agent
  ├─ test agent
  ├─ review agent
  └─ implementation agent
```

设计要求：agent role、isolated workspace、output contract、result summarization、conflict detection、budget limit。

### Phase 11：项目理解能力

Repo indexing：

```text
.codex/index/
  files.json
  symbols.json
  dependencies.json
  commands.json
  architecture.md
```

自动识别：技术栈、package manager、test/build/lint command、entrypoints、API routes、database schema、env vars、CI workflow、deployment target。

### Phase 12：浏览器和 QA 能力

能力：打开网页、点击、填表、截图、检查 console/network error、responsive 测试、bug report、自动修 bug 后回归测试。

路线：Playwright 本地 browser tool、screenshot tool、DOM inspect tool、console/network capture、visual diff、QA skill。

### Phase 13：Computer Use

能力：截屏、OCR/视觉理解、鼠标点击、键盘输入、应用切换、剪贴板、文件拖拽、权限确认。

场景：IDE、浏览器、设计工具、终端、第三方网站、无 API 软件。

### Phase 14：个人 AI 应用平台化

抽象成：Agent Runtime、Tool Runtime、Workflow Runtime、Memory Runtime、UI Runtime。

可派生应用：AI 编程工具、AI 浏览器助手、AI 自动化办公工具、AI 数据分析助手、AI 运维助手、AI 桌面助理、AI 远程任务执行器。

## 五、开发优先级

### 第一优先级：日常能用的 coding agent

1. 跑通 Codex
2. 理清核心调用链
3. 加多模型配置
4. 加 Anthropic/OpenRouter/DeepSeek
5. 加中文配置体验
6. 优化 TUI/CLI 使用体验
7. 加常用 slash commands

### 第二优先级：长期记忆和 workflow

1. memory
2. context save/restore
3. investigate workflow
4. review workflow
5. commit workflow
6. ship workflow
7. QA workflow

### 第三优先级：UI

1. local app server
2. web UI
3. diff viewer
4. tool call viewer
5. permission modal
6. settings page
7. model/provider page
8. session manager
9. Tauri desktop

### 第四优先级：平台化

1. plugin system
2. skill marketplace
3. multi-agent
4. remote control
5. browser QA
6. computer use
7. cloud sync
8. team collaboration

## 六、MVP 版本

### MVP v0.1：个人可用版

- 本地运行
- 支持 OpenAI + OpenRouter + DeepSeek
- 支持默认模型配置
- 支持读写代码
- 支持 bash
- 支持 git diff
- 支持 plan mode
- 支持中文输出
- 支持基础 memory
- 支持 `/review`
- 支持 `/commit`
- 支持 `/investigate`

### MVP v0.2：舒适版

- provider 设置向导
- model switcher
- 项目级配置
- context save/restore
- 更好的权限确认
- 自动识别测试命令
- 自动运行相关测试
- 更好的错误提示

### MVP v0.3：工作台版

- Web UI
- 会话列表
- diff viewer
- tool call viewer
- permission approval
- settings page
- 项目列表
- 日志面板

### MVP v0.4：自动化版

- QA skill
- review skill
- ship skill
- browser tool
- screenshot
- remote task
- scheduled task

### MVP v1.0：个人 AI 开发系统

- Tauri 桌面端
- 多 agent
- 插件系统
- 完整 memory
- 远程控制
- browser QA
- computer use
- 多模型智能路由
- 安全权限系统

## 七、不建议做的事

1. 不要一开始重写全部 UI：先稳定 agent loop、model provider、tools、config。
2. 不要一开始做云端平台：先本地，后云端。
3. 不要深度复制 cc-haha 源码：可以借鉴功能和体验，不建议复制核心实现。
4. 不要一开始支持太多模型：第一阶段支持 OpenAI、OpenRouter、DeepSeek/Anthropic-compatible 即可。

## 八、长期产品定位建议

### 方向 A：个人版 Claude Code / Codex 增强器

最实际：本地运行、多模型、中文友好、自动化工作流、桌面端。

### 方向 B：AI 开发工作台

项目管理、会话管理、多 agent、QA、review、deploy、dashboard。

### 方向 C：通用 Agent 平台

不只写代码，可做浏览器任务、办公自动化、数据分析、远程控制、插件市场。

建议：先做 A，自然长成 B，最后考虑 C。

## 九、下一步

下一步做 Codex 架构勘察报告：

1. CLI 入口在哪里
2. TUI 入口在哪里
3. agent loop 在哪里
4. model provider 在哪里
5. tool execution 在哪里
6. config loader 在哪里
7. permission/sandbox 在哪里
8. MCP 在哪里
9. app-server 在哪里
10. 哪些地方最适合插入自定义改造

报告保存为：

```text
docs/yuqei-codex-architecture-notes.md
```
