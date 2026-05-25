# Blues — 项目宪法

> 一切产品决策、命名、文案、视觉、代码取舍，都回到这份文档对齐。
> 当任何争论出现，以本文为准。
>
> 最后修订：2026-05-24

---

## 1. 定位

### 主标语
> **Blues — Git for AI collaboration.**
> **Blues — AI 协作的 Git。**

### 副标语
> **Make universes with AI.**
> **和 AI 一起开宇宙。**

### 组合呈现
> **Blues — Git for AI collaboration. Make universes with AI.**
> **Blues — AI 协作的 Git。和 AI 一起开宇宙。**

---

## 2. 价值锚

Blues 把 **AI 协作**从瞬时态升级到时间态。

对手做的是更聪明的瞬时智能。
**Blues 做的是承载 AI 协作的时间基础设施。**

| 维度 | 别人 | Blues |
|---|---|---|
| 单位 | 一次对话 / 一次任务 | 一次协作（plan + memory + worktree + snapshot） |
| 状态 | 瞬时 | 有过去、现在、未来 |
| 操作 | 完成、调用、记住 | 追溯、分叉、重放、继承 |
| 主语 | "agent 能…" | "AI 协作过程能…" |

---

## 3. 四件套动词（产品语言根）

围绕**时间**组织：

| 时间维度 | 动词（中） | 动词（英） | 实现 |
|---|---|---|---|
| 协作的过去 | **追溯** | **trace** | provenance map / event log |
| 协作的现在 | **分叉** | **fork** | plan fork + sandbox snapshot clone |
| 协作的折叠 | **重放** | **replay** | replay node / time-travel |
| 协作的未来 | **继承** | **inherit** | linked_projects / cross-project memory |

> 凡是 Blues 的功能命名、CLI 子命令、UI 按钮、文档章节，
> 都尽量贴在这四个动词上，不要发明同义新词。

---

## 4. 反派叙事

> **Other AI tools forget. Blues remembers, branches, and travels through time.**
> **别的 AI 工具用完即弃。Blues 让协作有过去、现在和未来。**

被反派化的对象：
- 任何把 AI 对话做成"消失态"的工具
- 任何把"agent 智能"当成唯一卖点的工具
- 任何把记忆/状态/决策锁在云端不可携带的工具

具体竞品对照：
- Codex / Antigravity / Claude Code → 单线、无 fork、无回放
- ChatGPT / Cursor / Continue → 对话用完即弃、无项目身份
- OpenClaw / Hermes / OpenHuman → 强在 agent 维度，缺时间维度

---

## 5. 双语义场

### 5.1 Git 语义场（CLI / API / 文档用）

借 Git 既有的硬核威望，命名直接套：

| Blues 概念 | Git 词 | CLI |
|---|---|---|
| 创建 plan | init / start | `blues plan new` |
| 节点完成自动 commit | commit | （隐式） |
| Fork | branch / fork | `blues plan fork <node>` |
| 回退 | reset / checkout | `blues plan rewind <node>` |
| 重放 | replay / cherry-pick | `blues plan replay <node>` |
| 合并 | merge | `blues plan merge` |
| 对比 | diff | `blues plan diff a..b` |
| 历史 | log | `blues plan log` |
| 找出处 | blame | `blues mem blame <fact>` |

> **铁律**：CLI 永远说人话。不要把宇宙美学污染到命令行。
> 没有 `blues star navigate`，只有 `blues plan replay`。

### 5.2 宇宙语义场（视觉 / 营销 / UI 情绪用）

只用在情绪面：

| Blues 概念 | 宇宙化叙事 |
|---|---|
| Plan | 一次航行 / 一颗星 |
| Fork | 平行宇宙 |
| Memory | 星图 / 记忆星座 |
| linked_projects | 星系 |
| Time travel | 时间折返 |
| Project Identity | 锚点 / 坐标 |
| Daemon | 引力中心 |

UI 风格基线（v0.1 desktop）：
- 深色基调
- Plan Graph 视觉化为星系连接图
- Fork 用空间裂变动效
- 字体偏 mono 现代风（Geist Mono / JetBrains Mono）
- 极客感 + 一点冷静的浪漫

---

## 6. 产品取舍（不卷什么 / 死磕什么）

### 不卷
- **不卷模型 SOTA**：路由 + 多 provider，不自训
- **不卷工具数量**：兼容 Claude Code skill / E2B / MCP，借生态
- **不卷 agent 智能**：用业界主流 agent 模式，工程化做到极致

### 死磕（护城河五件套）
1. **Daemon 协议**（gRPC + MCP 双通道）
2. **记忆引擎**（认知三层 + 预算编译器 + 巩固）
3. **Plan Graph**（DAG + Fork + Replay + Time-travel）
4. **Project Identity**（UUID + linked_projects + 跨设备同步）
5. **CubeSandbox 集成**（MicroVM + 快照 + eBPF 网络）

> 这五件别人不会跟着卷——它们是结构性优势，不是参数优势。

---

## 7. 战略护城河

| 护城河 | 机制 |
|---|---|
| **开放协议** | MCP 暴露记忆 = 任何 AI 工具都能用 Blues 记忆，反向引流 |
| **E2B 兼容** | CubeSandbox 兼容 E2B SDK = 一脚踏入 E2B 生态 |
| **本地优先 + E2EE** | 信任沉淀、隐私党刚需 |
| **开源 Apache 2.0** | 价值锚不可被改，社区只能改实现 |
| **时间维度** | 用得越久越离不开（记忆复利、协作沉淀） |

---

## 8. 目标用户

**个人 hacker / 超级个体 / OPC（One-Person Company）**

不是：
- 大型企业团队（先不做，未来再考虑）
- AI 小白（不是入门工具）
- 一次性玩家（Blues 的复利对短期用户没意义）

是：
- 一人多项目并行
- 跨设备工作（mac/win/linux）
- 重视决策可追溯（客户/法务/未来的自己会问）
- 想把"和 AI 协作的能力"沉淀成个人资产

---

## 9. 三态同构

```
[Desktop (Tauri)]   [VS Code Ext]   [CLI]
        ↕               ↕             ↕
        └────── gRPC over UDS ────────┘
                       ↓
              [Blues Daemon (Rust)]
                 daemon 是真神
```

> 任何客户端切换不丢上下文，Daemon 持有所有状态。
> 三态背后是同一个引擎、同一份记忆、同一份协作历史。

---

## 10. 摘星之路（北极星）

> **第一个让 AI 工作具有时间深度的工具。**
>
> 一个 OPC 五年之后回头看，能说：
> "我所有的 AI 协作历史、决策、记忆都还在，能复盘、能复用、能传承。"
>
> ——这种工具只有一个，叫 Blues。

---

## 附：宣传素材模板

### GitHub README 首行
```
Blues — Git for AI collaboration.
Branch your AI conversations. Fork your decisions. Travel back in time.
```

### Twitter bio
```
Git for AI. Make universes with AI. Open-source.
```

### 中文 Landing Hero
```
AI 协作的 Git。
和 AI 一起开宇宙。
```

### 投资人 / 合作方 一句话
```
Blues is Git for AI collaboration —
the open-source time infrastructure for human-AI work.
```

### 演讲开场白
```
所有 AI 工具都健忘。
Blues 不健忘——它给 AI 协作配上了 Git。
```
