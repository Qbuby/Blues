# 协议 + 项目（Protocol & Project）—— v0.1 冻结

> 本文是 Blues v0.1 协议层与项目模型的最终冻结档。
> 所有相关代码、CLI、UI 必须按本文实现，更动需提 RFC。
>
> 状态：**FROZEN** · 最后修订：2026-05-24

---

## 0. 总览

```
        ┌────────────────────┐  ┌────────────────────┐  ┌──────┐
        │ Desktop (Tauri)    │  │ VS Code Ext (v0.3) │  │ CLI  │
        └────────────┬───────┘  └─────────┬──────────┘  └──┬───┘
                     │                    │                │
                     └─── gRPC over UDS / Named Pipe ───────┘
                                       │
                              ┌────────▼─────────┐
                              │  Blues Daemon    │
                              │  (Rust, tokio)   │
                              ├──────────────────┤
                              │ blues-memory     │
                              │ blues-model      │
                              │ blues-agent      │
                              │ blues-sandbox    │
                              │ blues-skill      │
                              │ blues-protocol   │
                              └────────┬─────────┘
                                       │
        ┌──────────────────────────────┴──────────────────────────────┐
        │                  MCP Server (stdio / http)                   │
        │   暴露给 Claude Code / Cursor / Continue / 任何 MCP 客户端   │
        └──────────────────────────────────────────────────────────────┘
```

**核心原则**

1. **Daemon 即真神**：所有状态由 daemon 持有；客户端是无状态壳。
2. **协议双通道**：gRPC 给 Blues 自家客户端（全量能力）；MCP 给第三方（v0.1 仅 memory）。
3. **Project 是身份不是路径**：UUID 优先，路径可变。
4. **本地优先**：默认全部本地，云同步是可选项（v0.4+）。

---

## 1. Project 模型

### 1.1 四层结构

```
ProjectIdentity (uuid，永生，跨设备)
        │ 1:N
ProjectRoot (本机绝对路径，可多个：primary / mirror / worktree / detached)
        │
ProjectContext (语义：记忆、policy、skill 配置、linked_projects)
        │
ProjectSession (运行时态：active plans、open agents，daemon 重启可恢复)
```

### 1.2 ProjectIdentity Schema

```toml
# <project>/.blues/project.toml
[project]
id          = "01HXR8VAB7CQXMK3ZN0E6PWQ72"   # ULID
name        = "client-acme"
slug        = "client-acme"
created_at  = "2026-05-24T01:00:00Z"
schema      = 1                                # 配置文件版本

[[linked]]
project = "shared-skills"                      # 另一个 project 的 slug 或 id
scope   = ["skill", "procedural-memory"]

[[linked]]
project = "lessons-learned"
scope   = ["episodic-memory"]

[memory]
storage = "global"                             # global | inproject
                                               # 默认 global，写到 ~/.blues/projects/<id>/

[policy]
inherit = "default"                            # 继承用户级默认 policy
                                               # 节点/项目级可在 .blues/policy.toml 覆盖
```

### 1.3 ProjectRoot Roles

| Role | 含义 | 谁创建 |
|---|---|---|
| `primary` | 主开发位置 | 用户 init / import |
| `mirror` | 同步过来的副本（多设备） | sync 引擎（v0.4+） |
| `worktree` | git worktree，归属某个 plan | plan 引擎自动 |
| `detached` | 临时挂载（外接硬盘/远程目录） | 用户显式 |

### 1.4 边界识别策略（决策 P1 = B）

进入未注册目录时：

1. 优先检查 `.blues/project.toml`（显式标记）
2. 没有则启发：
   - 找 `.git/` 根
   - 找 `package.json` / `Cargo.toml` / `pyproject.toml` / `go.mod`
   - 取最近一层有标志的目录作候选根
3. desktop / CLI 弹窗："是否初始化为 Blues 项目？" 三选：
   - 初始化（生成 `.blues/project.toml`）
   - 临时模式（不落盘标记，仅本会话）
   - 跳过

