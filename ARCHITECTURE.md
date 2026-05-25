# ARCHITECTURE — Blues v0.1 总架构

> Blues v0.1 的工程总图与模块边界。
> 把已冻的协议层 + UI 形态 + 各引擎决策汇总成一张可执行蓝图。
> 此后 `#5 workspace 骨架` / `#7 proto` / `#8 memory` 等实现任务，均以本文为契约。
>
> 状态：**FROZEN** · 最后修订：2026-05-24

---

## 0. 立场重申

来自 `BLUES_VALUES.md`：

> Blues 把 AI 协作从瞬时态升级到时间态。
> 死磕五件套：Daemon 协议 / 记忆引擎 / Plan Graph / Project Identity / CubeSandbox 集成。

整个架构服务于这五件套。任何模块若与五件套无关，**不在 v0.1**。

---

## 1. 工程总图

### 1.1 三态同构（Client Surfaces）

```
┌────────────────────┐  ┌────────────────────┐  ┌────────────────────┐
│ Desktop (Tauri)    │  │ VS Code Ext        │  │ CLI                │
│ desktop/           │  │ vscode-ext/        │  │ blues-cli (crate)  │
│ ── 主交付（U1=A）  │  │ ── 轻补位（U6=B）  │  │ ── 必备            │
└──────────┬─────────┘  └──────────┬─────────┘  └─────────┬──────────┘
           │                       │                       │
           │ Tauri command         │ CLI 子进程 (--json)   │ gRPC
           │  ↓                    │  ↓                    │  ↓
           │ blues-protocol client │ blues CLI             │ direct
           └───────────────────────┴───────────────────────┘
                                   │
                          gRPC over UDS / Named Pipe
                                   │
                          ┌────────▼─────────┐
                          │  blues-daemon    │
                          └──────────────────┘
```

- Desktop 用 Tauri command 在 Rust 侧调 `blues-protocol` 客户端 → gRPC（决策 G2 = A）
- VS Code Ext **不**自己说 gRPC，全部走 `blues` CLI 子进程 + JSON（vscode.md §2）
- CLI 直接连 daemon

### 1.2 Cargo Workspace（12 个 crate）

```
crates/
├── blues-core/          # 基础类型 / trait / error / ULID
├── blues-protocol/      # proto 编译 / gRPC stub / MCP schema
├── blues-config/        # 配置加载链 + secrets
├── blues-memory/        # 记忆引擎（最大战场）
├── blues-model/         # 路由 + provider 抽象 + 各家实现
├── blues-agent/         # agent 执行器（ReAct loop / tool call）
├── blues-plan/          # plan graph DAG + 状态机 + fork（v0.2 完整）
├── blues-sandbox/       # 沙箱抽象 + host/worktree/cubesandbox 三档
├── blues-skill/         # skill 加载 + 调度（兼容 Claude Code 格式）
├── blues-daemon/        # 守护进程（gRPC server + 持久化 + 事件总线）
├── blues-cli/           # CLI 客户端
└── blues-mcp/           # MCP server（独立 binary）

apps/
├── desktop/             # Tauri 应用（src-tauri/ 是 Rust crate，不在 workspace 主图）
└── vscode-ext/          # TypeScript 扩展（不在 workspace）
```

### 1.3 Crate 依赖图

```
                            blues-core
                                ▲
        ┌──────┬──────┬─────┬──┴──┬──────┬──────┬──────┐
        │      │      │     │     │      │      │      │
   protocol config memory model agent  plan  sandbox skill
        ▲      ▲      ▲     ▲     ▲      ▲      ▲      ▲
        │      │      │     │     │      │      │      │
        └──────┴──────┴─────┴──┬──┴──────┴──────┴──────┘
                               │
                          blues-daemon
                               ▲
                       ┌───────┴───────┐
                       │               │
                   blues-cli       blues-mcp
```

**铁律**：

- `blues-core` 不依赖任何 blues-* 内部 crate（仅依赖标准库 + 第三方）
- 每个引擎 crate（`memory` / `model` / `agent` / `plan` / `sandbox` / `skill`）只依赖 `blues-core`，**不互相依赖**
- 引擎之间的协作通过 `blues-daemon` 编排，不通过 crate 直接 import
- `blues-cli` / `blues-mcp` 都是 daemon 的 facade，不直接 import 引擎 crate（只通过 `blues-protocol` 客户端调 daemon）

> 这条架构铁律保证：
> 1. 引擎可以独立测试 / 独立替换
> 2. 没有循环依赖
> 3. 协议层是唯一的 facade，未来加 desktop / vscode / cursor 不会污染引擎

