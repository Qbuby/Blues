# UI 形态总览（UI Surfaces）—— v0.1 冻结

> Blues 三态（Desktop / CLI / VS Code Ext）+ 视觉调性总冻结档。
> 子文档 `desktop.md` / `cli.md` / `vscode.md` 在此基础上展开。
>
> 状态：**FROZEN** · 最后修订：2026-05-24

---

## 0. 三态分工与优先级

```
┌────────────────────────┐  ┌────────────────────┐  ┌────────────────────┐
│  Desktop (Tauri)       │  │  CLI               │  │  VS Code Ext       │
│  ── v0.1 主交付 ──     │  │  ── v0.1 必备 ──    │  │  ── v0.2 补位 ──   │
│  护城河视觉 + 主屏     │  │  daemon/project     │  │  侧栏轻量补充      │
│  Plan Graph + Inbox    │  │  bootstrap + 脚本化 │  │  inline 审批       │
└────────────┬───────────┘  └─────────┬──────────┘  └─────────┬──────────┘
             │                        │                       │
             └──── gRPC over UDS / Named Pipe ────────────────┘
                              │
                     ┌────────▼─────────┐
                     │  Blues Daemon    │
                     └──────────────────┘
```

**v0.1 优先级（决策 U1 = A）**：

1. **Desktop 优先**：完成度按 desktop > CLI > VS Code Ext
2. **CLI 必备**：daemon 控制、project init、脚本化是底层刚需，desktop 也依赖它
3. **VS Code Ext 延后**：v0.1 只占名（status bar + 命令面板透传），完整功能 v0.2

> 优先级是**完成度**优先级，不是**存在性**优先级。
> v0.1 三态都必须能跑端到端，但 desktop 是用户感知的主要表面。

---

## 1. 已冻结决策清单

### UI 议题（U1-U8）

| ID | 决策 | 选项 | 文档 |
|---|---|---|---|
| **U1** | 三态优先级 | **A** Desktop 优先 | 本文 §0 |
| **U2** | Desktop 主屏布局 | **B** 三栏（project / plan graph / inbox） | `desktop.md` §2 |
| **U3** | Memory Inbox 位置 | **A+B** 全局页签 + 嵌入 plan 流双显 | `desktop.md` §3 |
| **U4** | Plan Graph 视觉 | **A 默认 + B 切换**（DAG 默认 / 星座可切） | `desktop.md` §4 |
| **U5** | CLI 叙事风格 | **C** 极简默认 + `--rich` 开 TUI | `cli.md` §1 |
| **U6** | VS Code Ext 范围 | **B** 轻量补充 | `vscode.md` §1 |
| **U7** | "Make universes" 视觉锚 | **B** 时空裂变（fork 动效） | `desktop.md` §6 |
| **U8** | Onboarding | **A** 即刻 init + 试跑 | `desktop.md` §7 |

---

## 2. 视觉调性（三态共用）

### 2.1 基线

| 维度 | 取值 |
|---|---|
| 主色调 | 深色（`#0B0D12` 背景 / `#E6E8EE` 前景） |
| 强调色 | 蓝紫渐变（`#5B7CFA` → `#8B5CF6`），fork 用 `#F59E0B`（琥珀） |
| 字体 - 正文 | Inter / 系统 sans |
| 字体 - 代码 / 终端 / Plan Graph 标签 | **Geist Mono**（首选）/ JetBrains Mono（fallback） |
| 圆角 | 8px（卡片）/ 4px（按钮）/ 0px（终端） |
| 动效曲线 | `cubic-bezier(0.4, 0, 0.2, 1)`，250ms 默认 |

### 2.2 情绪

> **极客感 + 一点冷静的浪漫。**
> 不是赛博朋克的霓虹，不是企业 SaaS 的灰白，是观星者的工作站。

