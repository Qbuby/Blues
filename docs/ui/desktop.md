# Desktop UI（Tauri）—— v0.1 冻结

> Blues v0.1 主交付表面。三栏布局 + Plan Graph + Memory Inbox + Onboarding。
> 上位文档：[`overview.md`](./overview.md)
>
> 状态：**FROZEN** · 最后修订：2026-05-24

---

## 1. 应用骨架

### 1.1 技术栈

| 层 | 选型 | 理由 |
|---|---|---|
| 壳 | Tauri 2.x | Rust 后端无缝、包体积小、跨平台 |
| 前端框架 | SolidJS | 细粒度响应式适配高频事件流（plan / inbox），bundle 小 |
| 样式 | Tailwind CSS + CSS Variables（主题色） | 调性一致、无 runtime 开销 |
| 图渲染 | Cytoscape.js（DAG 默认） + Pixi.js（星座视图，v0.2） | 两套 renderer 共享 plan-graph 纯数据层 |
| 终端组件（plan 节点输出预览） | xterm.js | 复用业界标准 |
| 状态管理 | SolidJS signals + 本地 store | 不引入 Redux 类重型库 |

> 选型一旦确认即冻结。任何替换需走 RFC（理由：影响整个前端工程基线）。

### 1.2 目录结构

```
desktop/
├── src-tauri/                  # Rust 后端（Tauri）
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/           # Tauri command -> blues-protocol gRPC client
│   │   │   ├── project.rs
│   │   │   ├── plan.rs
│   │   │   ├── memory.rs
│   │   │   └── daemon.rs
│   │   └── events/             # daemon SubscribeEvents -> Tauri event emit
│   │       └── relay.rs
│   └── tauri.conf.json
└── src/                        # SolidJS 前端
    ├── app.tsx
    ├── shell/                  # 三栏布局壳
    ├── views/
    │   ├── project-tree/
    │   ├── plan-graph/         # DAG renderer
    │   ├── plan-inspector/     # node 详情浮窗
    │   ├── memory-inbox/       # 全局 inbox + 嵌入卡片
    │   ├── onboarding/
    │   └── settings/
    ├── stores/
    │   ├── active-context.ts   # 镜像 daemon 的 ActiveContext
    │   ├── plan-state.ts
    │   └── inbox.ts
    └── theme/
```

### 1.3 与 daemon 的桥接（决策 G2 = A）

```
Frontend (SolidJS)
    ↓ Tauri invoke()
Tauri command (Rust)
    ↓ blues-protocol gRPC client
Daemon (UDS / Named Pipe)
```

**前端不直接说 gRPC**，统一通过 Tauri command 调用。前端拿到的是 TypeScript-friendly 的 plain object。

事件流方向：

```
Daemon SubscribeEvents (gRPC stream)
    ↓ events/relay.rs（Rust 侧持久订阅）
Tauri emit("blues:event", ...)
    ↓
Frontend listen()
```

---

## 2. 主屏布局（决策 U2 = B）

### 2.1 三栏

```
┌──────────────────────────────────────────────────────────────────────────┐
│  [topbar]  client-acme · plan: refactor-auth · ●●● 3 inbox · 12k tok    │
├──────────┬──────────────────────────────────────────┬────────────────────┤
│          │                                          │                    │
│ Projects │           Plan Graph (DAG)               │  Active Plan       │
│ ──────── │                                          │  Inbox Cards       │
│ ●client- │     ●─→●─→●─→●                          │                    │
│  acme    │           ↓                              │  ┌──────────────┐  │
│ ○shared- │           ●─→●                           │  │ candidate #1 │  │
│  skills  │                                          │  │ "user prefer │  │
│ ○lessons │     [legend / view toggle]               │  │  logout..."  │  │
│ ○...     │                                          │  │ [Approve]    │  │
│          │                                          │  └──────────────┘  │
│ + new    │                                          │  ...               │
│          │                                          │                    │
├──────────┴──────────────────────────────────────────┴────────────────────┤
│  [statusbar]  daemon ✓ 32ms · ollama ✓ · kiro ✓ · global mem 1.2k facts │
└──────────────────────────────────────────────────────────────────────────┘
```

| 栏 | 宽度 | 内容 |
|---|---|---|
| 左 - Project Tree | 240px（可调，最小 180px） | active projects 列表 / + 新建 / 嵌套展开 |
| 中 - Plan Graph | flex | DAG 视图主区，下方折叠 Plan Inspector 抽屉 |
| 右 - Active Plan Inbox | 320px（可调，可整体收起） | 当前 plan 产出的记忆候选卡片（U3 = B） |

### 2.2 Topbar

```
[logo] [active project ▾] [active plan ▾]    ●●●3  [tokens]  [settings ⚙]
```