---

## 2. 各 crate 职责边界

### 2.1 `blues-core`

**职责**：所有 crate 共用的基础类型与 trait。

**导出**：

```rust
// types
pub struct ProjectId(Ulid);
pub struct ProjectSlug(String);
pub struct PlanId(Ulid);
pub struct NodeId(Ulid);
pub struct FactId(Ulid);
pub struct EpisodeId(Ulid);
pub struct InboxItemId(Ulid);

pub enum NodeStatus { Pending, Running, Paused, Done, Error, Forked }
pub enum PlanStatus { Pending, Running, Paused, Done, Error, Cancelled }
pub enum MemoryType { Episodic, Semantic, Procedural }

pub struct Capability { /* fs.read / net.fetch / ... */ }
pub struct Provenance { sources: Vec<Source>, chain: Vec<Step> }

// errors
pub enum BluesError {
    NotFound, PermissionDenied, Conflict,
    InvalidArgument, ProtocolMismatch, Internal, Unavailable,
}
pub type Result<T> = std::result::Result<T, BluesError>;

// async trait helpers
pub trait EventEmitter { fn emit(&self, ev: Event); }
```

**不放**：业务逻辑、IO、网络、配置加载。

### 2.2 `blues-protocol`

**职责**：proto 编译、gRPC stub、MCP schema、protocol_version 常量。

**关键文件**：

```
proto/
├── blues.proto              # gRPC service 全量
├── cli_outputs.proto        # CLI --json 输出 schema（cli.md §7）
└── mcp.proto                # MCP 工具参数 schema（与 blues.proto 对齐）
```

**导出**：

- `tonic` 生成的 client / server stub
- MCP schema 的 JSON 描述（给 mcp serve 用）
- `PROTOCOL_VERSION: u32 = 1`
- 协议版本不兼容时的 `UpgradeRequired` 错误构造

**不放**：业务逻辑（proto 只描述 wire 协议，行为在 daemon 实现）。

### 2.3 `blues-config`

**职责**：配置加载链 + secrets 管理。

**加载顺序**（从 `protocol-and-project.md §5.1`）：

```
CLI flag > BLUES_* env > <project>/.blues/*.toml > ~/.blues/config.toml > defaults
```

**Secrets 策略**：

```rust
pub trait SecretStore {
    fn get(&self, key: &str) -> Option<String>;
}

// 实现优先级
// 1. OSKeychainStore（macOS Keychain / Win Credential Manager / Linux Secret Service）
// 2. EnvVarStore（fallback，从 api_key_env 读环境变量）
// 3. PlaintextFileStore（仅在 keychain 不可用时启用，警告日志）
```

**不放**：业务逻辑、网络。

### 2.4 `blues-memory`（最大战场）

**职责**：认知三层 + 写入漏斗 + 巩固 + 上下文编译器。

**子模块**：

```
blues-memory/src/
├── lib.rs
├── types.rs                    # Fact / Episode / Procedure / Relation / Inbox
├── store/
│   ├── sqlite.rs               # 关系存储（rusqlite + WAL）
│   ├── vector.rs               # 向量索引（hnsw 或 lancedb，待 §4.3 决策）
│   └── fts.rs                  # 全文索引（sqlite FTS5）
├── ingest/
│   ├── extractor.rs            # 从 chat / file 抽取候选
│   └── inbox.rs                # inbox 候选区
├── recall/
│   ├── multi_route.rs          # 多路召回（vector + fts + relation）
│   ├── ranker.rs               # rerank
│   └── compiler.rs             # 预算化上下文编译器
├── consolidate/
│   ├── nightly.rs              # 巩固任务
│   ├── decay.rs                # 衰减 + 失效
│   └── procedure.rs            # 从重复 episode 提炼 procedure
└── api.rs                      # 对 daemon 暴露的 trait
```

**对外 trait**：

```rust
pub trait MemoryEngine: Send + Sync {
    async fn save(&self, project: ProjectId, write: MemoryWrite) -> Result<MemoryRef>;
    async fn query(&self, project: ProjectId, q: MemoryQuery) -> Result<MemoryResults>;
    async fn compile_context(&self, req: ContextRequest) -> Result<CompiledContext>;
    async fn list_inbox(&self, project: ProjectId, filter: InboxFilter) -> Result<InboxItems>;
    async fn approve_inbox(&self, item: InboxItemId, edit: Option<Edit>) -> Result<FactId>;
    async fn reject_inbox(&self, item: InboxItemId) -> Result<()>;
    async fn blame(&self, fact: FactId) -> Result<Provenance>;
    async fn consolidate(&self, project: ProjectId, mode: ConsolidateMode) -> Result<()>;
}
```