### 1.5 Memory 落盘位置（决策 P2 = C，默认 A）

```
默认（global）：
  ~/.blues/projects/<project_id>/
    memory.db
    memory.vec
    memory.fts
    inbox/
    sessions/

可选（inproject，project.toml 显式开）：
  <project>/.blues/
    memory.db          # 注意 git 默认 ignore
    ...
```

`<project>/.blues/.gitignore` 自动生成：忽略 memory.db / sessions / inbox，仅 commit `project.toml` / `policy.toml` / `skills/`。

### 1.6 linked_projects 方向（决策 P3 = A，单向）

- A 在 `linked` 里写 B → A 的 agent 查询时可见 B 的指定 scope 记忆
- B 不会自动看到 A——除非 B 也显式 link A
- 防止跨客户项目污染

scope 取值：
- `episodic-memory`：可读对方的 episode
- `semantic-memory`：可读对方的 entity/relation
- `procedural-memory`：可读对方的 procedure
- `skill`：可调用对方的 skill
- `all`：等价上面四个

### 1.7 Project 嵌套（决策 P4 = B）

允许 monorepo 子目录是独立 project：

```
~/work/big-monorepo/                  # parent project
├── .blues/project.toml               # id = parent
├── apps/
│   ├── frontend/
│   │   └── .blues/project.toml       # id = child-1，独立
│   └── backend/
│       └── .blues/project.toml       # id = child-2，独立
└── packages/
    └── (no .blues/ → 归 parent)
```

**记忆隔离**：默认互不可见。child 显式 `link = ["parent"]` 才能反向查 parent。

**plan 归属**：plan 跑在哪个 project 由 cwd 决定（最近的 .blues/）。

### 1.8 Project 生命周期

```
create → activate → (run plans) → archive → delete
   ↑                                          ↓
   └──── import (from path / manifest) ──────┘
```

- **create**：`blues project init` / desktop 新建
- **activate**：daemon 加入活动列表，监听 `.blues/`
- **archive**：从活动列表移除，资源不删
- **delete**：彻底移除，三次确认（CLI/UI/keyword）
- **import**：`blues project import <path>` 从他人/旧机器接管

---

## 2. Daemon 生命周期

### 2.1 启动模式（决策 D1 = A，on-demand 默认）

| 模式 | 触发 | 命令 |
|---|---|---|
| **on-demand**（默认） | 客户端连接时 spawn | 自动 |
| **manual** | 用户显式 | `blues daemon start` |
| **auto-start** | OS 启动 | `blues daemon enable-autostart` |

启动开销目标：**< 200ms**（Rust + tokio 实测可达）。

### 2.2 Socket 位置

| 平台 | 路径 |
|---|---|
| Linux / macOS | `~/.blues/daemon.sock`（UDS） |
| Windows | `\\.\pipe\blues-daemon-<user>` |

权限：仅当前用户可读写（决策 D3 = A，严格单用户）。

### 2.3 进程文件

```
~/.blues/
├── daemon.pid              # PID 文件
├── daemon.sock             # IPC socket（Linux/macOS）
├── state/                  # 持久化状态（active plans 等）
└── logs/
    ├── daemon.log          # 主日志，按日 rotate
    └── audit.jsonl         # 权限决策审计
```

### 2.4 Crash Recovery & 优雅停止

**优雅停止**：
1. 收到 SIGTERM
2. 拒绝新连接
3. 等待 in-flight tool calls 完成（30s 超时）
4. 持久化 plan 状态到 `state/`
5. 退出

**强杀超时**：30s 后未退出 → SIGKILL

**Crash recovery**（决策 D2 = A）：
- daemon 重启读 `state/`
- 所有 active plans 状态强制改为 `paused`
- 客户端连上后看到 `paused` 状态，由用户决定 resume/cancel
- 不自动 resume，避免重复副作用

### 2.5 客户端连接握手