- **active project**：单击下拉切换，对应调 `SetActiveContext`
- **active plan**：单击进入 plan list 抽屉
- **inbox 红点**：全局未审批数（U3 = A），点击进 Inbox 全局页签
- **tokens**：当日 cumulative，hover 显示按 provider/model 拆解

### 2.3 Statusbar

| 段 | 内容 | 状态色 |
|---|---|---|
| daemon | `✓ <ping>ms` 或 `✗ disconnected` | 绿 / 红 |
| 各 provider | `<name> ✓` 或 `✗`（按 ListModels 心跳） | 绿 / 红 |
| 全局记忆 | `global mem <count> facts` | 灰 |

statusbar 不是装饰，是 health 总览。所有底层故障都先在这里冒头。

### 2.4 全局页签切换（左下角或顶 nav）

```
[Plans] [Inbox] [Memory Search] [Skills] [Settings]
```

- **Plans**：默认页，三栏布局
- **Inbox**：全局 inbox 总览（U3 = A）
- **Memory Search**：全文 + 向量混合检索 UI，可看 provenance
- **Skills**：已安装 skill 列表 / 启用切换 / 来源
- **Settings**：profile / providers / policy / 路径配置

---

## 3. Memory Inbox（决策 U3 = A+B）

### 3.1 双显模式

| 模式 | 位置 | 内容 |
|---|---|---|
| **嵌入模式（B）** | 三栏右栏 | 仅当前 active plan 产出的候选 |
| **全局模式（A）** | Inbox 页签 | 所有 project 的全部候选，可过滤 |

### 3.2 候选卡片结构

```
┌────────────────────────────────────────────────┐
│ episodic │ confidence 0.82 │ from node N#3     │
│ ──────────────────────────────────────────────│
│ "user prefers logout to be confirmation-less   │
│  for trusted devices"                          │
│                                                │
│ source: chat at 14:32 / file:auth.ts:42       │
│ ──────────────────────────────────────────────│
│ [Approve]  [Edit]  [Reject]   ⋯               │
└────────────────────────────────────────────────┘
```

字段：

- **type**：`episodic` / `semantic` / `procedural` 色卡区分
- **confidence**：0-1 横条，< 0.5 红 / 0.5-0.8 黄 / > 0.8 绿
- **from**：plan_id / node_id / 文件 / 用户输入 来源链
- **content**：候选事实正文，可展开看 raw extraction context

### 3.3 操作

| 操作 | 行为 |
|---|---|
| Approve | 调 `ApproveInboxItem`，卡片淡出 |
| Edit | 进入内联编辑模式，保存即 approve（含编辑） |
| Reject | 调 `RejectInboxItem`，卡片左滑消失，可撤销（5s） |
| 批量 approve | 多选 + 顶部按钮，confirm 弹窗（>10 条强制） |
| 全选 type X | 一键选中某 type 全部 |

### 3.4 事件流

```
daemon: MemoryInboxAdded { project_id, item_id, plan_id?, node_id?, summary, confidence }
    ↓
Tauri relay
    ↓
frontend inbox store
    ├── 全局 inbox 顶部插入卡片（带"new" 高亮 1s）
    └── 如果 plan_id 匹配 active plan → 右栏同步插入
```

---

## 4. Plan Graph（决策 U4 = A 默认 + B 切换）

### 4.1 DAG 默认视图

- **布局算法**：左→右分层（dagre），同层节点垂直堆叠
- **节点形态**：圆角矩形，含 icon（LlmTask / Tool / Subgraph）+ 标题 + 状态
- **状态色**：

| status | 颜色 | 动效 |
|---|---|---|
| pending | 灰 | 无 |
| running | 蓝紫渐变 | 边框扫光 |
| paused | 琥珀 | 静态 |
| done | 绿 | 无 |
| error | 红 | 抖动一次 |
| forked | 琥珀虚线边 | fork 瞬间裂变（参 §6） |

- **边**：实线箭头默认，依赖关系；fork 边用琥珀虚线
- **缩放**：滚轮 + 双指捏合，Ctrl+0 重置

### 4.2 Inspector 抽屉

点节点 → 底部滑出 Inspector：

```
┌──────────────────────────────────────────────────────────┐
│ Node N#3  LlmTask  status: running   started 14:32:11   │
├──────────────────────────────────────────────────────────┤
│ [Output] [Inputs] [Memory used] [Tokens] [Permissions]  │
│ ────────────────────────────────────────────────────────│
│  ...streaming text from NodeOutputDelta...               │
└──────────────────────────────────────────────────────────┘
```

Tabs：