具体禁忌：
- ❌ 不用霓虹粉、霓虹绿、超饱和荧光色
- ❌ 不用拟物（按钮不要阴影 3D 凸起）
- ❌ 不用插画风吉祥物
- ❌ 不堆叠多种字体
- ✅ 用纯色 + 渐变 + 微光（glow）
- ✅ 用动效表达"时间维度"（fork 裂变 / replay 回流）
- ✅ 关键路径用 mono 字体，强化 hacker 气质

### 2.3 双语义场使用规则

参见 `BLUES_VALUES.md §5`：

| 场合 | 语义场 | 例 |
|---|---|---|
| CLI 子命令 / API 名 | Git 语义场 | `blues plan fork` / `blues mem blame` |
| Desktop 按钮文字（功能位） | Git 语义场 | "Fork from this node" |
| Desktop 视觉效果命名 | 宇宙语义场（仅内部） | "constellation view" / "fork burst" |
| Landing / 营销素材 | 宇宙语义场 | "Make universes with AI" |
| 错误消息 / 日志 | Git 语义场 | "Plan diverged at node N" |

> **铁律**：用户输入路径（CLI / 按钮 label）永远说人话。宇宙美学只在情绪面（视觉、营销、过场）用。

---

## 3. 倒推：UI 决策对下游的影响

### 3.1 对 Memory 引擎

- **Inbox 嵌入 plan 流**（U3 = B 部分）→ daemon `MemoryInboxAdded` 事件必须携带 `plan_id` / `node_id`，否则前端无法定位插入点
- **Inbox 全局页签**（U3 = A 部分）→ inbox 必须支持按 project / 时间 / confidence 过滤
- **Onboarding 即刻试跑**（U8）→ memory 引擎首次写入要在 < 5s 内可见结果，否则破体感

### 3.2 对 Plan Graph

- **双视图（DAG + 星座）**（U4）→ plan 数据结构必须**纯数据**（节点 + 边 + 状态 + 元数据），渲染层独立。两套 renderer 共享同一份 `PlanState`
- **时空裂变作为视觉锚**（U7）→ daemon `PlanStateChanged` 事件中 fork 必须有独立 op type（不是普通 `node_added`），前端才能播裂变动效
- **Replay/Rewind 是一级公民**（v0.2，但需为 v0.1 视觉留位）→ 节点必须可点击进入"历史态查看"

### 3.3 对 Daemon SubscribeEvents

UI 倒逼以下事件必须存在（已在 `protocol-and-project.md §3.3` 列出）：

- `PlanStateChanged` / `NodeStateChanged` / `NodeOutputDelta`：plan graph 实时刷新
- `MemoryInboxAdded`：inbox 红点 + 卡片
- `PermissionAsk`：阻塞模态弹窗
- `TokenUsage`：左下角 status bar 滚动

新增（UI 议题确认后追加）：

- `PlanForked { plan_id, parent_node_id, new_plan_id }` — 前端播裂变动效
- `PlanReplayed { plan_id, from_node_id }` — 前端播时间回流动效（v0.2）

> 这两个事件 `protocol-and-project.md §3.3` 现在没有，**v0.1 协议层必须补上**。
> 不需要重开 RFC，UI 议题冻结即触发协议层补丁。

---

## 4. 三态间状态切换契约

> 用户在三态之间切换，**绝不能丢上下文**。这是 BLUES_VALUES §9 的硬承诺。

### 4.1 共享状态

所有三态共享 daemon 持有的：

- 当前 active project
- 当前 active plan（如有）
- inbox 未读计数
- token usage 当日累计

### 4.2 切换体感

| 场景 | 期望 |
|---|---|
| Desktop 打开，CLI 跑 `blues plan log` | CLI 立即看到 desktop 当前选中的 plan |
| CLI 跑 `blues mem save`，desktop 已开 inbox 页 | desktop inbox 实时插入新候选（动画下滑） |
| VS Code Ext 在 plan node 点 "approve permission"，desktop 模态同步关闭 | 模态关闭并显示"已由 vscode-ext 批准" |
| Desktop 选中 project A，新开 CLI 终端 | 新 CLI 默认作用于 project A（除非 cwd 指向别的 project） |