```
Client                          Daemon
  │ ─── Connect (UDS/pipe) ─────→
  │
  │ ─── HelloRequest             │
  │     client_kind, version,    │
  │     protocol_version ────────→
  │
  │ ←──── HelloResponse          │
  │       server_version,        │
  │       supported_features,    │
  │       active_projects[],     │
  │       session_token
  │
  │ ─── Subscribe(events) ──────→ (可选)
  │
  │ ←──── EventStream (server) ──│
```

- **协议版本握手**：`protocol_version` 不兼容 → daemon 返 `UpgradeRequired` 并指示升级方向
- **能力协商**：server 返回 feature flags（如 `sync_enabled` / `cubesandbox_available`）
- **session_token**：客户端持有，断线重连可拿来 replay 错过的事件
- **多客户端**：同一 daemon 可同时连多个客户端（desktop + cli + mcp）

### 2.6 健康检查与控制

| 命令 | 行为 |
|---|---|
| `blues daemon status` | UDS ping，返回 PID/uptime/active counts |
| `blues daemon stop` | SIGTERM 优雅停止 |
| `blues daemon restart` | stop + start |
| `blues daemon logs` | tail logs/daemon.log |
| `blues daemon enable-autostart` | 注册 OS 自启（launchd/systemd/任务计划） |

---

## 3. gRPC 服务清单（v0.1）

### 3.1 编码与 transport（决策 G1 = A，G2 = A）

- **编码**：protobuf binary（默认）
- **Transport**：gRPC over UDS（Linux/macOS）/ Named Pipe（Windows）
- **Tauri 桥**：Tauri command 调用 daemon 客户端（Rust），前端通过 invoke 触发；不暴露 grpc-web 端口
- **远程访问**（v0.3+）：可选启用 gRPC over TLS，绑指定端口

### 3.2 服务定义

```proto
service BluesService {
  // ── Daemon 元 ─────────────────────────
  rpc Hello       (HelloRequest)        returns (HelloResponse);
  rpc Health      (Empty)               returns (HealthStatus);
  rpc Shutdown    (ShutdownRequest)     returns (Empty);

  // ── Project ───────────────────────────
  rpc ListProjects        (Empty)              returns (ProjectList);
  rpc GetProject          (ProjectRef)         returns (Project);
  rpc CreateProject       (CreateProjectReq)   returns (Project);
  rpc UpdateProjectConfig (UpdateProjectReq)   returns (Project);
  rpc ImportProject       (ImportProjectReq)   returns (Project);
  rpc ArchiveProject      (ProjectRef)         returns (Empty);

  // ── Memory ────────────────────────────
  rpc QueryMemory         (MemoryQuery)        returns (MemoryResults);
  rpc SaveMemory          (MemoryWrite)        returns (MemoryRef);
  rpc ListInbox           (InboxFilter)        returns (InboxItems);
  rpc ApproveInboxItem    (InboxAction)        returns (Empty);
  rpc CompileContext      (ContextRequest)     returns (CompiledContext);

  // ── Plan / Agent ──────────────────────
  rpc CreatePlan          (CreatePlanReq)      returns (Plan);
  rpc StartPlan           (PlanRef)            returns (PlanHandle);
  rpc PausePlan           (PlanRef)            returns (Empty);
  rpc ResumePlan          (PlanRef)            returns (Empty);
  rpc CancelPlan          (PlanRef)            returns (Empty);
  rpc EditNode            (EditNodeReq)        returns (Node);
  rpc InjectMessage       (InjectReq)          returns (Empty);
  rpc GetPlanState        (PlanRef)            returns (PlanState);

  // ── Model ─────────────────────────────
  rpc ListModels          (Empty)              returns (ModelList);
  rpc GetUsage            (UsageQuery)         returns (UsageReport);

  // ── Streams ───────────────────────────
  rpc SubscribeEvents     (EventFilter)        returns (stream Event);
  rpc StreamAgentOutput   (NodeRef)            returns (stream ChatEvent);
}
```

