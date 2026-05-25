# CLI UX —— v0.1 冻结

> Blues CLI 必须能跑端到端 daemon / project / memory / plan / model 全套操作。
> 默认极简（grep-friendly），强交互场景自动开 TUI（决策 U5 = C）。
> 上位文档：[`overview.md`](./overview.md)
>
> 状态：**FROZEN** · 最后修订：2026-05-24

---

## 1. 叙事风格（决策 U5 = C）

### 1.1 默认极简

- 输出**只对人有用 + 对 pipe 友好**
- 默认无颜色装饰（除非 stdout 是 TTY）
- 默认无 spinner（除非 TTY 且操作 > 500ms）
- 永远说人话：`blues plan fork`，不是 `blues star navigate`
- 错误退出码语义化（参 §6）

### 1.2 三种输出模式

| 模式 | 触发 | 形态 |
|---|---|---|
| **plain（默认）** | TTY，无 flag | 纯文本，列对齐，pipe 无残留色码 |
| **rich** | `--rich` 或部分子命令默认开 | TUI（ratatui），交互式，watch 模式 |
| **json** | `--json` | 一次性输出 JSON，机读，schema 见 §7 |

### 1.3 自动 TUI 子命令

以下子命令默认进 TUI（除非 `--no-rich` 或非 TTY）：

- `blues plan watch <plan-id>` — 实时跟踪 plan 节点状态
- `blues mem inbox` — 翻牌式审批
- `blues daemon logs --follow` — 滚动日志高亮

其他子命令默认 plain，加 `--rich` 才进 TUI。

### 1.4 颜色与图标规则

- TTY 默认上色，非 TTY / `NO_COLOR=1` / `--no-color` 全关
- 不用 emoji 装饰（除非 `--rich`）。状态用 ASCII 标记：

| 状态 | plain | rich |
|---|---|---|
| ok / done | `*` 或裸文本 | `●` 绿 |
| running | `>` | `◐` 蓝紫旋转 |
| paused | `~` | `◌` 琥珀 |
| error | `!` | `✗` 红 |
| pending | `.` | `○` 灰 |

> **铁律**：plain 模式**不出现任何 emoji 或非 ASCII 装饰字符**。脚本要稳。

---

## 2. 全局 flags

```
--project <id|slug>      指定 project（默认读 daemon ActiveContext，cwd fallback）
--json                   JSON 输出
--rich                   强制 TUI
--no-rich                禁用 TUI
--no-color               关颜色
--quiet, -q              只输出错误
--verbose, -v            详细日志（可叠加 -vv / -vvv）
--socket <path>          覆盖 daemon socket 位置
--config <path>          覆盖配置文件
--help, -h
--version
```

环境变量映射：

| 变量 | 等价 flag |
|---|---|
| `BLUES_PROJECT` | `--project` |
| `BLUES_SOCKET` | `--socket` |
| `BLUES_CONFIG` | `--config` |
| `NO_COLOR` | `--no-color` |
| `BLUES_JSON` | `--json` |

---

## 3. 命令骨架

### 3.1 Daemon

```
blues daemon start                    前台启动 daemon
blues daemon start --detach           后台启动（写 pid）
blues daemon stop                     SIGTERM 优雅停止
blues daemon restart                  stop + start
blues daemon status                   ping，输出 PID/uptime/active counts
blues daemon logs [--follow] [--lines N]
blues daemon enable-autostart         注册 OS 自启
blues daemon disable-autostart
```

`status` 默认输出（plain）：

```
daemon: running (pid 12345, uptime 2h 34m)
socket: /Users/u/.blues/daemon.sock
projects active: 3
plans running: 1, paused: 0
```

`status --json`：

```json
{
  "running": true,
  "pid": 12345,
  "uptime_secs": 9244,
  "socket": "/Users/u/.blues/daemon.sock",
  "projects": { "active": 3 },
  "plans": { "running": 1, "paused": 0, "done": 12 }
}
```

### 3.2 Project

```
blues project init [<path>]                   初始化为 Blues project
blues project init --name <name> --slug <s>   显式命名
blues project list                            列出所有 active project
blues project info [<id|slug|.>]              展示详情，. = cwd
blues project archive <id|slug>
blues project delete <id|slug>                三次确认
blues project import <path|manifest>          从他人/旧机器接管
blues project link <a> <b> --scope <s>        a 单向 link b
blues project unlink <a> <b>
blues project use <id|slug>                   设 ActiveContext.project
```

