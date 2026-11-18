# 用 Rust 实现一个基础 Raft 库：架构设计与核心数据结构

如果目标是用 Rust 重写一个基础 Raft 库，我建议采用类似 `etcd-io/raft` 的设计思路：把 Raft 协议核心实现为一个确定性的状态机，将网络、磁盘存储、定时器和业务状态机放在外部，通过明确的接口与核心交互。

这样设计有几个好处：

- 协议逻辑与网络、磁盘 IO 解耦，便于单元测试。
- 可以用模拟网络测试选举、网络分区、消息乱序等场景。
- Rust 的类型系统可以帮助约束节点角色、日志索引、消息类型与所有权。
- 后续可以替换 TCP、QUIC、内存存储、磁盘存储，而不需要重写 Raft 核心。

这里先确定项目边界：我们实现的是一个可嵌入其他 Rust 服务的 Raft 共识库，不是完整的分布式 KV 数据库。第一版实现固定成员集群、Leader 选举、日志复制、提交、状态机应用和持久化；快照、成员变更、ReadIndex 等作为后续阶段。

## 一、总体架构

我建议把系统分成 5 个层次：

Application / State Machine

KV 数据库、元数据服务、业务命令执行

Raft Node API

提议命令、接收消息、Tick、Ready / Advance

## Raft Core

Election

任期、投票、角色

Replication

日志匹配、复制进度

Commit / Safety

提交推进与安全约束

Raft Log

追加、截断、索引

Storage

日志、任期、投票、快照

Transport

RPC、重试、连接、消息传递

图中最重要的架构原则是：Raft Core 不直接访问网络，也不直接操作文件。 它负责根据输入改变协议状态，并产生需要外部执行的操作。

例如，Leader 产生了一条需要发送给 Follower 的日志复制消息，Core 只负责生成消息；由外部 Transport 负责发送。Leader 需要持久化日志时，Core 提供待持久化数据，由外部 Storage 完成写入。

不过，持久化和消息发送有严格的先后关系，不能简单地把两者都丢进异步任务中随意执行。后面会专门讲这个问题。

## 二、内部模块怎么划分？

建议使用 Rust workspace，先做成一个核心 crate，避免第一版过早拆分多个 crate。

```
rust-raft/
├── Cargo.toml
├── crates/
│   └── raft-core/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── config.rs
│           ├── types.rs
│           ├── node.rs
│           ├── raft.rs
│           ├── role.rs
│           ├── election.rs
│           ├── replication.rs
│           ├── log.rs
│           ├── quorum.rs
│           ├── message.rs
│           ├── ready.rs
│           ├── storage.rs
│           ├── snapshot.rs
│           ├── membership.rs
│           └── error.rs
├── examples/
│   └── kv-cluster/
└── tests/
    ├── election.rs
    ├── replication.rs
    ├── partition.rs
    ├── recovery.rs
    └── model_tests.rs
```

这些模块的职责如下。

| 模块               | 核心职责                                   |
| ---------------- | -------------------------------------- |
| `types.rs`       | `NodeId`、`Term`、`LogIndex` 等基础类型       |
| `config.rs`      | 集群 ID、本地节点 ID、选举超时、心跳间隔等配置             |
| `node.rs`        | 对外 API、消息入口、Tick、Ready 驱动接口            |
| `raft.rs`        | Raft 总状态与核心事件分发                        |
| `role.rs`        | Follower、Candidate、Leader 状态与转换        |
| `election.rs`    | 选举计时、投票请求、投票判定                         |
| `replication.rs` | Leader 复制进度、`next_index`、`match_index` |
| `log.rs`         | 日志追加、索引查询、冲突处理、提交边界                    |
| `quorum.rs`      | 多数派判断、投票统计、提交位置计算                      |
| `message.rs`     | RequestVote、AppendEntries 等协议消息        |
| `ready.rs`       | 向外部暴露待持久化、待发送和待应用的工作                   |
| `storage.rs`     | 日志与 HardState 的持久化接口                   |
| `snapshot.rs`    | 快照生成、安装、日志压缩                           |
| `membership.rs`  | 集群成员及投票权配置                             |
| `error.rs`       | 协议错误、存储错误、参数错误                         |