**写入漏斗**（**默认走 inbox，不直接落库**）：

```
External (chat / file / MCP save / agent extraction)
        ↓
   Inbox (待审批，含 confidence + provenance)
        ↓ approve / edit / reject
   SQLite + Vector + FTS
        ↓
   定期 consolidate（衰减、合并、procedure 提炼）
```

**v0.1 必须**：

- episodic / semantic / procedural 三层 schema 与 CRUD
- inbox 写入漏斗
- 多路召回（vector + fts，relation 留 v0.2）
- 预算化上下文编译器（dedup + 排序 + token 预算）
- blame（provenance map）

**v0.1 可选**：

- nightly consolidate（v0.1 给手动 `blues mem consolidate`，自动调度 v0.2）
- procedure 提炼（v0.2）

### 2.5 `blues-model`

**职责**：模型路由 + provider 抽象。

**子模块**：

```
blues-model/src/
├── lib.rs
├── provider/
│   ├── mod.rs                  # ModelProvider trait
│   ├── openai_compat.rs        # 通用 OpenAI 兼容（kiro / ollama / 自建）
│   ├── anthropic.rs
│   └── stub.rs                 # 测试 stub
├── router/
│   ├── mod.rs                  # 路由策略 trait
│   ├── smart.rs                # smart preset（能力匹配 + 成本）
│   ├── economy.rs              # economy preset（最便宜可用）
│   └── performance.rs          # performance preset（最强模型）
├── usage.rs                    # token / cost 累计
└── api.rs
```

**对外 trait**：

```rust
pub trait ModelEngine: Send + Sync {
    async fn list(&self) -> Result<ModelList>;
    async fn chat(&self, req: ChatRequest) -> Result<ChatStream>;
    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse>;
    async fn usage(&self, query: UsageQuery) -> Result<UsageReport>;
    async fn health(&self, provider: &str) -> Result<bool>;
}
```

**v0.1 必须**：

- OpenAI 兼容（cover kiro / ollama / vllm / 任何兼容端）
- Anthropic 原生
- 三档路由 preset（smart / economy / performance）
- streaming chat（含 tool call 协议）
- embedding（默认 ollama nomic-embed-text）

**v0.1 不做**：

- 多 provider fallback（v0.2，先靠用户手动切 preset）
- 自动降级 / 熔断（v0.2）

### 2.6 `blues-agent`

**职责**：单 agent 执行器。Plan 中每个 LlmTask 节点对应一个 agent run。

**子模块**：

```
blues-agent/src/
├── lib.rs
├── react.rs                    # ReAct loop（thought / action / observation）
├── tool_call.rs                # 工具调用协议（与 model provider tool spec 对齐）
├── prompt.rs                   # 系统 prompt 模板
├── budget.rs                   # token 预算 + 超限处理
└── api.rs
```

**对外 trait**：

```rust
pub trait AgentEngine: Send + Sync {
    async fn run(&self, req: AgentRequest) -> Result<AgentStream>;
}

pub struct AgentRequest {
    pub plan_id: PlanId,
    pub node_id: NodeId,
    pub project: ProjectId,
    pub prompt: String,
    pub context: CompiledContext,    // from blues-memory
    pub tools: Vec<ToolSpec>,        // from blues-skill / blues-sandbox
    pub model_hint: Option<String>,
    pub budget: Budget,
}
```

**关键约束**：

- 不直接持有 model / memory / sandbox 实例，全部通过 daemon 传入的 trait object（DI）
- 所有外部副作用（fs / net / shell）必须通过 sandbox 派发的 tool 调用
- 流式输出每个 step 经 `EventEmitter` → daemon → 客户端

### 2.7 `blues-plan`

**职责**：Plan Graph DAG 引擎、状态机、Fork / Replay（v0.2）。

**子模块**：

```
blues-plan/src/
├── lib.rs
├── graph.rs                    # 节点 / 边 / 拓扑
├── state.rs                    # PlanState 持久化（写 ~/.blues/state/plans/）
├── scheduler.rs                # 节点调度（拓扑 + 依赖 + 并发）
├── machine.rs                  # 状态机（pending → running → done / error / paused）
├── fork.rs                     # v0.2: fork 实现
├── replay.rs                   # v0.2: replay
└── api.rs
```

**对外 trait**：