`project init` 交互流（plain，TTY）：

```
$ blues project init
This doesn't look like a Blues project yet.

Detected hints:
  - .git/ at .
  - package.json at .

Initialize Blues here? [Y/n] y
Project name [blues]: my-app
Slug [my-app]:
Memory storage [global]:

✓ Created .blues/project.toml
  id   01HXR8VAB7CQXMK3ZN0E6PWQ72
  name my-app
  slug my-app
```

`project list` 输出：

```
NAME            SLUG          ID                          PRIMARY ROOT                STATUS
* my-app        my-app        01HXR8...PWQ72              /Users/u/work/my-app        active
  client-acme   client-acme   01HX9F...K3Z2W              /Users/u/work/client-acme   active
  shared-skills shared-skills 01HX2...Q9N1L                                            archived
```

`*` 标记 ActiveContext 当前 project。

### 3.3 Memory

```
blues mem query <text> [--scope project|linked|global] [--top-k N] [--type T]
blues mem save <text> [--type episodic|semantic|procedural] [--source S] [--confidence F]
blues mem inbox                       默认进 TUI
blues mem inbox list                  非交互列表
blues mem inbox approve <item-id>...
blues mem inbox reject <item-id>...
blues mem inbox edit <item-id>
blues mem blame <fact-id>             显示 fact 来源链
blues mem stats                       facts/episodes/relations 总数
```

`query` 默认输出：

```
$ blues mem query "how do we handle logout"
3 results (project=my-app, scope=project)

[0.91] semantic   logout requires confirmation modal for trusted devices
                  → fact-01HXR9... · from plan refactor-auth, node N#3
[0.84] episodic   user said: "we got burned by silent logout last quarter"
                  → ep-01HXR8... · 2026-05-22
[0.71] procedural after logout: clear local cache, redirect to /login
                  → proc-01HXQF... · 5 uses
```

`--json` 输出符合 `MemoryResults` proto。

### 3.4 Plan

```
blues plan new <intent> [--no-start]                创建（默认立即 start）
blues plan list [--status running|paused|done|all]
blues plan log <plan-id>                            事件日志（git log 风）
blues plan show <plan-id>                           当前快照
blues plan watch <plan-id>                          默认 TUI
blues plan pause <plan-id>
blues plan resume <plan-id>
blues plan cancel <plan-id>
blues plan inject <plan-id> <node-id> <message>
blues plan edit <plan-id> <node-id> [--prompt P]
# v0.2:
blues plan fork <plan-id> <node-id>
blues plan replay <plan-id> <node-id>
blues plan rewind <plan-id> <node-id>
blues plan diff <a> <b>
blues plan merge <src> <dst>
```

v0.2 子命令在 v0.1 必须存在但返回：

```
$ blues plan fork P#5 N#3
error: 'plan fork' is not available in v0.1 (planned for v0.2)
exit code 64 (NotImplemented)
```

`plan log` 输出（git log 致敬）：

```
$ blues plan log P#5

plan P#5  refactor-auth
created 2026-05-24 14:20  by desktop
status  running

  ●  N#1  LlmTask     plan steps                   2s    2026-05-24 14:20
  ●  N#2  Tool        fs.read auth.ts              0.1s  2026-05-24 14:21
  ●  N#3  LlmTask     extract assumptions          12s   2026-05-24 14:22
  ◐  N#4  LlmTask     propose refactor             ...   running since 14:33
  ○  N#5  Tool        fs.write auth.ts             -     pending
  ○  N#6  LlmTask     verify                       -     pending
```

### 3.5 Model

```
blues model list                    所有 provider + 可用 model
blues model usage [--since DATE] [--by provider|model|day]
blues model test <provider>         健康检查（cheap call）
```

### 3.6 MCP

```
blues mcp serve [--project <id|slug>] [--stdio | --http <addr>]
                                    给 MCP host 用（参协议 §4）
blues mcp test                      自检 4 个工具是否暴露
```

### 3.7 杂项

```
blues version                       版本信息
blues completions <bash|zsh|fish|pwsh>  shell 补全脚本
blues skill list
blues skill install <path|url>
blues skill enable <name>
blues skill disable <name>
```