第一版可以先实现前 12 个模块。`snapshot.rs`、`membership.rs` 可以保留设计位置，但不要在基础选举和日志复制尚未通过测试时就实现复杂功能。

## 三、核心结构设计

这是整个库最值得先设计清楚的部分。建议先确定核心数据结构，再实现选举和日志复制，避免开发到一半才发现模块之间的状态重复或职责混乱。

### 1. 基础类型：NodeId、Term、LogIndex

不要把节点 ID、任期和日志索引全部写成 `u64`。虽然它们底层都是整数，但语义不同，混用很容易产生难以排查的错误。

```
// types.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Term(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogIndex(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogId {
    pub index: LogIndex,
    pub term: Term,
}
```

这几个类型的关系：

- `NodeId` 标识一个 Raft 节点。
- `Term` 标识一次选举任期。
- `LogIndex` 标识日志序列中的位置。
- `LogId` 将日志位置和创建该日志时的任期绑定在一起。

为什么 `LogId` 同时包含 `index` 和 `term`？

因为 Raft 检查日志匹配时，不能只看索引。两个节点在相同索引处的日志任期相同，才可以利用 Log Matching 性质推导它们具有相同的前缀。

例如：

```
Node A: (1,1) (2,1) (3,2) (4,2)
Node B: (1,1) (2,1) (3,2) (4,3)
```

A、B 在索引 3 处的 `LogId` 相同，所以索引 1 到 3 的前缀相同；但索引 4 不同，不能直接认定整个日志相同。

### 2. LogEntry：日志条目

```
// log.rs

#[derive(Debug, Clone)]
pub struct LogEntry<C> {
    pub id: LogId,
    pub payload: EntryPayload<C>,
}

#[derive(Debug, Clone)]
pub enum EntryPayload<C> {
    Blank,
    Normal(C),
    // 后续可扩展配置变更条目
}
```

这里使用泛型 `C` 表示业务命令，避免 Raft 核心绑定某一种序列化格式或 KV 数据模型。

例如：

```
#[derive(Debug, Clone)]
pub enum KvCommand {
    Set { key: String, value: Vec<u8> },
    Delete { key: String },
}
```

应用层可以使用 `LogEntry<KvCommand>`，也可以把序列化后的 `Vec<u8>` 作为命令。

我建议基础库优先使用序列化后的字节作为协议边界，业务类型由上层自行定义。这样网络协议、日志存储和状态机之间更容易保持解耦。

还要注意：日志中的 `Term` 表示创建该日志时的 Leader 任期，不是当前节点的任期。

### 3. HardState：必须持久化的协议状态

```
// storage.rs

#[derive(Debug, Clone, Default)]
pub struct HardState {
    pub term: Term,
    pub voted_for: Option<NodeId>,
    pub commit_index: LogIndex,
}
```

这里是一个用于架构设计的简化版本。

三个字段分别代表：

- `term`：节点已经知晓的最高任期。
- `voted_for`：当前任期投给哪个候选人。
- `commit_index`：已知已经提交的最高日志索引。

`term` 和 `voted_for` 是标准 Raft 中必须可靠持久化的状态。`commit_index` 在这个架构中也可以持久化，以便恢复，但不能认为 Raft 协议要求它必须以同样方式持久化；也可以通过安全恢复逻辑重建。

实际实现中还需要明确 `HardState` 是否发生变化，以及哪些消息必须等到对应状态持久化后才能发送。

### 4. RaftLog：管理整个日志序列

`RaftLog` 是独立于 Leader/Follower 角色的基础结构，负责日志本身的管理。