```rust
pub trait PlanEngine: Send + Sync {
    async fn create(&self, req: CreatePlanReq) -> Result<Plan>;
    async fn start(&self, plan: PlanId) -> Result<PlanHandle>;
    async fn pause(&self, plan: PlanId) -> Result<()>;
    async fn resume(&self, plan: PlanId) -> Result<()>;
    async fn cancel(&self, plan: PlanId) -> Result<()>;
    async fn edit_node(&self, req: EditNodeReq) -> Result<Node>;
    async fn inject(&self, req: InjectReq) -> Result<()>;
    async fn state(&self, plan: PlanId) -> Result<PlanState>;
    // v0.2:
    async fn fork(&self, plan: PlanId, node: NodeId) -> Result<PlanId>;
    async fn replay(&self, plan: PlanId, node: NodeId) -> Result<PlanHandle>;
    async fn rewind(&self, plan: PlanId, node: NodeId) -> Result<()>;
}
```

**v0.1 必须**：

- 创建 / 启动 / 暂停 / 恢复 / 取消
- 节点 edit / inject
- 状态持久化（daemon crash recovery 的根基）
- 拓扑调度 + 并发执行
- 完整事件流发到 EventBus

**v0.1 不做**：

- fork / replay / rewind / merge（v0.2）
- diff（v0.2）
- 子图 (Subgraph) 的嵌套（v0.2）

### 2.8 `blues-sandbox`

**职责**：沙箱抽象 + 三档 backend。

**子模块**：

```
blues-sandbox/src/
├── lib.rs
├── api.rs                      # SandboxBackend trait
├── host.rs                     # backend: 直接在 host 跑（trust mode）
├── worktree.rs                 # backend: git worktree + 进程隔离
├── cube.rs                     # backend: cubesandbox（MicroVM）
├── capability.rs               # capability 检查 + 审计
└── policy.rs                   # 加载 .blues/policy.toml
```

**三档 backend**：

| Backend | 隔离强度 | 启动开销 | 适用 |
|---|---|---|---|
| `host` | 无 | < 10ms | 用户显式信任 / 只读探索 |
| `worktree` | 进程级 + git worktree | < 100ms | 默认推荐 |
| `cubesandbox` | MicroVM + eBPF | < 60ms（cube 自己 SLA） | 最高安全 / 多实例并发 |

**对外 trait**：

```rust
pub trait SandboxBackend: Send + Sync {
    async fn spawn(&self, spec: SandboxSpec) -> Result<SandboxHandle>;
    async fn exec(&self, h: &SandboxHandle, cmd: ExecCmd) -> Result<ExecResult>;
    async fn fs_op(&self, h: &SandboxHandle, op: FsOp) -> Result<FsOpResult>;
    async fn snapshot(&self, h: &SandboxHandle) -> Result<SnapshotId>;     // v0.2
    async fn restore(&self, snap: SnapshotId) -> Result<SandboxHandle>;    // v0.2
    async fn destroy(&self, h: SandboxHandle) -> Result<()>;
}
```

**Capability 拦截**：

每次 tool call → sandbox 检查 capability → 如未授权 → 触发 `PermissionAsk` 事件 → 用户决策 → 缓存策略。

**v0.1 必须**：

- host backend（裸跑，权限 ask 兜底）
- worktree backend（默认）
- capability 检查 + audit log
- policy.toml 加载与覆盖

**v0.1 不做**：

- cubesandbox 集成（v0.2，需要 KVM 环境）
- snapshot / restore（v0.2 配合 plan fork）
- eBPF 网络策略（v0.2 随 cubesandbox）

### 2.9 `blues-skill`

**职责**：Skill 加载 / 启用 / 暴露给 agent。

**子模块**：

```
blues-skill/src/
├── lib.rs
├── format.rs                   # 兼容 Claude Code skill 格式
├── loader.rs                   # 从 ~/.blues/skills/ + <project>/.blues/skills/ 加载
├── registry.rs                 # 在线 skill 列表
├── exec.rs                     # 执行 skill（透传到 sandbox）
└── api.rs
```

**对外 trait**：

```rust
pub trait SkillEngine: Send + Sync {
    async fn list(&self, project: ProjectId) -> Result<Vec<Skill>>;
    async fn install(&self, source: SkillSource) -> Result<Skill>;
    async fn enable(&self, project: ProjectId, name: &str) -> Result<()>;
    async fn disable(&self, project: ProjectId, name: &str) -> Result<()>;
    async fn invoke(&self, req: SkillInvocation) -> Result<SkillResult>;
}
```

**v0.1 必须**：

- 兼容 Claude Code skill 格式（命令式 skill，JSON 描述）
- 加载 + enable / disable
- 通过 sandbox 执行（不能绕开 capability 检查）

**v0.1 不做**：