---

## 4. TUI 子命令细节

### 4.1 `blues plan watch`

```
┌─ plan P#5 refactor-auth ──── status: running ─── 14:20 ──── 12k tok ────┐
│                                                                          │
│  ● N#1  LlmTask    plan steps                  2s     done               │
│  ● N#2  Tool       fs.read auth.ts             0.1s   done               │
│  ● N#3  LlmTask    extract assumptions         12s    done               │
│  ◐ N#4  LlmTask    propose refactor            ...    running            │
│      ┌─ output ────────────────────────────────────────────────────────┐ │
│      │ Looking at auth.ts, I see three concerns...                     │ │
│      │ 1. logout flow has two divergent paths                          │ │
│      │ 2. session token expiry isn't centrally managed                 │ │
│      │ ▌                                                               │ │
│      └─────────────────────────────────────────────────────────────────┘ │
│  ○ N#5  Tool       fs.write auth.ts            -      pending            │
│                                                                          │
│  [p] pause  [r] resume  [c] cancel  [i] inject  [e] edit  [q] quit       │
└──────────────────────────────────────────────────────────────────────────┘
```

按键：

| 键 | 行为 |
|---|---|
| `j/k` | 上下选节点 |
| `Enter` | 进入节点详情视图 |
| `p` | pause plan |
| `r` | resume plan |
| `c` | cancel plan（confirm） |
| `i` | inject message 到选中节点 |
| `e` | edit 选中节点 prompt |
| `f` | fork 选中节点（v0.2 灰） |
| `q` | quit |

### 4.2 `blues mem inbox`

```
┌─ inbox · my-app · 7 candidates ─────────────────────────────────────────┐
│                                                                          │
│  > [1/7] episodic   conf 0.82   from plan refactor-auth, node N#3        │
│        "user prefers logout to be confirmation-less for trusted          │
│         devices"                                                         │
│        source: chat at 14:32                                             │
│                                                                          │
│    [2/7] semantic   conf 0.71   from plan refactor-auth, node N#3        │
│        "auth.ts owns logout flow"                                        │
│                                                                          │
│    [3/7] procedural conf 0.65   from plan onboard-new-dev, node N#7      │
│        ...                                                               │
│                                                                          │
│  [a] approve  [r] reject  [e] edit  [A] approve all type   [q] quit      │
└──────────────────────────────────────────────────────────────────────────┘
```

按键：

| 键 | 行为 |
|---|---|
| `j/k` | 翻 |
| `a` | approve 当前 |
| `r` | reject 当前 |
| `e` | edit（进 $EDITOR 临时文件） |
| `space` | 多选 toggle |
| `A` | 批量 approve 选中（或当前 type 全部） |
| `R` | 批量 reject |
| `q` | quit |

### 4.3 `blues daemon logs --follow`

ratatui 滚动视图，按级别上色（rich 模式），可 `f` 切过滤。

---

## 5. 输出契约

### 5.1 plain 模式

- **列对齐**：用空格不用 tab（pipe 给 awk 友好）
- **header 行可关**：`--no-header` 输出无表头
- **稳定字段顺序**：每个子命令的列顺序写入文档并冻结
- **ID 显示**：默认短 ID（前 8 字符），`--full-id` 全 ULID

### 5.2 json 模式

- 一次性输出（不流式），即使是 watch 类命令 `--json` 也输出最终态
- schema 与 gRPC proto 对齐，字段名 `snake_case`
- 顶层永远是 object（不裸 array）：

```json
{
  "kind": "memory_query_result",
  "version": 1,
  "data": { ... }
}
```

`kind` / `version` 给消费方做 schema 兼容判断。

### 5.3 stream 输出（v0.2 新增）

部分命令支持 `--stream-json`（NDJSON，逐行）：

- `blues plan watch --stream-json`
- `blues daemon logs --follow --stream-json`

每行一个 JSON 对象，含 `kind` / `data` / `ts`。

---

## 6. 退出码