```
// log.rs

pub struct RaftLog<C> {
    // 已经通过提交规则确认的日志边界
    committed: LogIndex,

    // 已经应用到状态机的边界
    applied: LogIndex,

    // 日志条目和底层存储
    unstable: Vec<LogEntry<C>>,
    storage: Box<dyn Storage<C>>,
}
```

这是概念结构，真实实现通常需要更加精细的索引与内存管理。

各字段的关系：

- `committed`：协议已确定的日志提交位置。
- `applied`：应用层已执行的日志位置。
- `unstable`：尚未确认持久化的日志条目。
- `storage`：已经持久化的日志及相关元数据。

正常情况下：

\\[ applied \le committed \le lastLogIndex \\]

如果有快照，还需要把快照边界和日志索引的语义一起纳入约束。

为什么需要区分 `committed` 和 `applied`？

例如：

```
日志索引：  1   2   3   4   5   6
提交状态：  ✓   ✓   ✓   ✓   -   -
应用状态：  ✓   ✓   -   -   -   -
```

这里 `committed = 4`，`applied = 2`。日志 3、4 已经提交，但状态机还没执行它们。

Raft Core 可以继续处理协议消息，而应用层按照顺序执行已提交的日志。这种分离非常重要，尤其是在数据库状态机执行速度较慢时。

另外，`unstable` 不应简单理解为“所有尚未提交的日志”。它主要表示尚未稳定保存的日志。一条日志可以尚未提交，但已经持久化；也可以已被复制到某个节点的内存中，却还未完成持久化。

### 5. RaftState：一个节点的整体协议状态

```
// raft.rs

pub struct RaftState<C> {
    pub id: NodeId,

    pub current_term: Term,
    pub voted_for: Option<NodeId>,

    pub role: Role,
    pub leader_id: Option<NodeId>,

    pub log: RaftLog<C>,
    pub membership: Membership,

    pub election_elapsed: u64,
    pub heartbeat_elapsed: u64,
}
```

`RaftState` 是节点级别的核心上下文。

它维护整个节点当前的协议状态，但不应该再重复维护一份与 `RaftLog` 不一致的日志、提交索引或应用索引。

例如，`current_term` 与持久化的 `HardState.term` 必须遵守统一的更新规则。`RaftLog.committed` 是核心日志状态中的提交边界，生成 Ready 时再决定如何向外部暴露提交进度。

这里需要注意一个实现细节：`election_elapsed` 和 `heartbeat_elapsed` 是逻辑计数，不必是实际毫秒数。通过外部 `tick()` 驱动可以避免核心依赖操作系统计时器。

### 6. Role：节点角色如何建模？

最简单的设计是：

```
// role.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Follower,
    Candidate,
    Leader,
}
```

不过，角色和角色专属状态是两个不同概念。

例如，只有 Leader 才需要维护每个 Follower 的复制进度。如果把所有字段都塞进 `RaftState`，会出现大量 `Option` 字段：

```
// 不推荐的设计示意
struct RaftState {
    role: Role,
    votes: Option<...>,
    next_index: Option<...>,
    match_index: Option<...>,
}
```

这会让非法状态很容易出现，比如 Follower 却保留一套过期的 Leader 复制进度。

对于基础版本，可以先使用：

```
pub struct Raft<C> {
    pub state: RaftState<C>,
    pub votes: VoteTracker,
    pub progress: ProgressTracker,
}
```

通过角色转换函数来控制 `votes` 和 `progress` 的初始化、清理与使用。

如果你希望 Rust 的类型系统进一步保证状态合法，可以把它改成：

```
pub enum RoleState {
    Follower,
    Candidate(CandidateState),
    Leader(LeaderState),
}

pub struct CandidateState {
    pub votes: VoteTracker,
}

pub struct LeaderState {
    pub progress: ProgressTracker,
}
```