- 在线 marketplace（v0.3+）
- 签名验证（v0.3+）
- skill 沙箱内执行的细粒度审计（v0.2）

### 2.10 `blues-daemon`

**职责**：所有引擎的编排者，gRPC 服务，状态持久化，事件总线。

**子模块**：

```
blues-daemon/src/
├── main.rs                     # binary 入口
├── lib.rs
├── server/
│   ├── mod.rs                  # tonic gRPC server
│   ├── ipc.rs                  # UDS / Named Pipe 监听
│   ├── handler/                # 每个 RPC 一个 handler
│   │   ├── hello.rs
│   │   ├── project.rs
│   │   ├── memory.rs
│   │   ├── plan.rs
│   │   ├── model.rs
│   │   └── stream.rs
│   └── auth.rs                 # 单 OS 用户检查（D3 = A）
├── state/
│   ├── manager.rs              # ActiveContext / projects / plans
│   ├── persist.rs              # 写 ~/.blues/state/
│   └── recovery.rs             # crash recovery（D2 = A）
├── eventbus/
│   ├── mod.rs                  # tokio::broadcast based event bus
│   └── filter.rs               # SubscribeEvents 过滤
├── lifecycle/
│   ├── start.rs                # on-demand spawn
│   ├── stop.rs                 # 优雅停止
│   └── autostart.rs            # OS 自启注册
└── orchestrator/
    ├── mod.rs                  # 编排引擎间的协作（plan node → agent → sandbox → memory）
    └── permission.rs           # PermissionAsk 路由 + 缓存
```

**核心责任**：

- **编排 plan 执行**：plan 调度 → agent run → tool call → sandbox exec → memory ingest → event emit
- **持有 ActiveContext**：跨客户端的"当前 project / plan"共享单例
- **EventBus**：tokio broadcast，所有引擎事件汇总，按 `EventFilter` 分发给订阅客户端
- **Crash Recovery**：重启时把 active plans 改 `paused`（D2 = A）

### 2.11 `blues-cli`

**职责**：CLI 客户端。详见 `docs/ui/cli.md`。

**子模块**：

```
blues-cli/src/
├── main.rs
├── cmd/
│   ├── daemon.rs
│   ├── project.rs
│   ├── memory.rs
│   ├── plan.rs
│   ├── model.rs
│   ├── mcp.rs
│   └── version.rs
├── output/
│   ├── plain.rs
│   ├── rich.rs                 # ratatui
│   └── json.rs
└── client.rs                   # blues-protocol 客户端封装
```

### 2.12 `blues-mcp`

**职责**：MCP server，暴露 4 个 memory 工具给第三方（Claude Code / Cursor / Continue / 任何 MCP host）。

**子模块**：

```
blues-mcp/src/
├── main.rs                     # `blues mcp serve` 子命令的实际实现
├── stdio.rs                    # MCP stdio transport
├── http.rs                     # MCP HTTP transport（v0.2）
├── tools/
│   ├── query.rs                # blues_memory_query
│   ├── compile_context.rs      # blues_compile_context
│   ├── save.rs                 # blues_memory_save（→ inbox）
│   └── inbox_list.rs           # blues_memory_inbox_list
└── ratelimit.rs                # MCP3 速率限制
```

**关键约束**：MCP 工具的实现**不复制**业务逻辑，全部通过 `blues-protocol` 客户端调 daemon 的对应 RPC。MCP 是**协议适配器**，不是独立服务。

---

## 3. 双层编排（Plan + Agent）

```
┌─────────────────────────────────────────────────────────────┐
│  Plan Layer（DAG，宏观）                                     │
│  ─ 节点（LlmTask / Tool / Subgraph）                         │
│  ─ 边（依赖关系）                                            │
│  ─ 状态（pending / running / paused / done / error / forked）│
│  ─ 持久化（崩溃可恢复）                                      │
└──────────────────────────┬──────────────────────────────────┘
                           │ 调度每个节点
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  Agent Layer（ReAct，微观）                                  │
│  ─ 系统 prompt 拼装（含 compiled context）                   │
│  ─ thought / action / observation 循环                       │
│  ─ tool call 派发到 sandbox / skill                          │
│  ─ 流式输出经事件总线广播                                    │
└─────────────────────────────────────────────────────────────┘
```

**为什么分两层**：

- Plan = "做什么 + 顺序"（用户可视、可分叉、可回放）
- Agent = "怎么做单步"（一次 LLM 调用 + tool 链）

把这两层混在一起会失去 fork / replay 能力——这是与 Codex / Antigravity / Claude Code 的根本区别。

**v0.1 范围**：