### 3.3 事件类型（SubscribeEvents）

```
event PlanStateChanged    { plan_id, old_status, new_status }
event NodeStateChanged    { plan_id, node_id, status, error? }
event NodeOutputDelta     { plan_id, node_id, chat_event }
event MemoryInboxAdded    { project_id, item_id, summary, confidence }
event PermissionAsk       { plan_id, node_id, capability, args, request_id }
event TokenUsage          { plan_id, node_id, provider, in, out, cost_usd }
event ProjectActivated    { project_id }
event ProjectArchived     { project_id }
event DaemonShuttingDown  { eta_ms }
```

客户端按 `EventFilter` 订阅子集，避免广播风暴。

### 3.4 错误模型

所有 RPC 返回标准 `tonic::Status`，错误码使用 `BluesError` 映射：

| BluesError | gRPC Code |
|---|---|
| `NotFound`         | `NOT_FOUND` |
| `PermissionDenied` | `PERMISSION_DENIED` |
| `Conflict`         | `ABORTED` |
| `InvalidArgument`  | `INVALID_ARGUMENT` |
| `ProtocolMismatch` | `FAILED_PRECONDITION` |
| `Internal`         | `INTERNAL` |
| `Unavailable`      | `UNAVAILABLE` |

错误 detail 用 `google.rpc.ErrorInfo` 携带结构化信息（如 missing fields、suggested action）。

---

## 4. MCP Server（v0.1 仅 Memory）

### 4.1 战略意义

MCP 是 Blues 的**特洛伊木马**：让所有非 Blues 客户端（Claude Code / Cursor / Continue / 任何 MCP host）能用上 Blues 的记忆引擎。

- 用户先尝到记忆引擎甜头 → 反向引流到完整 Blues
- Blues 记忆 = 跨工具的"知识根"
- 这是 Codex / Antigravity / CC 做不到的（私有记忆）

### 4.2 暴露范围（决策 MCP1 = B，查询 + 写入但走 inbox）

```
Tool: blues_memory_query
  描述: 多路召回，返回相关记忆 + provenance map
  参数:
    project_ref      (string)  # ulid 或 slug，必填
    query            (string)
    top_k            (int, default 8)
    scope            (enum: project | linked | global, default project)

Tool: blues_compile_context
  描述: 编译给定任务的预算化上下文（已 dedup、已排版、含来源）
  参数:
    project_ref         (string)
    task_description    (string)
    token_budget        (int)
    model_id            (string, optional)  # 用于上下文窗对齐

Tool: blues_memory_save
  描述: 写入记忆（默认进 inbox 等审批，不直接落库）
  参数:
    project_ref     (string)
    content         (string)
    type            (enum: episodic | semantic | procedural)
    source          (string, optional)
    confidence      (float 0-1, optional)

Tool: blues_memory_inbox_list
  描述: 列出待审批的记忆候选
  参数:
    project_ref   (string)
    limit         (int, default 20)
```

**v0.1 不暴露**：
- 删除（避免 MCP 误删）
- 巩固触发
- Plan / Agent 操作

### 4.3 Project 标识（决策 MCP2 = A 默认 + C fallback）

MCP 客户端如何告诉 daemon "我现在是哪个 project"：

1. **MCP 配置时硬编码**（推荐）：
   ```json
   {
     "mcpServers": {
       "blues": {
         "command": "blues",
         "args": ["mcp", "serve", "--project", "<project-id-or-slug>"]
       }
     }
   }
   ```
2. **工具调用时传 `project_ref`**：每次调用都带，覆盖 #1
3. **自动检测（fallback）**：MCP host 启动时如果 cwd 在某个已注册 project 内，自动用那个

### 4.4 速率限制与配额（决策 MCP3）

默认开启：
- 每个 MCP client 连接：100 RPC/min（query / compile_context）
- 每个 MCP client 连接：30 saves/min（写入更慢）
- 全局：1000 RPC/min（防御）
- 配额可在 `~/.blues/config.toml` 调