这种方式让角色专属字段更加清晰。但它会让跨角色状态更新和事件处理稍复杂一些。我的建议是：第一版使用 `Role` 枚举加独立的角色状态管理，等协议稳定后再决定是否升级为强类型角色状态机。

### 7. LeaderProgress：Leader 如何跟踪每个 Follower？

这是日志复制中最关键的数据结构之一。

```
// replication.rs

#[derive(Debug, Clone)]
pub struct Progress {
    // Leader 认为下一次应该从哪个索引开始发送
    pub next_index: LogIndex,

    // Leader 已确认该 Follower 匹配到的最高日志索引
    pub match_index: LogIndex,

    pub state: ProgressState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressState {
    Probe,
    Replicate,
    Snapshot,
}

pub struct ProgressTracker {
    pub peers: std::collections::HashMap<NodeId, Progress>,
}
```

为什么需要 `next_index` 和 `match_index` 两个字段？

假设 Leader A 的日志如下：

```
A: [1, 2, 3, 4, 5, 6, 7]
```

Leader 可能认为 B 已经匹配到索引 5，但 C 只匹配到索引 2：

```
A: [1, 2, 3, 4, 5, 6, 7]
B: [1, 2, 3, 4, 5]
C: [1, 2]
```

那么：

| 节点        | `match_index` | `next_index`       |
| --------- | ------------- | ------------------ |
| A（Leader） | 自身日志末尾        | 通常无需作为 Follower 跟踪 |
| B         | 5             | 6                  |
| C         | 2             | 3                  |

其中：

- `match_index` 是已确认匹配的位置，不能因为发送请求就直接增加。
- `next_index` 是 Leader 对下一次复制位置的估计，失败时可以回退。

`ProgressState` 用于优化复制过程。`Probe` 适合探测正确的日志位置，`Replicate` 适合连续发送日志，`Snapshot` 表示需要通过快照追赶。

注意：Leader 的 `match_index` 不是提交索引。 即使 B 已经匹配到 5，也不代表索引 5 已经提交；还需要结合其他节点的进度、当前任期和提交规则判断。

### 8. VoteTracker：投票统计

```
// election.rs

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    Granted,
    Rejected,
}

#[derive(Default)]
pub struct VoteTracker {
    pub votes: HashMap<NodeId, Vote>,
}
```

候选人收到投票结果后，更新 `VoteTracker`，并通过 `quorum.rs` 判断：

- 是否已经获得多数派支持？
- 是否已经确定无法获得多数派？
- 是否仍然需要等待其他节点？

不建议在 `election.rs`、`raft.rs` 和 `quorum.rs` 中分别实现一套多数派逻辑，否则成员变更时很容易出现不一致。

投票记录应当按任期管理。新任期开始时，需要重置该轮选举的投票状态。

## 四、核心结构之间的关系

前面列出了许多结构，现在把它们串起来。

Raft

节点级协调入口

RaftState

Term、Vote、Role、Leader ID

RaftLog

日志、Committed、Applied

VoteTracker

Candidate 的投票记录

ProgressTracker

Leader 对各节点的复制进度

Ready / Storage / Transport

持久化状态、发送消息、交付已提交日志

可以用一句话概括它们之间的关系：

- `Raft` 负责把事件分发给正确的协议逻辑。
- `RaftState` 记录节点当前的全局协议状态。
- `RaftLog` 负责管理日志序列及提交、应用边界。
- `VoteTracker` 负责候选人的选举进度。
- `ProgressTracker` 负责 Leader 对每个 Follower 的复制进度。
- `quorum.rs` 根据投票或复制进度计算多数派结果。
- `Ready` 把需要外部执行的副作用交给驱动层。

这里的 `quorum.rs` 是一套规则与计算逻辑，不需要成为一个单独保存可变状态的对象。

## 五、消息类型：节点之间传递什么？

Raft 节点通过消息完成选举和日志复制。建议统一用 `Message` 枚举表达协议消息，而不是每种消息都设计独立的发送流程。