- **Output**：xterm.js 流式 chat 输出
- **Inputs**：node 的 prompt / params
- **Memory used**：CompileContext 输出的来源 facts，每条可点跳转
- **Tokens**：本节点 token 消耗
- **Permissions**：本节点请求过的 capability，approve / reject 历史

### 4.3 节点交互菜单（右键）

```
▸ View output
▸ Edit & re-run        (EditNode)
▸ Inject message       (InjectMessage)
▸ Pause from here
▸ Fork from this node  (v0.2)
▸ Replay this node     (v0.2)
▸ Rewind to here       (v0.2)
▸ Copy node ID
▸ Copy provenance
```

v0.2 项 v0.1 阶段灰色禁用，hover tooltip "v0.2"。

### 4.4 星座视图（v0.2，但占位）

- v0.1 toolbar 上有切换按钮，灰色禁用，tooltip "v0.2"
- 数据层 `PlanState` 在 v0.1 已包含星座所需字段（节点位置可由力导向算出，不依赖额外数据）

---

## 5. 视觉细节

### 5.1 调色板

| token | 值 | 用途 |
|---|---|---|
| `--bg-primary` | `#0B0D12` | 主背景 |
| `--bg-secondary` | `#12151C` | 卡片 / 抽屉 |
| `--bg-tertiary` | `#1A1F29` | hover / active |
| `--fg-primary` | `#E6E8EE` | 主文字 |
| `--fg-secondary` | `#9AA0AC` | 次文字 |
| `--fg-tertiary` | `#5C6470` | 辅助 / placeholder |
| `--accent-primary` | `#5B7CFA` | 主交互（按钮 / 链接） |
| `--accent-secondary` | `#8B5CF6` | 强调（active plan） |
| `--accent-fork` | `#F59E0B` | fork 专用 |
| `--success` | `#34D399` | done / online |
| `--warning` | `#FBBF24` | paused |
| `--error` | `#F87171` | error / offline |
| `--border` | `#252A35` | 分隔线 |

### 5.2 字体

```css
:root {
  --font-sans: "Inter", system-ui, sans-serif;
  --font-mono: "Geist Mono", "JetBrains Mono", ui-monospace, monospace;
}
```

`--font-mono` 用于：plan 节点 ID / token 数 / 时间戳 / xterm / 全部 status bar 内容 / Memory Search 结果 facts 的 ID。

### 5.3 动效

| 场景 | 动效 | 时长 |
|---|---|---|
| 卡片插入 inbox | 顶部下滑 + 0→1 透明度 | 250ms |
| 卡片 approve | 右滑 + 1→0 透明度 | 200ms |
| 卡片 reject | 左滑 + 1→0 透明度 | 200ms |
| Plan node running 边框扫光 | linear gradient 旋转 | 2s loop |
| Plan node done 闪绿 | 0.5s 单次 | 500ms |
| Plan fork（U7 = B） | 见 §6 | 800ms |
| 模态弹出 | scale 0.96→1 + 透明度 | 200ms |
| 抽屉滑出 | translate-y | 300ms |

> 所有动效遵循 §2 overview 的 `cubic-bezier(0.4, 0, 0.2, 1)`。
> `prefers-reduced-motion: reduce` 时禁用所有非必要动效（仅保留状态色）。

### 5.4 间距与栅格

- 8px 基础栅格，所有 padding/margin 用 `4 / 8 / 12 / 16 / 24 / 32` 阶
- 卡片内 padding 16，卡片间 gap 12
- 三栏 splitter 宽 4px，hover 高亮

---

## 6. 时空裂变动效（决策 U7 = B）

> "Make universes with AI" 的视觉锚定点。
> 每次 plan fork 触发，**在主屏 plan graph 上播一次**，是 Blues 唯一的"超出工具感"的瞬间。

### 6.1 触发

daemon 事件 `PlanForked { plan_id, parent_node_id, new_plan_id }` 到达 → 前端在 parent_node 位置播动效。

### 6.2 视觉序列（800ms）

```
0ms     parent node 边框扩散一圈琥珀光晕（半径 +40px，0.5 透明度）
        同时一道琥珀光线从 parent node 射出
200ms   光晕收缩回 node，光线持续延伸
400ms   光线尽头出现新 node "种子"（闪烁的圆点）
600ms   种子展开成完整 node，琥珀虚线连接 parent → new
800ms   稳定，new node 状态为 pending（普通色）
```

### 6.3 实现要点

- DAG 视图：dagre 布局重算时**禁用过渡**，让新节点位置一次性出现，由动效负责"出生"
- 星座视图（v0.2）：力导向给新节点初始位置，动效用 Pixi.js shader 做粒子裂变
- 性能：动效用 CSS transform / opacity，不触发布局重算

### 6.4 不做

- 不做"宇宙背景星空"
- 不做循环常驻动画（除节点状态色）
- 不做粒子系统作为持续装饰