### 4.3 实现路径

- daemon 维护 `ActiveContext { project_id, plan_id, ... }` 单例
- 每个客户端通过 `Hello` 握手获得 `session_token`，订阅 `ActiveContextChanged` 事件
- 客户端切换 project / plan 时调 `SetActiveContext` RPC，daemon 广播给所有订阅者
- CLI 默认无状态，但读取 daemon 的 `ActiveContext` 作为默认值（除非 `--project` / `--plan` 显式覆盖）

> 此契约新增 RPC：`SetActiveContext` / 事件：`ActiveContextChanged` —— 由 UI 议题冻结触发，加入协议层补丁清单。

---

## 5. v0.1 UI 交付物

### Desktop（主交付）

1. ✅ Tauri 应用启动 + 连接 daemon（gRPC over UDS/Named Pipe，Tauri command 桥接）
2. ✅ 三栏主屏（project tree / plan graph / inbox+sidebar）
3. ✅ Plan Graph DAG 视图（v0.1 不要求星座视图，但数据层要为切换留位）
4. ✅ Memory Inbox：全局页签 + plan 流嵌入卡片
5. ✅ Onboarding：首次打开检测 cwd → init 引导 → 跑第一个 plan
6. ✅ 视觉调性按 §2 落地

### CLI（必备）

1. ✅ `blues daemon {start,stop,status}` + `blues project {init,list,info}`
2. ✅ `blues mem {query,save,inbox}` 全量
3. ✅ `blues plan {new,list,log,pause,resume,cancel,inject}` 全量
4. ✅ 默认极简（grep-friendly），`blues plan watch` / `blues mem inbox` 自动开 TUI
5. ✅ 全局 flags：`--project` / `--json` / `--no-color` / `--rich`

### VS Code Ext（补位）

1. ✅ 仅 status bar 显示当前 project + active plan
2. ✅ 命令面板透传若干常用命令（query memory / pause plan / approve inbox）
3. ✅ 不做侧栏视图、不做 plan graph、不做 onboarding
4. ⏭ 完整功能延后到 v0.2

---

## 6. 不在 v0.1 范围

- 移动端（不规划）
- Web 应用 / SaaS 形态（不规划，本地优先原则）
- 主题切换 / 深浅色（v0.2，v0.1 仅深色）
- 多语言 i18n（v0.2，v0.1 仅 zh-CN + en-US 双轨硬编码）
- 桌面通知 / 系统托盘（v0.2）
- Plan Graph 星座视图（v0.2，数据层留位即可）
- Replay / Rewind / Time-travel UI（v0.2，与 plan 引擎一起做）
- 自定义键位（v0.2）

---

## 7. 协议层补丁清单（UI 议题触发）

UI 决策冻结后，**自动补到 `protocol-and-project.md` 下一次 schema bump 的 RFC 里**。在此先备案：

1. 新增 RPC `SetActiveContext(SetActiveContextReq) returns (Empty)`
2. 新增事件 `ActiveContextChanged { project_id, plan_id?, by_client_kind }`
3. 新增事件 `PlanForked { plan_id, parent_node_id, new_plan_id }`
4. 新增事件 `PlanReplayed { plan_id, from_node_id }`（v0.2 实现，v0.1 占位）
5. `MemoryInboxAdded` 增加可选字段 `plan_id` / `node_id`，便于前端嵌入定位

> 这五条是 UI 议题倒逼的协议变更，规模小，不开新 RFC，直接在协议 v0.1 实现时一并写入。

---

## 8. 子文档索引

- [`desktop.md`](./desktop.md)：三栏布局 / Plan Graph / Inbox / 视觉细节 / Onboarding
- [`cli.md`](./cli.md)：命令骨架 / 输出模式 / TUI 子命令 / exit codes
- [`vscode.md`](./vscode.md)：v0.1 占位范围 / status bar / 命令面板

---

**FROZEN. UI 议题（U1-U8）冻结于 2026-05-24。**