```
// message.rs

#[derive(Debug, Clone)]
pub struct Message<C> {
    pub from: NodeId,
    pub to: NodeId,
    pub term: Term,
    pub kind: MessageKind<C>,
}

#[derive(Debug, Clone)]
pub enum MessageKind<C> {
    RequestVote {
        last_log_index: LogIndex,
        last_log_term: Term,
    },

    RequestVoteResponse {
        granted: bool,
    },

    AppendEntries {
        prev_log_index: LogIndex,
        prev_log_term: Term,
        entries: Vec<LogEntry<C>>,
        leader_commit: LogIndex,
    },

    AppendEntriesResponse {
        success: bool,
        match_index: Option<LogIndex>,
    },

    // 后续可增加 InstallSnapshot 等消息
}
```

这是便于说明的协议模型，真实实现还需要定义拒绝原因、冲突提示、快照传输、消息编码，以及更精确的复制响应语义。

### 消息处理链路

以 A 选举为例：

```
A.tick()
   |
   v
Election 发现选举超时
   |
   v
A.current_term += 1
A.voted_for = A
   |
   v
生成 RequestVote 消息
   |
   v
Ready 暴露待发送消息
   |
   v
外部 Transport 发送给 B、C
   |
   v
B、C 接收消息并调用 step()
   |
   v
返回投票响应
   |
   v
A 收到响应，更新 VoteTracker
   |
   v
quorum 判定获得多数票
   |
   v
A 转为 Leader
```

实际代码里，任期更新和投票必须按规则持久化。上面展示的是协议流程，而不是说 `current_term` 增加后可以不等持久化就随意发送所有消息。

另外，`step()` 不应该直接进行网络发送。它应当更新内部状态，并产生消息或待执行工作，交给外部驱动。

## 六、Ready / Advance：我最建议你认真设计的接口

如果你只在整个架构中深入研究一个设计点，我建议研究 `Ready`。

它解决的核心问题是：协议状态机如何告诉外部驱动，接下来必须持久化什么、发送什么消息、应用哪些日志？

### 1. 为什么不能在 Raft Core 里直接写磁盘和发网络请求？

假设收到一个 AppendEntries 请求，需要完成以下操作：

1. 检查 Leader 任期。
2. 检查前置日志。
3. 追加新日志。
4. 将日志写入磁盘。
5. 返回成功响应。

如果 Core 直接调用磁盘和网络接口，协议代码就会被异步 IO、错误处理和重试逻辑缠绕起来，测试也会更复杂。

因此，建议 Core 生成一组待处理工作：

```
// ready.rs

pub struct Ready<C> {
    pub hard_state: Option<HardState>,
    pub unstable_entries: Vec<LogEntry<C>>,
    pub messages: Vec<Message<C>>,
    pub committed_entries: Vec<LogEntry<C>>,
}
```

这同样是一个概念版本。实际实现还需要定义快照、读状态、持久化进度以及每一批 Ready 的生命周期。

### 2. Ready 的外部处理流程

```
Raft Core
    |
    | 生成 Ready
    v
Driver
    |
    +--> 持久化 HardState / Entries
    |
    +--> 发送允许发送的 Messages
    |
    +--> 将已提交日志交给 State Machine
    |
    v
Advance
    |
    v
Raft Core 允许继续推进处理进度
```

这里必须特别强调：不是每条消息都必须等到所有日志写盘之后才发送，而是必须遵守对应消息所依赖的持久化顺序。

例如：

- 依赖某条日志已持久化的 AppendEntries 成功响应，不能在日志实际落盘之前发出。
- 依赖新任期和投票状态的选举响应，必须遵守持久化规则。
- 发送只用于提示或探测、且不依赖未持久化状态的消息，可以有不同的调度时机。

第一版实现最安全的策略是：先使用保守的顺序化驱动——按协议要求完成必要的持久化，再发送依赖该持久化结果的消息。