超限返回 MCP 错误码 `rate_limited`，含 `retry_after_ms`。

### 4.5 MCP 与 gRPC 的关系

```
[3rd-party MCP host]    [Blues client]
       ↓ MCP                ↓ gRPC
       └──→ Daemon (single business logic) ←──┘
                  ↓
            blues-memory
```

**同一个底层 service**，不同协议层 facade。MCP 工具的参数语义必须和 gRPC RPC 完全对齐——任何 RPC 行为变更，MCP 自动跟进。

---

## 5. 配置加载

### 5.1 优先级（高 → 低）

1. CLI flag
2. 环境变量 `BLUES_*`
3. `<project>/.blues/project.toml` + `<project>/.blues/policy.toml`
4. `~/.blues/config.toml`
5. Built-in defaults

每层只覆盖自己声明的字段，**不重写整个对象**。

### 5.2 关键配置块

```toml
# ~/.blues/config.toml

[daemon]
mode = "on-demand"            # on-demand | manual | autostart

[memory]
embedding_provider = "ollama"
embedding_model    = "nomic-embed-text"
extraction_threshold = 0.7
auto_consolidate = "nightly"

[[model.providers]]
name = "kiro"
type = "openai-compat"
endpoint = "https://kiro.aidong-ai.com"
api_key_env = "KIRO_API_KEY"

[[model.providers]]
name = "anthropic"
type = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"

[[model.providers]]
name = "ollama"
type = "openai-compat"
endpoint = "http://localhost:11434/v1"

[router]
preset = "smart"               # smart | economy | performance

[mcp]
enabled = true
rate_limit_query_per_min = 100
rate_limit_save_per_min  = 30
```

### 5.3 Secrets

- API key **不写在配置文件**
- 默认走 OS keychain（macOS Keychain / Win Credential Manager / Linux Secret Service）
- fallback：`api_key_env` 指向环境变量
- 永远不落明文磁盘

---

## 6. 数据落盘总图

```
~/.blues/
├── config.toml                      用户全局配置
├── daemon.pid
├── daemon.sock                      （或 named pipe）
├── state/                           daemon 持久化状态
│   ├── plans/<plan_id>.json
│   └── sessions/<session_id>.json
├── logs/
│   ├── daemon.log
│   └── audit.jsonl
├── projects/
│   └── <project_id>/                （memory storage = global 时）
│       ├── memory.db
│       ├── memory.vec
│       ├── memory.fts
│       ├── inbox/
│       └── sessions/
├── global/                          跨项目全局图谱
│   ├── memory.db
│   └── memory.vec
├── skills/                          安装的 skill（兼容 Claude Code 格式）
│   └── <skill-name>/
├── cache/
│   ├── embeddings/
│   └── responses/
└── secrets/                         （仅在 OS keychain 不可用时使用）

<project>/.blues/                    项目内（git 友好）
├── project.toml                     ✓ commit
├── policy.toml                      ✓ commit
├── skills/                          ✓ commit
├── plans/                           ✓ commit（归档的 plan，去敏感）
├── memory.db                        ✗ gitignore（如开 inproject）
├── inbox/                           ✗ gitignore
└── sessions/                        ✗ gitignore
```

`<project>/.blues/.gitignore`：

```
memory.db
memory.vec
memory.fts
inbox/
sessions/
*.tmp
```

---

## 7. CLI 命令骨架（v0.1）

```
# Daemon
blues daemon {start, stop, status, restart, logs, enable-autostart}

# Project
blues project {init, list, info, archive, delete, import}

# Memory
blues mem query <text> [--scope project|linked|global] [--top-k N]
blues mem save <text> [--type episodic|semantic|procedural]
blues mem inbox {list, approve, reject, edit}
blues mem blame <fact-id>

# Plan
blues plan new <intent>
blues plan list
blues plan log <plan-id>
blues plan pause <plan-id>
blues plan resume <plan-id>
blues plan cancel <plan-id>
blues plan inject <plan-id> <node-id> <message>
# v0.2:
blues plan fork <plan-id> <node-id>
blues plan replay <plan-id> <node-id>
blues plan rewind <plan-id> <node-id>
blues plan diff <a> <b>

# Model
blues model list
blues model usage [--since date]

# MCP
blues mcp serve [--project <id|slug>]

# Misc
blues version
blues --help
```

