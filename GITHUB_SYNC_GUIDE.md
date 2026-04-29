# GitHub Fork / Clone / Sync 教学笔记

本文记录这次把 OpenAI Codex release 版本纳入自己 GitHub fork 的完整流程。

适用场景：

- 官方仓库有一个 release / pre-release 版本。
- 你想先 fork 到自己的 GitHub。
- 再 clone 到本地电脑。
- 以后官方更新时，可以继续用 Git 同步新版本。

本次示例：

- 官方仓库：`https://github.com/openai/codex`
- 你的 fork：`https://github.com/cyuqei/codex`
- 本地目录：`/Users/yuqei/codex-git`
- 目标版本 tag：`rust-v0.126.0-alpha.12`
- 本地工作分支：`my/rust-v0.126.0-alpha.12`

## 1. 核心概念

### release 页面不是 Git 仓库

如果从 GitHub release 页面下载源码 zip，解压出来的目录通常没有 `.git`。

没有 `.git` 的目录不能用：

```bash
git pull
git fetch
git push
git switch
```

所以长期维护一定要用 `git clone`。

### fork 是你自己的远程仓库

fork 后，你会有自己的仓库：

```text
https://github.com/cyuqei/codex
```

这个仓库通常叫 `origin`。

### upstream 是官方仓库

官方仓库：

```text
https://github.com/openai/codex
```

我们把它命名为 `upstream`，以后从这里拉官方更新。

### tag 是固定版本快照

例如：

```text
rust-v0.126.0-alpha.12
```

这是一个固定版本，不会自己变化。以后官方发布新版本，会出现新的 tag。

## 2. 第一次准备仓库

先进入你想放项目的目录：

```bash
cd /Users/yuqei
```

从官方仓库 clone：

```bash
git clone https://github.com/openai/codex.git codex-git
cd codex-git
```

把官方仓库改名为 `upstream`：

```bash
git remote rename origin upstream
```

添加你自己的 fork 作为 `origin`：

```bash
git remote add origin https://github.com/cyuqei/codex.git
```

检查 remote：

```bash
git remote -v
```

正常应该类似：

```text
origin    https://github.com/cyuqei/codex.git (fetch)
origin    https://github.com/cyuqei/codex.git (push)
upstream  https://github.com/openai/codex.git (fetch)
upstream  https://github.com/openai/codex.git (push)
```

## 3. 切到某个 release / pre-release 版本

先拉取官方所有 tag：

```bash
git fetch upstream --tags --prune
```

基于目标 tag 创建自己的分支：

```bash
git switch -c my/rust-v0.126.0-alpha.12 rust-v0.126.0-alpha.12
```

把这个分支推送到自己的 fork：

```bash
git push -u origin my/rust-v0.126.0-alpha.12
```

成功时会看到类似：

```text
To https://github.com/cyuqei/codex.git
 * [new branch] my/rust-v0.126.0-alpha.12 -> my/rust-v0.126.0-alpha.12
branch 'my/rust-v0.126.0-alpha.12' set up to track 'origin/my/rust-v0.126.0-alpha.12'.
```

GitHub 可能会提示创建 Pull Request，这是正常提示，不是错误。

## 4. GitHub 登录认证

GitHub 网站密码不能再用于命令行 `git push`。

如果看到：

```text
Password authentication is not supported for Git operations.
```

说明你在 `Password:` 里输入了 GitHub 网站密码。

现在推荐两种方式。

## 5. 推荐方式：GitHub CLI 登录

安装 GitHub CLI：

```bash
brew install gh
```

登录：

```bash
gh auth login
```

交互选项建议：

```text
GitHub.com
HTTPS
Authenticate Git with your GitHub credentials? Yes
Login with a web browser
```

浏览器会打开 GitHub 授权页面。

页面提示：

```text
Authorize your device
Enter the code displayed in the app or on the device you're signing in to.
```

这是正常的。把终端里显示的一次性 code 输入网页即可。

登录完成后，再执行：