### 3. Advance 的意义

`Advance` 用来告诉 Core：外部驱动已经处理完某批 Ready 中约定的工作，可以继续推进相应的进度。

例如，外部已经：

- 保存了这批日志。
- 发送了允许发送的消息。
- 将对应的已提交日志交付给状态机。

那么 Core 就可以确认这批工作已经完成。

需要注意，`Advance` 不应该简单地理解为“所有消息都已成功到达目标节点”。发送成功通常只代表消息被传递给本地网络层；Follower 是否收到、是否持久化，需要通过后续协议响应判断。

对于第一版 Rust 库，我建议先实现单批次 Ready、严格顺序处理和显式完成确认。等正确性测试稳定后，再考虑多批次 Ready 并行持久化和异步流水线。

## 七、Storage 接口应该怎么设计？

建议将 Raft 需要的持久化能力抽象成 trait。

```
// storage.rs

pub trait Storage<C> {
    type Error;

    fn initial_state(&self) -> Result<HardState, Self::Error>;

    fn last_index(&self) -> Result<LogIndex, Self::Error>;

    fn term(
        &self,
        index: LogIndex,
    ) -> Result<Option<Term>, Self::Error>;

    fn entries(
        &self,
        low: LogIndex,
        high: LogIndex,
    ) -> Result<Vec<LogEntry<C>>, Self::Error>;

    fn append(
        &mut self,
        entries: &[LogEntry<C>],
    ) -> Result<(), Self::Error>;

    fn persist_hard_state(
        &mut self,
        state: &HardState,
    ) -> Result<(), Self::Error>;
}
```

这是展示接口边界的简化版本。真正实现还需要明确日志覆盖规则、索引范围、原子写入语义、持久化完成语义，以及快照和恢复的接口。

建议第一版提供两个实现：

- `MemStorage`：用于单元测试、模拟网络和快速开发。
- `FileStorage`：使用文件或嵌入式数据库实现可靠持久化。

如果后续要支持高吞吐量，可以再加入批量写入、日志分段和 Group Commit。

Storage 不能替 Raft 决定哪些日志已提交。 它负责保存数据和提供读取能力；提交决策仍由 Raft Core 完成。

## 八、对外 API：如何让使用者驱动 Raft？

我建议参考以下接口：

```
pub trait RaftNode<C> {
    fn campaign(&mut self);

    fn propose(&mut self, command: C);

    fn step(&mut self, message: Message<C>);

    fn tick(&mut self);

    fn ready(&mut self) -> Option<Ready<C>>;

    fn advance(&mut self, token: ReadyToken);
}
```

这里的签名是架构示意，尚未包含错误返回、消息验证、Ready 所有权以及并发约束。正式实现中应该使用 `Result`，并明确哪些 API 只能由一个驱动任务调用。

这些 API 的作用：

| API          | 含义               |
| ------------ | ---------------- |
| `campaign()` | 主动发起选举           |
| `propose()`  | 向本地 Raft 提交待复制命令 |
| `step()`     | 输入来自其他节点的消息      |
| `tick()`     | 推进逻辑时钟，触发心跳或选举超时 |
| `ready()`    | 获取需要外部处理的工作      |
| `advance()`  | 确认一批工作已经按要求处理完成  |

我建议不要直接让 `RaftNode` 内部创建 TCP 连接或启动 Tokio 任务。外部使用者可以选择 Tokio、async-std，或者自定义运行时。

### 推荐的并发模型

第一版采用每个节点一个串行驱动任务。

```
Tokio Runtime
     |
     +---- Node A Driver ---- Raft Core A
     |
     +---- Node B Driver ---- Raft Core B
     |
     +---- Node C Driver ---- Raft Core C
```

每个 Driver 串行处理：

- 定时器 Tick。
- 收到的协议消息。
- 本地客户端提议。
- Ready 持久化和发送流程。