| 码 | 含义 | 例 |
|---|---|---|
| 0 | 成功 | 正常完成 |
| 1 | 通用失败 | 未分类错误 |
| 2 | 用法错误 | 参数错误（clap 默认） |
| 64 | NotImplemented | v0.2 命令在 v0.1 |
| 65 | InvalidArgument | 字段格式不对 |
| 66 | NotFound | project/plan/node 不存在 |
| 67 | Conflict | 状态冲突（如 plan 已 running） |
| 68 | PermissionDenied | 用户拒批准 / capability 拦 |
| 69 | Unavailable | daemon 未连上 |
| 70 | ProtocolMismatch | client/daemon 版本不兼容 |
| 71 | Internal | daemon 内部错 |
| 130 | Interrupted | Ctrl+C |

退出码与 `BluesError` / gRPC code 映射在 `protocol-and-project.md §3.4` 一致。

---

## 7. JSON Schema 索引

每个支持 `--json` 的命令的 schema 定义在 `proto/cli_outputs.proto`，由 `blues-protocol` 生成 stub。

v0.1 必须冻结 schema 的命令：

- `daemon status`
- `project list` / `project info`
- `mem query` / `mem inbox list` / `mem stats`
- `plan list` / `plan show` / `plan log`
- `model list` / `model usage`
- `version`

> Schema 一旦冻结，破坏性变更要 schema version bump。

---

## 8. 与 daemon 的连接

### 8.1 自动 spawn（决策 D1 = A）

CLI 调用任意需要 daemon 的命令时：

```
1. 尝试连 socket
2. 连不上 → spawn `blues daemon start --detach`
3. 等待 socket 出现（poll 50ms × 20 次 = 1s 超时）
4. 重连
5. 仍失败 → 报 exit 69 + 提示用户 `blues daemon start` 手动起
```

### 8.2 ActiveContext 解析顺序

`--project` 解析：

```
1. CLI flag --project
2. 环境变量 BLUES_PROJECT
3. cwd 向上找 .blues/project.toml
4. daemon ActiveContext.project（多个客户端共享）
5. 报错：no project specified, run inside one or pass --project
```

### 8.3 协议握手

每次连接：

1. CLI 发 `Hello { client_kind: "cli", version, protocol_version }`
2. daemon 响 `HelloResponse`
3. 不兼容 → exit 70 + 提示升级哪边

---

## 9. 实现要点（指给 blues-cli crate）

| 决策 | 实现 |
|---|---|
| CLI 解析框架 | `clap` v4，derive 模式 |
| TUI | `ratatui` + `crossterm` |
| 进度 / spinner | `indicatif`，TTY 自动启用 |
| JSON 序列化 | `serde_json`，schema 与 proto 对齐 |
| 颜色 | `nu-ansi-term` 或 `owo-colors`，统一 disabled-by-default 在非 TTY |
| 配置 | 复用 daemon 的 config 加载链 |
| Shell 补全 | `clap_complete` |

---

## 10. v0.1 验收清单（CLI）

### 必须

- [ ] `blues daemon {start,stop,status,restart,logs}` + autostart
- [ ] `blues project {init,list,info,archive,delete,import,link,unlink,use}`
- [ ] `blues mem {query,save,inbox(+TUI),blame,stats}`
- [ ] `blues plan {new,list,log,show,watch(+TUI),pause,resume,cancel,inject,edit}`
- [ ] `blues plan {fork,replay,rewind,diff,merge}` 存在但返回 exit 64
- [ ] `blues model {list,usage,test}`
- [ ] `blues mcp {serve,test}`
- [ ] `blues version` / `blues completions <shell>`
- [ ] 全局 flags 全部生效，环境变量映射
- [ ] plain / rich / json 三模式按规则切换
- [ ] 所有 v0.1 列出的命令都有 schema 冻结的 JSON 输出
- [ ] 自动 spawn daemon
- [ ] 退出码符合 §6
- [ ] Linux + macOS + Windows 端到端通过

### 可选

- [ ] `--stream-json`（v0.2）
- [ ] `--full-id` / `--no-header` 等细节 flag

---

## 11. 不在 v0.1 范围

- `blues plan {fork,replay,rewind,diff,merge}` 实现（v0.2，v0.1 仅占位）
- `--stream-json`（v0.2）
- 远程 daemon（`--socket tcp://...`）（v0.3+）
- 多 daemon 切换（v0.3+）
- 插件式 CLI 子命令（不规划）

---

**FROZEN. CLI v0.1 冻结于 2026-05-24。**