详细 CLI UX 见 `docs/ui/cli.md`（议题 4 产出）。

---

## 8. 已冻结决策清单

### Project（P1-P4）
- **P1 = B**：边界识别用启发 + 显式覆盖
- **P2 = C 默认 A**：memory 默认 `~/.blues/projects/<id>/`，可选改 inproject
- **P3 = A**：linked_projects 单向
- **P4 = B**：允许嵌套，记忆默认隔离 + 可显式 link

### Daemon（D1-D3）
- **D1 = A**：on-demand 默认启动
- **D2 = A**：crash recovery 把 active plans 设为 paused，不自动 resume
- **D3 = A**：严格单 OS 用户

### gRPC（G1-G2）
- **G1 = A**：protobuf binary 编码
- **G2 = A**：Tauri command（不开 grpc-web 端口）

### MCP（MCP1-MCP3）
- **MCP1 = B**：v0.1 暴露查询 + 写入（save 走 inbox），不暴露删除
- **MCP2 = A 默认 + C fallback**：MCP 配置硬编码 project，缺则 cwd 自动检测
- **MCP3**：默认开启速率限制

---

## 9. v0.1 协议交付物

实现到位才算 v0.1 协议层 ready：

1. ✅ `proto/blues.proto` 编译通过，`blues-protocol` crate 生成 stub
2. ✅ `blues-daemon` 启动后能 `Hello` / `Health` / `Shutdown`
3. ✅ `blues-cli` 实现 `blues daemon {start,stop,status}` + `blues project {init,list,info}`
4. ✅ Project 创建 / 激活 / 嵌套识别 / .blues/project.toml schema 校验
5. ✅ MCP server 启动 `blues mcp serve --project <ref>`，暴露 4 个 memory 工具（接口可先 stub，等 memory crate 完成）
6. ✅ 跨平台：Linux + macOS UDS / Windows Named Pipe 至少一个端到端通

---

## 10. 不在 v0.1 范围

明确划出去，避免范围蔓延：

- gRPC over TLS / 远程 daemon（v0.3+）
- gRPC-Web / REST gateway（v0.2+）
- E2EE Sync（v0.4+）
- Federation / 多 daemon（v0.5+）
- MCP 暴露 Plan / Agent 操作（v0.2+）
- MCP 删除/巩固/管理类工具（v0.2+）
- 多 OS 用户 daemon（不规划）

---

## 附录 A：术语对照

| 术语 | 含义 |
|---|---|
| Project | 用户工作单元，UUID 身份，多 root 多设备 |
| ProjectRoot | Project 在某机器上的某个本地路径 |
| Plan | 一次完整的 AI 协作任务（DAG） |
| Node | Plan 中的一个执行单元（LlmTask / Tool / Subgraph） |
| Episode | 一次会话或事件的时间切片（记忆类型） |
| Memory Inbox | 待审批的记忆候选区 |
| Provenance | 记忆/输出的来源追溯链 |
| Capability | 工具声明的副作用类别（fs.read / net.fetch / ...） |
| Daemon | Blues 的本地服务进程 |
| Backend | Sandbox 的执行后端（host / worktree / cubesandbox） |

## 附录 B：未来 RFC 触点

任何变动以下事项都需走 RFC：

- 改 project.toml schema（schema version bump）
- 改 gRPC service 方法签名
- 改 MCP 工具参数
- 改配置加载优先级
- 改 `~/.blues/` 目录布局

---

**FROZEN. v0.1 起点。**