这样可以避免在 `RaftState`、`RaftLog`、`ProgressTracker` 上引入大量锁，也能让 Core 保持同步、确定性的状态机。

等以后需要优化吞吐量，再让磁盘写入、网络传输和状态机应用并行执行，同时通过 Ready token、批次编号或等效的进度机制维持严格的依赖关系。

## 九、一个写请求在整个架构里如何流转？

把前面所有结构结合起来，看看客户端执行 `SET x 10` 时，内部到底发生什么。

1\. Application

调用 propose(SET x 10)

2\. Raft Core

检查角色，将命令追加到 Leader 的日志

3\. RaftLog + Ready

记录未稳定日志，产生待持久化条目和待发送消息

Storage

持久化 Leader 日志

Transport

向 Follower 发送日志

4\. Follower 持久化并回复

Leader 根据有效响应更新 matchIndex

5\. Quorum + Commit

结合多数派复制进度与当前任期规则推进 commitIndex

6\. State Machine

按顺序应用已提交日志，并向客户端返回结果

这里还有两个必须在代码中明确的细节：

1. Leader 自己的日志也必须可靠持久化，不能只统计 Follower 的响应。
2. `match_index` 只能根据符合协议要求的复制确认更新，不能因为消息已发送就提前推进。

## 十、Rust 实现中特别值得注意的设计问题

### 1. 避免让一个结构同时承担所有职责

不建议把选举、日志操作、消息发送、磁盘 IO 全部写在一个几千行的 `raft.rs` 中。

但也不要为了模块化，把每个小函数都变成一个独立的 trait 对象。

推荐的边界是：

- `Raft`：事件分发与状态转换。
- `Election`：选举规则与投票统计。
- `RaftLog`：日志管理。
- `ProgressTracker`：复制进度。
- `Quorum`：多数派计算。
- `Ready`：外部副作用交付。

这些模块之间直接使用具体类型即可。第一版没有必要为了可插拔性给所有内部模块都增加 trait。

### 2. 处理 Rust 的借用问题

一个常见场景是：你需要遍历 `ProgressTracker` 中的节点，同时从 `RaftLog` 获取日志并更新提交索引。

如果这些字段都放在同一个可变借用范围里，很容易遇到借用冲突。

建议：

- 先读取需要的数据，计算复制结果。
- 再单独修改 `commit_index`。
- 避免在持有 `&mut Progress` 时调用可能再次借用整个 `Raft` 的方法。
- 将纯计算函数抽离，例如 `calculate_commit_index()`。

例如：

```
fn calculate_commit_index(
    progress: &ProgressTracker,
    current_term: Term,
    log: &RaftLog<Vec<u8>>,
) -> Option<LogIndex> {
    // 读取复制进度，计算满足提交规则的索引。
    // 这里只读，不直接修改 Raft 状态。
    todo!()
}
```

这只是函数边界示意。实际实现还需要处理配置成员、Leader 自身进度和日志任期查询。

这种先计算、后修改的方式，也有助于让共识规则成为可独立测试的纯函数。

### 3. 区分协议错误和运行环境错误

建议定义统一错误类型：

```
pub enum RaftError {
    NotLeader,
    StaleTerm,
    InvalidMessage,
    InvalidLogIndex,
    StorageFailure,
    InvalidConfiguration,
}
```

正式版本可以使用 `thiserror`，并将底层存储错误通过泛型或错误源保留下来。

但不要把“当前不是 Leader”与“磁盘写入失败”当成同一种错误：

- `NotLeader` 通常是正常的协议状态。
- `StorageFailure` 可能意味着节点无法继续安全推进协议，需要停止相关处理并进行恢复。

尤其是持久化失败，不能简单忽略后继续发送依赖该持久化结果的成功响应。

### 4. 为索引设计明确的边界语义

这是自研 Raft 实现最容易出现 bug 的地方之一。

