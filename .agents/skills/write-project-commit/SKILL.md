---
name: "write-project-commit"
description: "Prepare repo-local Git commits for this workspace. Use when Codex needs to inspect `git status`, isolate the correct path set, draft or execute a commit, explain commit boundaries, or summarize staged changes in the current project's style: Chinese topic line, flat `- 修改项` entries, only current-theme files, and pre/post commit verification."
---

# Write Project Commit

按当前仓库的提交习惯整理提交边界、撰写提交信息，并在提交前后做最小必要核对。

## 核心约束

1. 只提交当前主题相关文件。
2. 不把并行改动顺手带进提交。
3. 提交信息使用中文，格式固定为：

```text
主题
- 修改项
- 修改项
```

4. `主题` 只表达一个主题，不把多个重构主题混成一个提交。
5. `修改项` 写结果和动作，不写空话，不堆文件清单。
6. 默认先核对边界，再执行 `git add` / `git commit`。

## 工作流

### 1. 先缩提交边界

先看：

```bash
git status --short
git diff --stat
```

必要时继续看：

```bash
git diff -- <path-set>
git diff --cached -- <path-set>
```

目标是先回答这 3 个问题：
- 当前工作区里有哪些并行改动？
- 这次提交真正属于哪个主题？
- 哪些路径应该进这次提交，哪些不该进？

### 2. 明确 path set（路径集合）

只把同一主题的文件归到一个 path set。

常见做法：
- 代码 + 对应测试 + 对应文档/计划回填，可以一起进同一提交。
- 无关 owner 模块、无关 skill、无关临时文件，不进入本次提交。
- 如果工作区里已经有别的 tracked 改动，先在说明里明确“本次提交边界”，不要假设它们也该一起提交。

### 3. 按仓库风格写提交信息

`主题` 的写法：
- 用中文。
- 长度保持短而完整。
- 常用动词风格：
  - `收口...`
  - `新增...`
  - `整理...`
  - `调整...`
  - `修复...`
  - `回填...`
  - `推进...`

`修改项` 的写法：
- 一行一个平铺 bullet，不做嵌套。
- 写“改了什么 + 为什么/结果是什么”。
- 优先描述边界、行为、验证，而不是机械罗列文件名。

推荐风格：

```text
收口 orders_matched serde 风险
- 收紧关键字段反序列化并补充坏值测试
- 提升 payload 解析失败日志并回填阶段计划
```

避免这种写法：

```text
修改一些文件
- 修了一些问题
- 调整了很多内容
```

## 用户意图分流

### 用户说“提交”

直接进入提交流程，但先做边界核对，不要先长篇讨论。

最少要做：
1. 看 `git status --short`
2. 看 `git diff --stat`
3. 说明本次提交边界
4. 写出符合仓库风格的提交信息
5. 提交后再看一次 `git status --short`

### 用户说“全量提交”

仍然先核对工作区里是否有明显无关改动。

如果确实存在并行改动，先明确：
- 是真的要把全部当前变更一起提交；
- 还是只提交当前主题相关文件。

不要因为用户说“全量提交”就跳过边界判断。

### 用户说“写一个 commit message”

只输出提交信息草案即可，不主动执行提交。

输出时优先给：
- 一版推荐标题
- 2 到 4 条平铺 `- 修改项`

## 提交前后检查

提交前：
- 确认 staged / unstaged / untracked 哪些属于本次主题。
- 如果本次不该带上某些并行改动，要在回复里明确指出。

提交后：
- 再跑一次 `git status --short`。
- 如果工作区不干净，说明剩余文件是“并行改动”还是“本次提交后的残留”。
- 不把“还有别的改动”误报成提交失败。

## 沙箱与失败处理

如果 `git add` 或 `git commit` 报：

```text
Unable to create .../.git/index.lock: Read-only file system
```

按“`.git` 写入被环境限制”处理，不按“遗留锁文件”处理。

正确做法：
- 不手工删锁文件；
- 不猜测仓库损坏；
- 直接切换到允许写 `.git` 的执行方式，或请求相应授权。

## 输出要求

- 先说本次提交边界，再说建议的提交信息或执行结果。
- 如果只起草 message，保持短小，不展开通用 Git 教程。
- 如果实际提交了，补一句提交后工作区状态：
  - `git status --short` 为空；
  - 或还有哪些并行改动未纳入本次提交。
