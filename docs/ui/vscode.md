# VS Code Extension —— v0.1 冻结

> Blues VS Code 扩展在 v0.1 仅做**轻量补位**（决策 U6 = B）。
> 完整功能延后到 v0.2。
> 上位文档：[`overview.md`](./overview.md)
>
> 状态：**FROZEN** · 最后修订：2026-05-24

---

## 1. 范围（决策 U6 = B）

### 1.1 v0.1 必做

| 功能 | 形态 |
|---|---|
| 当前 active project 显示 | status bar 左段 |
| 当前 active plan + 状态 | status bar 右段 |
| 命令面板透传 | 8 条核心命令 |
| Memory inline 查询 | 选中文本 → 右键 → "Blues: Query memory" |
| Inbox 候选审批 | 通知气泡 + "Approve / Reject" 按钮 |
| 权限请求弹窗 | `PermissionAsk` 事件触发 modal |

### 1.2 v0.1 不做（延 v0.2）

- 侧栏 view（project tree / plan tree / memory tree）
- Plan Graph 渲染（webview）
- Memory Search 完整界面
- 设置编辑器 GUI
- inline diagnostics（"这段代码与 fact X 冲突"）
- 内嵌聊天 / agent 对话框

### 1.3 不做（不规划）

- 完整 mirror desktop 全功能（避免双线维护）
- VS Code 内启动 plan 的复杂表单（推到命令面板 + simple input）
- 多窗口同步（依赖 daemon ActiveContext，无需扩展自己实现）

---

## 2. 技术栈

| 层 | 选型 |
|---|---|
| 扩展类型 | TypeScript / VS Code Extension API |
| 与 daemon 通信 | Node 子进程包裹 `blues` CLI（`blues --json`），**不**自己说 gRPC |
| 事件订阅 | `blues daemon events --stream-json --filter <kinds>` 子进程 stdout |
| 通知 / status bar | VS Code 原生 API |
| 权限模态 | `vscode.window.showInformationMessage` + actions |

> **决策**：v0.1 不让扩展直连 gRPC。
> 理由：CLI 已封装协议层，扩展用 CLI 子进程模式可跨 IDE 复用（之后做 JetBrains / Zed 都能复用同一个壳逻辑）；少一份协议绑定就少一份维护。
> v0.2 性能/延迟若不够再切原生 gRPC。

---

## 3. 命令面板透传

8 条核心命令，前缀 `Blues:`：

| Command ID | 标题 | 行为 |
|---|---|---|
| `blues.queryMemory` | `Blues: Query memory` | 弹 input → 调 `blues mem query --json` → 结果列表 quickpick |
| `blues.saveMemory` | `Blues: Save memory from selection` | 选中文本 → 弹 type 选择 → 调 `blues mem save` |
| `blues.openInbox` | `Blues: Open inbox` | 调 `blues mem inbox list --json` → quickpick 逐条审批 |
| `blues.useProject` | `Blues: Switch project` | 列 active projects → quickpick → `SetActiveContext` |
| `blues.newPlan` | `Blues: New plan` | 弹 input intent → `blues plan new <intent>` |
| `blues.pausePlan` | `Blues: Pause active plan` | 调 `blues plan pause <active>` |
| `blues.resumePlan` | `Blues: Resume active plan` | 调 `blues plan resume <active>` |
| `blues.daemonStatus` | `Blues: Daemon status` | 调 `blues daemon status --json` → 信息消息 |

> 命令注册全部静态写入 `package.json`，不动态生成。

---

## 4. Status Bar

### 4.1 布局

```
[…    ] [Blues: my-app · plan refactor-auth ◐ · 3 inbox]    [other ext]
                                                            ^ right-aligned
```

### 4.2 段位

单一 status bar item（避免占位多个 slot）：

```
Blues: <project-slug> · plan <plan-slug> <status-icon> · <inbox-count> inbox
```

- 无 plan 时省略 plan 段：`Blues: my-app · 3 inbox`
- 无 inbox 时省略：`Blues: my-app · plan refactor-auth ◐`
- daemon 离线：`Blues: ✗ disconnected`，hover tooltip "Run `blues daemon start`"

### 4.3 状态图标

复用 CLI rich 模式语义（参 `cli.md §1.4`），但 VS Code 用其内置 codicon：

| 状态 | codicon |
|---|---|
| running | `$(sync~spin)` |
| paused | `$(debug-pause)` |
| done | `$(check)` |
| error | `$(error)` |
| disconnected | `$(circle-slash)` |

### 4.4 点击行为

点 status bar item → 弹 quickpick：

```
> Switch project
  Open inbox (3)
  Pause / Resume current plan
  Daemon status
  Settings...
```

---

## 5. 通知与模态

### 5.1 Inbox 候选

daemon 推 `MemoryInboxAdded` → 扩展显示 information message：

```
Blues: New memory candidate (episodic, conf 0.82)
"user prefers logout to be confirmation-less for trusted devices"
[Approve]  [Edit]  [Reject]  [Show all]
```

- 同时多条 → 合并为 "Blues: 5 new memory candidates [Open inbox]"
- 用户点 Edit → 打开临时文档编辑后 approve
- 通知不持久占屏，5s 自动消失（VS Code 默认）；红点保留在 status bar

### 5.2 PermissionAsk

daemon 推 `PermissionAsk` → 扩展弹 **modal**（`{ modal: true }`）：

```
⚠ Plan requests permission

Capability: fs.write
Path: /Users/u/work/my-app/auth.ts
Caller: node N#3 (LlmTask)

[Allow once]  [Allow always for plan]  [Deny]  [Cancel plan]
```