> Blues 的视觉浪漫**只在关键事件瞬间**爆发。日常使用是冷静的工作站。

---

## 7. Onboarding（决策 U8 = A）

### 7.1 首次启动流

```
[App start]
    ↓
检测 daemon 是否存在 → 否 → 自动 spawn（on-demand 模式，参协议 §2.1）
    ↓
显示欢迎屏（3 句话）
    ↓
"Open a folder to begin" 按钮
    ↓
用户选目录
    ↓
检测 .blues/project.toml
    ├── 存在 → 直接进主屏
    └── 不存在 → 启发识别（参协议 §1.4）
                ├── 找到候选根 → 弹三选（init / 临时 / skip）
                └── 未找到 → 提示"This doesn't look like a project. Init here?"
    ↓
[init 后]
    ↓
进入主屏，自动建议第一个 plan：
    "Try saying: 'help me understand this project'"
```

### 7.2 欢迎屏文案

```
Welcome to Blues.

Git for AI collaboration.
Branch your AI conversations. Fork your decisions. Travel back in time.

[Open a folder]      [Restore a project]
```

> 不做产品教程页 / 视频引导 / 三步介绍。目标用户讨厌教程。

### 7.3 第一次成功体感

定义 **"first happy path"**：用户从打开 app 到看到第一条 memory 候选出现在 inbox，**< 90s**。

最低要求：

1. daemon spawn < 200ms（协议层 SLA）
2. 目录识别 < 1s
3. init `.blues/project.toml` < 500ms
4. 第一个 plan 启动 < 2s（首次 model 调用不算）
5. plan 跑出第一条 memory 候选 < 60s（首次 model 调用 + extraction）

任何环节超时即破体感。SLA 写入 v0.1 验收清单。

---

## 8. 错误与离线状态

### 8.1 daemon 断连

- statusbar daemon 段变红 `✗ disconnected`
- 顶部横条横幅："Daemon disconnected. Trying to reconnect... [Retry now]"
- 主区不锁，只读模式（缓存的 plan / inbox 仍可看）
- 重连成功后横幅自动消失，所有 store 重新拉取

### 8.2 model provider 故障

- statusbar 对应 provider 段变红
- 主区不阻塞，但启动 plan 时如果路由命中故障 provider，弹 toast：
  ```
  Provider <name> is offline. Route to <fallback>?  [Yes] [Cancel]
  ```

### 8.3 PermissionAsk 模态

```
┌──────────────────────────────────────────┐
│ ⚠  Plan requests permission              │
├──────────────────────────────────────────┤
│ Capability: fs.write                     │
│ Path: /Users/.../auth.ts                 │
│ Caller: node N#3 (LlmTask)               │
│                                          │
│ [Allow once] [Allow always for this plan]│
│ [Deny] [Cancel plan]                     │
└──────────────────────────────────────────┘
```

模态阻塞主区交互，但允许切换其他 plan / project（不阻塞整个 app）。

---

## 9. v0.1 验收清单（Desktop）

### 必须

- [ ] Tauri 2.x app 启动，连上 daemon，三栏布局完整
- [ ] Project Tree：显示 active projects，支持创建 / 切换 / 嵌套展开
- [ ] Plan Graph DAG：渲染、状态色、Inspector 抽屉、Output 流式
- [ ] Memory Inbox：双显（嵌入 + 全局）、approve/edit/reject、批量
- [ ] Memory Search：基本检索 UI（结果 + provenance 跳转）
- [ ] Onboarding：三句欢迎屏 + Open folder + 启发识别 + init 引导
- [ ] PermissionAsk 模态阻塞但不锁全局
- [ ] Statusbar：daemon / providers / global mem 三段健康
- [ ] 视觉调性达标（深色基线 / mono 字体 / 间距 / 状态色）
- [ ] First happy path < 90s SLA

### 可选（v0.1 可缺，v0.2 必补）

- [ ] 星座视图切换
- [ ] Plan fork / replay / rewind UI
- [ ] 主题切换 / i18n
- [ ] 桌面通知 / 系统托盘
- [ ] 自定义键位

---

## 10. 不在 v0.1 范围

- 移动端 / Web SaaS（不规划）
- 主题切换（v0.2）
- i18n（v0.2，硬编码 zh-CN + en-US 双轨）
- Plan fork 视觉之外的 fork 操作 UI（v0.2）
- Replay / Rewind / Time-travel UI（v0.2）
- 系统托盘 / 通知（v0.2）
- 多窗口 / 多 daemon 切换（v0.3+）
- 设置项的 GUI 编辑器（v0.2，v0.1 仅展示，编辑走 config.toml）

---

**FROZEN. Desktop UI v0.1 冻结于 2026-05-24。**