- Plan 静态创建（用户给 intent，daemon 一次性规划出节点）
- Plan 动态扩展（agent 在节点内可建议新节点，daemon 加入 DAG）—— v0.2
- 子图嵌套（Subgraph 节点 = 一个嵌套 plan）—— v0.2

---

## 4. 记忆引擎（认知三层）

### 4.1 三层 schema

| 层 | 含义 | 存储 | 例 |
|---|---|---|---|
| **Episodic** | 时间切片：会话、事件 | SQLite + FTS（全文）+ 向量 | "2026-05-22 14:32 用户问了 logout 流程" |
| **Semantic** | 实体 / 关系 / 事实 | SQLite + 向量 + 关系图 | "auth.ts owns logout flow" |
| **Procedural** | 重复模式提炼的 procedure | SQLite + 调用统计 | "after logout: clear cache, redirect /login" |

> 三层并非相互替代，是**互补**。一次召回多路返回三层结果，由 ranker 决定融合。

### 4.2 写入漏斗

```
External Source
   ↓
Inbox（候选区）
  - 含 confidence
  - 含 provenance（chain）
  - 默认所有 MCP save、agent extraction 都进 inbox
   ↓
User Decision (approve / edit / reject)
   ↓
Confirmed Storage
  - episodic / semantic / procedural 三层之一
  - 写入 SQLite + 向量 + FTS
   ↓
（异步）Consolidation
  - 衰减
  - 合并相似事实
  - 提炼 procedure
```

**关键约束**：写入永远经 inbox（除非用户显式 `blues mem save --no-inbox`，v0.2）。这是 Blues 与 mem0 / letta 等"全自动记忆"工具的核心区别——**用户保有审批权**。

### 4.3 多路召回 + 预算编译器

```
Query
  ↓ ┌────────────────────────────────────┐
    │ Vector retrieval (top_k * 2)       │
    │ FTS retrieval (top_k * 2)          │
    │ Relation traversal (linked facts)  │
    └────────────────┬───────────────────┘
                     ↓
              Rerank（cross-encoder 或 simpler heuristic）
                     ↓
              Dedup + Diversify
                     ↓
              Compile to Context
                - Token budget aware
                - Prov chain attached
                - Sorted by relevance
```

**v0.1 默认**：

- Vector：HNSW（in-process）或 sqlite-vec（同进程，零运维）—— **v0.1 选 sqlite-vec** 减少依赖
- FTS：sqlite FTS5
- Relation：sqlite 表 + 内存 BFS（v0.1 不引专用图库）
- Rerank：简单 score 加权（v0.1 不引 cross-encoder，v0.2 接入 small reranker）

### 4.4 预算编译器

输入：query / budget（tokens）/ model_id（用于 context window 估算）
输出：

```
CompiledContext {
    blocks: Vec<Block> {
        kind: Episodic | Semantic | Procedural,
        text: String,
        provenance: Provenance,
        tokens: usize,
    },
    total_tokens: usize,
    omitted_count: usize,
}
```

约束：

- 永远不超过 budget
- 优先保留 high-confidence + recent + high-relevance
- 必须保留 provenance（agent 可引用 fact ID）

---

## 5. Sandbox 三档

```
host         worktree (默认)        cubesandbox (v0.2)
  │              │                       │
  ▼              ▼                       ▼
进程内        子进程 +              MicroVM +
不隔离         git worktree          eBPF 网络
  │              │                       │
启动 < 10ms   启动 < 100ms          启动 < 60ms（cube SLA）
              快照 = git stash      快照 = MicroVM snapshot
              捕获 fs / 进程        硬件级隔离
```

**Backend 选择**：

```toml
# <project>/.blues/policy.toml
[sandbox]
default = "worktree"            # host | worktree | cubesandbox

[sandbox.per_capability]
"net.fetch"     = "cubesandbox" # 网络访问强制最高隔离
"shell.exec"    = "worktree"
"fs.write"      = "worktree"
"fs.read"       = "host"        # 只读不需要隔离
```

**Capability 与 Backend 解耦**：

- `Capability` 描述工具想做什么（fs.write / net.fetch / ...）
- `Backend` 描述在哪儿做
- `policy.toml` 把两者绑定

每次 tool call → 解析 capability → 查 policy → 选 backend → 执行 / 拦截 / ask。

---

## 6. Model Router

### 6.1 三档 preset

| Preset | 策略 |
|---|---|
| **smart**（默认） | 按 task type 自动路由（chat→middle / code→strong / extraction→cheap） |
| **economy** | 永远用最便宜的可用 model |
| **performance** | 永远用最强 model（cost 不敏感） |