模态阻塞当前 VS Code 编辑器交互（与 desktop 同语义），用户决定后 reply daemon。

### 5.3 plan 完成 / 失败

```
Blues: Plan refactor-auth completed (6 nodes, 12k tokens, 1.2s wall).
[Show log]
```

```
Blues: Plan refactor-auth failed at node N#4: <error message>
[Show log]  [Retry from N#4]
```

`Retry from N#4` 调 `blues plan edit` + `resume`（v0.1 内可达），不依赖 v0.2 的 fork/replay。

---

## 6. Inline 查询交互

### 6.1 选中文本查询

1. 用户选中代码片段或注释
2. 右键 → `Blues: Query memory with selection`
3. 扩展调 `blues mem query "<selection>" --json`
4. 在右侧打开 webview**不要**——v0.1 用 quickpick 列出结果，逐条选择"open in inbox" / "copy fact ID" / "show provenance"

> **决策**：v0.1 拒绝 webview。原因：webview 有维护成本（CSP / state / 主题），v0.2 再做完整 panel。

### 6.2 选中文本存为记忆

1. 用户选中文本
2. 右键 → `Blues: Save selection as memory`
3. quickpick 选 type（episodic / semantic / procedural）
4. 后台调 `blues mem save`，写入 inbox（**不直接落库**，与 MCP save 同语义）
5. 通知 "Saved to inbox. [Approve now]"

---

## 7. 配置项（settings.json）

| 键 | 类型 | 默认 | 说明 |
|---|---|---|---|
| `blues.cliPath` | string | `"blues"` | CLI 可执行路径 |
| `blues.socketPath` | string \| null | `null` | 覆盖 daemon socket（用于多用户调试） |
| `blues.notifications.inbox` | enum | `"individual"` | `individual` / `aggregated` / `silent` |
| `blues.notifications.planComplete` | bool | `true` | plan 完成时通知 |
| `blues.statusBar.showInbox` | bool | `true` |
| `blues.statusBar.showActivePlan` | bool | `true` |
| `blues.activeProjectSync` | enum | `"daemon"` | `daemon`（跟 ActiveContext） / `workspace`（跟 VS Code workspace 根）|

`blues.activeProjectSync = "workspace"` 时：扩展启动时把 workspace folder 路径作为 project（如有 `.blues/project.toml`）传给 daemon SetActiveContext。

---

## 8. 事件订阅

扩展启动后，spawn 一个长驻子进程：

```
blues daemon events --stream-json \
  --filter "ActiveContextChanged,MemoryInboxAdded,PermissionAsk,PlanStateChanged,TokenUsage"
```

stdout NDJSON，每行一个事件，扩展 dispatch 到对应 handler。

子进程死掉 → 重启策略：指数退避（1s / 2s / 4s / 8s，封顶 30s），重试期间 status bar 显示 disconnected。

---

## 9. 与 desktop / CLI 的协同

参 `overview.md §4`。VS Code Ext 需要落实的协同：

- 扩展启动握手时 `SetActiveContext` 用 workspace folder（如配置开启）
- 任何用户在扩展内的"切 project"动作 → 调 `SetActiveContext` → daemon 广播 → desktop / CLI 同步
- desktop 切 project → 扩展通过 `ActiveContextChanged` 事件更新 status bar
- 权限批准在任一表面处理后，其他表面同步关闭模态（daemon 用 `PermissionResolved` 事件广播 —— 协议补丁清单已有 `PermissionAsk`，需新增对称的 `PermissionResolved`）

> **协议补丁追加**：`PermissionResolved { plan_id, node_id, request_id, decision, by_client_kind }`
> 加到 `overview.md §7` 的协议补丁清单。

---

## 10. 打包与分发

| 项 | 决策 |
|---|---|
| Marketplace ID | `blues-tools.blues-vscode` |
| 名称 | `Blues for VS Code` |
| 图标 | 与 desktop 同一套品牌（深色背景 + 蓝紫渐变） |
| 最低 VS Code 版本 | 1.85+ |
| 打包工具 | `vsce` |
| CI | 与主仓 monorepo 共享（v0.1 暂不发 marketplace，先 `.vsix` 本地装） |

v0.1 不上架 marketplace，仅在 GitHub Release 提供 `.vsix`。理由：扩展 v0.1 太薄，先靠 desktop / CLI 立招牌，v0.2 功能补齐再上架。

---

## 11. v0.1 验收清单（VS Code Ext）

### 必须

- [ ] 扩展正常 activate，连上 daemon（CLI 子进程模式）
- [ ] Status bar 显示 active project / plan / inbox 三段
- [ ] 8 条命令面板命令全部可用
- [ ] Inbox 候选通知 + Approve/Edit/Reject 流程
- [ ] PermissionAsk modal + 批准 round-trip
- [ ] Plan 完成 / 失败通知
- [ ] Inline query memory（quickpick）
- [ ] Inline save memory（quickpick + write to inbox）
- [ ] 配置项全部生效
- [ ] daemon 断线重连指数退避
- [ ] `.vsix` 在 Linux / macOS / Windows VS Code 端到端通过

### 可选

- [ ] codicon 主题适配（深 / 浅色都好看）

---

## 12. 不在 v0.1 范围

- 侧栏 view（v0.2）
- Plan Graph webview（v0.2）
- Memory Search 完整界面（v0.2）
- inline diagnostics（v0.3+）
- 内嵌聊天面板（v0.3+，可能不做）
- Marketplace 发布（v0.2）
- JetBrains / Zed 同源扩展（v0.3+，但 CLI 子进程模式给未来留好路）

---

**FROZEN. VS Code Ext v0.1 冻结于 2026-05-24。**