你需要提前确定：

- 日志索引是否从 1 开始。
- 空日志的 `last_index` 如何表示。
- `entries(low, high)` 是否采用左闭右开区间。
- `term(index)` 对快照边界和已压缩日志如何返回结果。
- `committed` 与 `applied` 是否允许等于快照索引。
- 日志截断是否可能影响已经提交的日志。

建议统一采用左闭右开区间：

\\[ [low, high) \\]

这样读取 3、4、5 三条日志时，调用 `entries(3, 6)` 即可。对边界的定义必须写入 Storage 接口文档和单元测试。

## 十一、建议的开发里程碑

不要一次性实现完整 Raft。按阶段推进更容易保证正确性。

## 开发计划

0/6 已完成

1\. 阶段 1：核心类型与单节点状态机

实现 NodeId、Term、LogId、LogEntry、Message、RaftState，以及基本状态转换。

测试重点：验证任期递增、低任期消息拒绝和高任期消息处理。

2\. 阶段 2：选举

实现 RequestVote、投票限制、日志新旧判断、随机化选举超时。

测试重点：测试多数派选举、重复投票、分裂投票与更高任期。

3\. 阶段 3：日志复制

实现 AppendEntries、前置日志检查、冲突修复、next_index 和 match_index。

测试重点：测试复制成功、落后节点追赶、冲突日志覆盖。

4\. 阶段 4：提交与状态机应用

实现多数派提交规则、当前任期限制、commitIndex 与 lastApplied。

测试重点：验证已提交日志不会被覆盖，所有节点按序应用。

5\. 阶段 5：Ready 与可靠存储

实现 MemStorage、持久化顺序、Ready / Advance 和节点重启恢复。

测试重点：模拟磁盘写入失败、节点重启和消息延迟。

6\. 阶段 6：生产能力

增加快照、日志压缩、安全成员变更、线性一致读与性能优化。

测试重点：进行网络分区、长期故障、恢复、并发和模型测试。

## 十二、我推荐的技术路线和参考项目

建议参考以下项目的设计思路，但不要直接照搬它们的全部架构。

- [etcd-io/raft](https://github.com/etcd-io/raft?utm_source=chatgpt.com)：Go 实现，适合理解确定性状态机、日志复制、Ready 机制。
- [openraft](https://github.com/databendlabs/openraft?utm_source=chatgpt.com)：Rust 实现，可以研究 Rust 类型设计、存储接口、日志管理和测试方式。
- [Raft 原始论文](https://raft.github.io/raft.pdf?utm_source=chatgpt.com)：用于校验你的协议行为是否符合理论。
- [Raft 可视化教程](https://thesecretlivesofdata.com/raft/?utm_source=chatgpt.com)：适合辅助理解选举与日志复制。

特别建议阅读 `openraft` 的源码，但将它作为设计参考，而不是第一版的依赖。因为你的目标是自己重写基础库，最有价值的是理解它如何表达状态、存储和日志边界，再决定自己的接口。

### 最后的架构建议

第一版我会采用以下组合：

- 语言与运行时：稳定版 Rust，核心同步实现，外部可选 Tokio。
- 协议模型：固定成员配置，Leader/Follower/Candidate 三种角色。
- 核心数据结构：`RaftState`、`RaftLog`、`VoteTracker`、`ProgressTracker`。
- 外部接口：`tick()`、`step()`、`propose()`、`ready()`、`advance()`。
- 存储层：`Storage` trait，加一个内存实现和一个可靠持久化实现。
- 测试方式：确定性模拟时钟、模拟网络、故障注入和多节点集成测试。

最重要的是：先实现一个可以在内存模拟网络中正确运行的 3 节点 Raft，再考虑真实网络、磁盘 IO 和性能优化。 只要状态机接口、日志索引语义、提交规则和持久化顺序一开始设计正确，后续扩展快照、成员变更和线性一致读就会容易很多。