### 6.2 Provider 抽象

```rust
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> ProviderKind;       // OpenAICompat | Anthropic | ...
    async fn list(&self) -> Result<Vec<ModelInfo>>;
    async fn chat(&self, req: ChatRequest) -> Result<ChatStream>;
    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse>;
    async fn health(&self) -> Result<bool>;
}
```

**v0.1 内置 provider**：

| Name | 类型 | 端点 | 用途 |
|---|---|---|---|
| `kiro` | OpenAI 兼容 | `https://kiro.aidong-ai.com` | claude-opus-4-7 中转 |
| `anthropic` | Anthropic 原生 | `https://api.anthropic.com` |
| `ollama` | OpenAI 兼容 | `http://localhost:11434/v1` | 本地 + embedding |

### 6.3 Tool call 协议

统一为 OpenAI tool calling 协议。Anthropic 在 provider 内部做 schema 转换。

---

## 7. 协议层（gRPC + MCP）

详见 `docs/architecture/protocol-and-project.md` §3-4。

**v0.1 协议补丁清单**（来自 `docs/ui/overview.md §7` + `docs/ui/vscode.md §9`）：

写入 `proto/blues.proto` 时一并落：

1. RPC `SetActiveContext(SetActiveContextReq) returns (Empty)`
2. Event `ActiveContextChanged { project_id, plan_id?, by_client_kind }`
3. Event `PlanForked { plan_id, parent_node_id, new_plan_id }`（v0.2 行为，事件先占位）
4. Event `PlanReplayed { plan_id, from_node_id }`（v0.2 行为，事件先占位）
5. `MemoryInboxAdded` 加可选字段 `plan_id` / `node_id`
6. Event `PermissionResolved { plan_id, node_id, request_id, decision, by_client_kind }`

> 这六条触发协议 schema 变更，但版本号仍为 v1（仅新增字段 / 事件，不破坏向后兼容）。

---

## 8. 端到端数据流（一个 plan 的生命周期）

```
1. 用户在 desktop 输入 intent
   "Refactor auth.ts to centralize logout flow"
   ↓
2. desktop → Tauri command → blues-protocol client → daemon CreatePlan
   ↓
3. daemon orchestrator
   ├── 调 blues-model（router=smart, preset=performance）规划节点
   ├── 写 blues-plan：创建 P#5 + 节点 N#1..N#6
   └── emit PlanStateChanged
   ↓
4. daemon 调用 PlanEngine.start
   ├── scheduler 调度 N#1（LlmTask）
   ├── orchestrator 包装 AgentRequest：
   │   ├── 调 blues-memory.compile_context(query=intent, budget=8k)
   │   ├── 拼 system prompt + tools（来自 skill + sandbox capability）
   │   └── 派发到 blues-agent.run
   ├── agent 跑 ReAct loop
   │   ├── tool call → 经 orchestrator → blues-sandbox.exec（host/worktree）
   │   ├── PermissionAsk → emit → 客户端模态 → 用户决策 → resolve
   │   └── 每 step emit NodeOutputDelta
   ├── agent 完成 → 节点状态 done → emit NodeStateChanged
   └── 节点产物经 blues-memory.ingest → 写 inbox
   ↓
5. 期间客户端：
   ├── desktop 渲染 plan graph + inspector + 流式输出
   ├── inbox 收到候选 → 右栏 + 全局
   └── statusbar token usage 滚动
   ↓
6. 节点链跑完 → PlanStateChanged(done) → desktop 通知 + log 归档
   ↓
7. （v0.2）用户 fork 某节点 → daemon PlanForked → desktop 播裂变动效
```

---

## 9. 数据落盘总图

复用 `protocol-and-project.md §6`：

```
~/.blues/
├── config.toml
├── daemon.pid / daemon.sock (或 named pipe)
├── state/
│   ├── plans/<plan_id>.json
│   └── sessions/<session_id>.json
├── logs/
│   ├── daemon.log
│   └── audit.jsonl
├── projects/<project_id>/
│   ├── memory.db (SQLite + FTS)
│   ├── memory.vec (sqlite-vec 表，v0.1 同 db 文件 inline)
│   ├── inbox/
│   └── sessions/
├── global/
│   ├── memory.db
│   └── memory.vec
├── skills/
├── cache/
└── secrets/ (仅 keychain 不可用时)

<project>/.blues/
├── project.toml ✓ commit
├── policy.toml  ✓ commit
├── skills/      ✓ commit
├── plans/       ✓ commit (归档 plan，去敏感)
├── memory.db    ✗ gitignore (inproject 模式下)
├── inbox/       ✗ gitignore
└── sessions/    ✗ gitignore
```