```bash
git push -u origin my/rust-v0.126.0-alpha.12
```

## 6. 备用方式：Personal Access Token

打开：

```text
https://github.com/settings/tokens
```

创建 token。Classic token 通常勾选：

```text
repo
workflow
```

以后 `git push` 提示：

```text
Username for 'https://github.com':
Password for 'https://...@github.com':
```

填写：

```text
Username: cyuqei
Password: 粘贴 token，不是 GitHub 网站密码
```

如果 macOS 记住了旧密码，可以清理：

```bash
printf "protocol=https\nhost=github.com\n\n" | git credential-osxkeychain erase
```

然后重新 push。

## 7. 为什么不用 SSH

你之前遇到：

```text
Connection closed by 198.18.0.22 port 22
fatal: Could not read from remote repository.
```

这通常表示当前网络或代理拦截了 GitHub SSH 的 22 端口。

所以这次我们使用 HTTPS + GitHub CLI 登录，更稳定。

如果以后想修 SSH，可以测试：

```bash
ssh -T git@github.com
```

如果 22 端口一直失败，可以考虑 GitHub SSH over 443，但日常使用 HTTPS 已经足够。

## 8. 查看当前仓库状态

进入仓库：

```bash
cd /Users/yuqei/codex-git
```

查看当前分支：

```bash
git branch --show-current
```

查看分支跟踪关系：

```bash
git branch -vv
```

查看远程仓库：

```bash
git remote -v
```

查看当前是否有本地改动：

```bash
git status
```

## 9. 以后官方发布新版本时怎么同步

比如官方以后发布：

```text
rust-v0.126.0-alpha.13
```

先拉取官方更新：

```bash
cd /Users/yuqei/codex-git
git fetch upstream --tags --prune
```

基于新 tag 创建你的分支：

```bash
git switch -c my/rust-v0.126.0-alpha.13 rust-v0.126.0-alpha.13
```

推到你的 fork：

```bash
git push -u origin my/rust-v0.126.0-alpha.13
```

## 10. 如果你在旧版本分支上做了自己的修改

假设你在旧分支有自己的提交：

```text
my/rust-v0.126.0-alpha.12
```

新版本出来后，你可以先创建新版本分支：

```bash
git fetch upstream --tags --prune
git switch -c my/rust-v0.126.0-alpha.13 rust-v0.126.0-alpha.13
```

然后把自己的提交搬过来。

查看旧分支提交：

```bash
git log --oneline my/rust-v0.126.0-alpha.12
```

把某个提交复制到当前分支：

```bash
git cherry-pick <commit-hash>
```

如果有冲突，解决冲突后：

```bash
git add <冲突文件>
git cherry-pick --continue
```

最后推送：

```bash
git push -u origin my/rust-v0.126.0-alpha.13
```

## 11. 常见报错

### 报错：Password authentication is not supported

原因：GitHub 不允许命令行用网站密码。

解决：

```bash
gh auth login
```

或者使用 Personal Access Token。

### 报错：Could not read from remote repository

可能原因：

- SSH 端口被拦截。
- 仓库地址写错。
- fork 不存在。
- SSH key 没配置。

优先解决方式：改用 HTTPS remote。

```bash
git remote set-url origin https://github.com/cyuqei/codex.git
```

### 报错：tag 不存在

先拉取官方 tag：

```bash
git fetch upstream --tags --prune
```

检查 tag：

```bash
git tag | grep rust-v0.126
```

## 12. 本次最终结果

本地仓库：

```text
/Users/yuqei/codex-git
```

当前分支：

```text
my/rust-v0.126.0-alpha.12
```

已经推送到：

```text
https://github.com/cyuqei/codex/tree/my/rust-v0.126.0-alpha.12
```

以后只需要记住三个动作：

```bash
git fetch upstream --tags --prune
git switch -c my/<新版本tag> <新版本tag>
git push -u origin my/<新版本tag>
```

这就是 fork + upstream + tag 分支的基本工作流。