---

## 10. 已冻决策清单（汇总）

### Project（P1-P4，from `protocol-and-project.md`）
- P1=B 边界识别用启发 + 显式覆盖
- P2=C 默认 A，memory 默认 `~/.blues/projects/<id>/`
- P3=A linked 单向
- P4=B 允许嵌套，记忆默认隔离

### Daemon（D1-D3）
- D1=A on-demand 默认
- D2=A crash recovery 把 active plans 设 paused
- D3=A 严格单 OS 用户

### gRPC（G1-G2）
- G1=A protobuf binary
- G2=A Tauri command 桥（不开 grpc-web 端口）

### MCP（MCP1-MCP3）
- MCP1=B v0.1 暴露 query + save（→ inbox），不暴露删除
- MCP2=A 默认 + C fallback
- MCP3 速率限制默认开

### UI（U1-U8，from `docs/ui/overview.md`）
- U1=A Desktop 优先
- U2=B 三栏布局
- U3=A+B Inbox 双显
- U4=A 默认 + B 切换（DAG / 星座）
- U5=C CLI 极简 + `--rich` 切 TUI
- U6=B VS Code Ext 轻量补充
- U7=B 时空裂变视觉锚
- U8=A 即刻 init + 试跑

### 架构（A1-A3，本文新引）
- **A1=A** 引擎间不互相 import，全部经 daemon orchestrator（§1.3 铁律）
- **A2=A** Memory 写入永远经 inbox 漏斗（§4.2）
- **A3=A** Sandbox capability 与 backend 解耦（§5）

---

## 11. v0.1 验收清单（架构层）

实现到位才算 v0.1 架构 ready：

- [ ] 12 个 crate workspace 成功 `cargo build` 通过（任务 #5）
- [ ] `blues-core` 类型与 trait 冻结，无后续破坏性改动
- [ ] `blues-protocol` `proto/blues.proto` 编译通过 + 6 条补丁全落
- [ ] `blues-config` 加载链 + secrets 三种 store 全通
- [ ] `blues-memory` 三层 + inbox + 多路召回 + 预算编译器达可用
- [ ] `blues-model` kiro / anthropic / ollama 三 provider + 三 preset
- [ ] `blues-agent` ReAct loop + tool call + 流式
- [ ] `blues-plan` 全状态机 + 持久化 + 调度
- [ ] `blues-sandbox` host + worktree backend + capability 拦截
- [ ] `blues-skill` 加载 + enable/disable + invoke
- [ ] `blues-daemon` orchestrator + eventbus + crash recovery
- [ ] `blues-cli` 命令骨架全通（cli.md §10）
- [ ] `blues-mcp` 4 工具 + stdio + ratelimit
- [ ] desktop（src-tauri）+ vscode-ext 与 daemon 端到端通

---

## 12. 不在 v0.1 范围

- gRPC over TLS / 远程 daemon（v0.3+）
- gRPC-Web / REST gateway（v0.2+）
- Plan fork / replay / rewind 实现（v0.2，事件 / API 占位）
- CubeSandbox 集成（v0.2）
- 自动 nightly consolidate（v0.2）
- Procedure 自动提炼（v0.2）
- Multi-provider fallback / 熔断（v0.2）
- E2EE Sync / 多设备同步（v0.4+）
- Federation / 多 daemon（v0.5+）
- 完整 VS Code Ext 功能（v0.2，v0.1 仅占位）
- JetBrains / Zed / Cursor 同源扩展（v0.3+）
- 多 OS 用户 daemon（不规划）

---

## 13. 子文档索引

| 文档 | 范围 |
|---|---|
| [`BLUES_VALUES.md`](../../BLUES_VALUES.md) | 项目宪法：定位 / 价值锚 / 反派叙事 / 双语义场 |
| [`docs/architecture/protocol-and-project.md`](./protocol-and-project.md) | 协议层 + 项目模型冻结 |
| [`docs/ui/overview.md`](../ui/overview.md) | UI 形态总览（U1-U8） |
| [`docs/ui/desktop.md`](../ui/desktop.md) | Desktop（Tauri）UI 冻结 |
| [`docs/ui/cli.md`](../ui/cli.md) | CLI UX 冻结 |
| [`docs/ui/vscode.md`](../ui/vscode.md) | VS Code Ext v0.1 占位 |

---

**FROZEN. Blues v0.1 架构冻结于 2026-05-24。**
**接下来：`#3 ROADMAP.md` 或直接 `#5 Cargo workspace 骨架`。**
