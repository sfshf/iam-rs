# raft

## 给我详细讲解一下raft理论

ChatGPT 答：

可以。你前面已经问到了分布式锁、分布式事务、Saga/TCC，这时候把 Raft 真正吃透非常有价值，因为 Raft 是很多分布式协调系统的底层基础。

我建议你不要把 Raft 理解成“一个选主算法”，而应该理解成：

**Raft 是一个让多个节点对一串操作达成一致，并且在部分节点宕机、网络延迟/丢包的情况下仍然保持一致的共识算法。**

官方论文把 Raft 定义为用于管理 **replicated log（复制日志）** 的 consensus algorithm，并把它拆成 Leader Election、Log Replication、Safety 等相对独立的部分。

### 一、先理解 Raft 到底解决什么问题

假设你有三个服务器：

```txt
        Client
          │
          ↓
    ┌───────────┐
    │           │
    ↓           ↓
 Server A    Server B    Server C
```

你希望这三个节点维护完全一样的数据：

```txt
A: x = 10
B: x = 10
C: x = 10
```

客户端执行：

```txt
SET x 20
```

最终必须：

```txt
A: x = 20
B: x = 20
C: x = 20
```

即使：

```txt
A 挂了
网络延迟
消息丢失
B 挂了
C 重启
```

也不能出现已经“提交”的数据被重新否定。

这就是 Consensus（共识）。

### 二、Raft 最核心的思想

如果只记住 Raft 的一句话：

**先选出一个 Leader，然后所有写操作都经过 Leader，Leader 把操作复制到多数节点，确认多数节点持久化后才认为操作 committed。**

也就是：

```txt
                 Client
                    │
                    ↓
                 Leader
                /      \
               ↓        ↓
           Follower   Follower
```

客户端：

```txt
SET x = 10
```

Leader：

```txt
Log:
[SET x 10]
```

复制：

```txt
Leader
  │
  ├────→ Follower B
  │
  └────→ Follower C
```

多数节点确认：

```txt
A ✓
B ✓
C ✓

Majority = 2/3
```

于是：

```txt
commit
```

然后三个节点把这个 log entry 应用到自己的 State Machine：

```txt
x = 10
```

### 三、为什么 Raft 需要 Log？

这是理解 Raft 的关键。

不要先想：

```txt
Raft = 复制数据库
```

应该想：

```txt
Raft = 复制一份操作日志
```

例如客户端执行：

```txt
SET x 10
SET y 20
SET x 30
DELETE y
```

Raft 保存的是：

```txt
Log
────────────────────
1  SET x 10
2  SET y 20
3  SET x 30
4  DELETE y
────────────────────
```

然后每个节点按照完全相同的顺序执行：

```txt
State Machine
```

于是：

```txt
SET x 10
    ↓
x = 10

SET y 20
    ↓
y = 20

SET x 30
    ↓
x = 30

DELETE y
    ↓
y 不存在
```

最终三个节点状态相同。

这就是经典的：

```txt
Replicated State Machine（复制状态机）
```

Raft 本身主要负责让多个节点拥有一致的、有序的日志；具体状态机可以是 KV、数据库元数据、配置等。论文第 2 节就是从 Replicated State Machine 开始建立这个模型。

### 四、Raft 节点有三个状态

每个节点始终处于：

```txt
Follower
Candidate
Leader
```

也就是：

```txt
              ┌───────────────┐
              │    Follower   │
              └───────┬───────┘
                      │
              election timeout
                      ↓
              ┌───────────────┐
              │   Candidate   │
              └───────┬───────┘
                      │
                 majority votes
                      ↓
              ┌───────────────┐
              │     Leader    │
              └───────────────┘
```

Hashicorp 的 Go Raft 实现也是这个三状态模型：Follower、Candidate、Leader。

### 五、Term：Raft 的逻辑时间

Raft 有一个非常重要的概念：

```txt
Term
```

你可以把 Term 理解成：

```txt
第 1 任 Leader
第 2 任 Leader
第 3 任 Leader
...
```

例如：

```txt
Term 1
Leader = A

Term 2
Leader = B

Term 3
Leader = C
```

Term 是单调递增的逻辑时钟。

节点保存：

```go
currentTerm uint64
```

当节点发现：

```txt
收到 term = 10 的消息
```

而自己：

```txt
currentTerm = 8
```

那么它知道：

```txt
我落后了。
```

于是：

```txt
currentTerm = 10
state = Follower
```

反过来：

```txt
自己 term = 10
收到 term = 8
```

则拒绝这个消息。

### 六、Leader Election：Leader 怎么选出来？

假设：

```txt
A
B
C
```

一开始：

```txt
A = Follower
B = Follower
C = Follower
```

Leader 不存在。

Follower 都在等待 Leader 的 heartbeat。

如果：

```txt
A
```

先超时：

```txt
Election Timeout
```

A：

```txt
Follower
   ↓
Candidate
```

然后：

```txt
term++
```

例如：

```txt
Term 1
```

A 给自己投票：

```txt
A → vote A
```

然后向 B、C 请求投票：

```txt
A
├── RequestVote → B
└── RequestVote → C
```

假设：

```txt
B → vote A
C → vote A
```

那么：

```txt
A = 3 votes
```

超过 majority：

```txt
3 / 3
```

A 成为：

```txt
Leader
```

### 七、为什么 Election Timeout 要随机？

这个细节非常重要。

假设：

```txt
A timeout = 200ms
B timeout = 200ms
C timeout = 200ms
```

那么：

```txt
A → Candidate
B → Candidate
C → Candidate
```

大家都给自己投票：

```txt
A = 1
B = 1
C = 1
```

没有人拿到 majority。

于是：

```txt
Split Vote
```

Raft 的做法是随机化 election timeout。

例如：

```txt
A = 173ms
B = 241ms
C = 198ms
```

于是：

```txt
173ms
 ↓
A 超时
 ↓
A Candidate
 ↓
请求投票
```

B/C 还没超时，就可以把票给 A。

这大幅降低 split vote 的概率。

Raft 论文明确把 randomized timers 作为 Leader Election 的关键设计。

### 八、Leader 是怎么维持统治的？

Leader 会不断发送：

```txt
Heartbeat
```

实际上 heartbeat 使用的是：

```txt
AppendEntries RPC
```

只不过没有新的 log entry。

所以：

```txt
Leader
 ├──── AppendEntries ───→ Follower B
 └──── AppendEntries ───→ Follower C
```

Follower 收到：

```txt
AppendEntries
```

就知道：

```txt
Leader 还活着。
```

于是重置 election timer。

### 九、Leader 挂掉以后怎么办？

假设：

```txt
A = Leader
B = Follower
C = Follower
```

然后：

```txt
A 💥
```

现在：

```txt
B
C
```

都收不到 heartbeat。

经过 election timeout：

```txt
B → Candidate
```

B：

```txt
term++
```

然后：

```txt
RequestVote
```

C：

```txt
Vote B
```

于是：

```txt
B = Leader
```

整个集群继续工作。

所以 Raft 的核心容错能力之一就是：

```txt
Leader Crash
     ↓
Election
     ↓
New Leader
     ↓
Continue
```

### 十、为什么必须是 Majority？

这是 Raft 最重要的设计之一。

假设 5 个节点：

```txt
A B C D E
```

majority：

```txt
3
```

也就是说：

```txt
5 → 3
7 → 4
9 → 5
```

公式：

```txt
floor(N / 2) + 1
```

#### 为什么不是 2/5？

因为两个多数集合必须存在交集。

例如：

```txt
Cluster:

A B C D E
```

一个 majority：

```txt
A B C
```

另一个 majority：

```txt
C D E
```

一定有：

```txt
C
```

**重叠。**

这非常关键。

因为：

```txt
任何两个 majority 都至少有一个共同节点。
```

因此一个已经 committed 的日志条目不会轻易消失。

### 十一、Log Replication：真正的核心

假设：

```txt
Leader = A

A: []
B: []
C: []
```

客户端：

```txt
SET x 10
```

发送：

```txt
Client
   ↓
Leader A
```

A 先追加：

```txt
A:
[SET x 10]
```

然后复制：

```txt
A ─────→ B
A ─────→ C
```

结果：

```txt
A: [SET x 10]
B: [SET x 10]
C: [SET x 10]
```

多数确认：

```txt
A ✓
B ✓
C ✓
```

于是：

```txt
commitIndex = 1
```

Leader 把：

```txt
SET x 10
```

应用到 State Machine：

```txt
x = 10
```

然后通知 follower：

```txt
commitIndex = 1
```

Followers 也 apply：

```txt
x = 10
```

### 十二、Log Entry 里有什么？

一个 Raft log entry 通常类似：

```go
type LogEntry struct {
    Term uint64
    Command []byte
}
```

例如：

```txt
Index   Term   Command
──────────────────────────
1       1      SET x 10
2       1      SET y 20
3       2      SET x 30
```

所以每条日志有：

```txt
Index
Term
Command
```

### 十三、为什么需要 Index + Term？

因为 Raft 需要判断：

```txt
两个节点某个位置上的 log 是否一致。
```

例如：

Leader:

```txt
Index Term
1     1
2     1
3     2
4     2
```

Follower：

```txt
Index Term
1     1
2     1
3     3
```

虽然：

```txt
Index 3
```

都有。

但是：

```txt
Term:
Leader = 2
Follower = 3
```

说明它们不一致。

于是 Leader 可以让 follower 回退并修复日志。

### 十四、Log Matching Property

这是 Raft Safety 的核心。

Raft 有一个非常重要的性质：

```txt
如果两个日志在某个 index 上拥有相同的 term，那么这两个日志在这个 index 之前的所有 entry 都必须相同。
```

例如：

A:

```txt
1/1
2/1
3/2
4/2
```

B：

```txt
1/1
2/1
3/2
4/2
```

如果：

```txt
index=3
term=2
```

一致，那么：

```txt
1、2、3
```

也必须一致。

这个性质来自 AppendEntries 的一致性检查。

### 十五、Leader 如何修复 Follower 的错误日志？

这是 Raft 很漂亮的地方。

假设：

Leader:

```txt
1/1
2/1
3/2
4/2
5/3
```

Follower：

```txt
1/1
2/1
3/2
4/4
```

Follower 的：

```txt
4/4
```

是错误的。

Leader 发送：

```txt
prevLogIndex = 3
prevLogTerm  = 2
entries = [4/2, 5/3]
```

Follower：

```txt
index=3
term=2
```

匹配。

于是：

```txt
删除自己的 4/4
```

然后：

```txt
追加 Leader 的：

4/2
5/3
```

最终：

```txt
Leader:
1/1
2/1
3/2
4/2
5/3

Follower:
1/1
2/1
3/2
4/2
5/3
```

一致。

### 十六、AppendEntries 为什么可以同时做 Heartbeat？

因为：

```txt
AppendEntries
```

既可以：

```txt
entries = []
```

也可以：

```txt
entries = [...]
```

所以：

```txt
entries = []
```

就是 heartbeat。

```txt
entries = [SET x 10]
```

就是日志复制。

这样 Raft 不需要两个完全不同的 RPC。

### 十七、Commit 到底是什么意思？

这个概念非常容易搞错。

假设：

```txt
Leader:
[SET x 10]
```

复制给：

```txt
B
```

现在：

```txt
A ✓
B ✓
C ❌
```

3 节点中已经 2 个有：

```txt
SET x 10
```

因此：

```txt
majority = 2
```

可以认为：

```txt
entry committed
```

但是：

```txt
写进日志 ≠ committed。
```

这是 Raft 初学者最容易混淆的地方。

### 十八、Committed ≠ Applied

Raft 通常有两个不同概念：

```txt
commitIndex
lastApplied
```

例如：

```txt
Log:

1 SET x 10
2 SET y 20
3 SET z 30
```

可能：

```txt
commitIndex = 3
lastApplied = 1
```

表示：

```txt
1 已经 apply
2、3 已经 committed
但是还没 apply
```

最终：

```txt
lastApplied → 3
```

State Machine：

```txt
x = 10
y = 20
z = 30
```

### 十九、Raft 的 Safety 到底保证什么？

Raft 真正厉害的不是“选 Leader”。

而是：

**已经 committed 的东西不能被另一个 Leader 覆盖。**

这就是 Raft Safety 的核心。

论文定义了几个重要性质，包括 Election Safety、Leader Append-Only、Log Matching、Leader Completeness、State Machine Safety。

### 二十、Election Safety

一个 Term：

```txt
最多只能有一个 Leader。
```

例如：

```txt
Term 5

A = Leader
B = Leader
```

这种情况不能发生。

为什么？

因为两个 Leader 都需要 majority。

但是两个 majority 必须交集。

同一个节点一个 Term 只能投一次票。

因此不可能两个候选人都得到 majority。

### 二十一、Leader Append-Only

Leader 一旦产生：

```txt
log entry
```

不会修改自己的历史日志。

也就是说：

```txt
Leader
```

不会：

```txt
修改：
index 3
```

而是通过 follower 的日志修复机制让其他节点追随 Leader 的日志。

### 二十二、Leader Completeness

这是 Raft 最关键也最难理解的安全性质之一。

**如果一个 log entry 在某个 term 已经 committed，那么所有更高 term 的 Leader 都必须包含这个 entry。**

这就是为什么 Leader Election 时不能随便让一个“日志落后”的节点成为 Leader。

### 二十三、Candidate 为什么必须拥有“足够新”的 Log？

RequestVote 不只是：

```txt
“你投我吧”
```

它还携带：

```txt
lastLogIndex
lastLogTerm
```

Follower 会判断：

```txt
你的 log 是否至少和我一样新？
```

一般比较：

```txt
lastLogTerm
```

如果相同：

```txt
lastLogIndex
```

越大越新。

所以：

```txt
Candidate Log
```

如果太旧：

```txt
拒绝投票
```

### 二十四、为什么这个机制能保证 committed entry 不丢？

这是 Raft 非常漂亮的一点。

假设：

```txt
5 个节点：

A B C D E
```

某 entry 已经 committed：

```txt
A B C
```

至少三个节点拥有它。

然后：

```txt
A B C 挂了
```

剩下：

```txt
D E
```

集群已经没有 majority：

```txt
2 / 5
```

因此不能继续安全地选出新的 Leader。

也就是说：

**只要一个 committed entry 存在于 majority，那么任何未来 majority 都必须和它重叠。**

于是未来的新 Leader 必然从拥有 committed entry 的节点集合里产生。

这就是：

```txt
Majority Intersection
```

+：

```txt
Election Restriction
```

共同保证：

```txt
Committed Entry
      ↓
不会丢
```

### 二十五、网络分区是理解 Raft 最好的例子

假设 5 个节点：

```txt
A B C | D E
```

网络断了：

```txt
A B C       D E
```

左边有：

```txt
3
```

右边：

```txt
2
```

原 Leader：

```txt
A
```

在左边。

那么：

```txt
A B C
```

仍然可以继续：

```txt
commit
```

因为：

```txt
3 >= majority(3)
```

但是：

```txt
D E
```

不能提交新的日志。

因为：

```txt
2 < 3
```

所以：

**Raft 在网络分区时宁愿停止写入，也不会让两个分区都成功提交冲突状态。**

这就是它的 Safety / Availability 权衡。

### 二十六、为什么 3 节点可以挂 1 台？

3 节点：

majority：

```txt
2
```

挂：

```txt
1
```

剩：

```txt
2
```

还能工作。

5 节点：

majority：

```txt
3
```

可以挂：

```txt
2
```

7 节点：

majority：

```txt
4
```

可以挂：

```txt
3
```

所以：

| 集群 | Majority | 最大容忍故障 |
| ---- | -------- | ------------ |
| 3    | 2        | 1            |
| 5    | 3        | 2            |
| 7    | 4        | 3            |

公式：

```txt
容忍故障数 = floor((N - 1) / 2)
```

这也是为什么生产系统常见：

```txt
3 nodes
5 nodes
```

而不是：

```txt
4 nodes
6 nodes
```

因为偶数节点不会增加比相邻奇数节点更多的故障容忍能力。

### 二十七、Raft 到底保证什么？

你可以把 Raft 的能力总结成：

```txt
                    Raft
                      │
          ┌───────────┼───────────┐
          ↓           ↓           ↓
       Leader       Log        Safety
       Election   Replication
          │           │           │
          ↓           ↓           ↓
       选主       日志复制      保证一致性
```

但它不保证：

```txt
❌ 网络一定可用
❌ 请求一定成功
❌ 系统一定实时
❌ 所有节点同时在线
❌ 永远有 Leader
```

它保证的是：

```txt
在满足多数节点可通信等条件时，系统能够形成一致的决定；即使发生部分故障，也不会为了继续服务而产生相互冲突的 committed 状态。
```

### 二十八、Raft 的完整 RPC

Raft 最核心其实就两个 RPC：

```txt
RequestVote
AppendEntries
```

另外论文还有：

```txt
InstallSnapshot
```

用于 Snapshot。

#### RequestVote

Candidate：

```txt
Candidate
   │
   │ RequestVote
   ↓
Follower
```

携带：

```txt
term
candidateId
lastLogIndex
lastLogTerm
```

Follower 返回：

```txt
voteGranted
```

#### AppendEntries

Leader：

```txt
Leader
   │
   │ AppendEntries
   ↓
Follower
```

携带：

```txt
term
leaderId
prevLogIndex
prevLogTerm
entries[]
leaderCommit
```

其中：

```txt
prevLogIndex
prevLogTerm
```

就是用于 Log Consistency Check。

### 二十九、Snapshot 为什么需要？

假设你的 Raft 已经运行一年：

```txt
Log:

1 SET...
2 SET...
3 SET...
...
10,000,000 SET...
```

如果每次新节点加入都要：

```txt
从 1 replay 到 10,000,000
```

显然很慢。

于是可以做：

```txt
Snapshot
```

例如：

```txt
Log:
1
2
3
...
100000
```

状态机当前：

```txt
x = 100
y = 200
z = 300
```

生成：

```txt
Snapshot @ index 100000
```

然后删除旧日志：

```txt
Snapshot
index = 100000
term = 20
state = {
   x:100,
   y:200,
   z:300
}

Remaining Log:
100001
100002
...
```

新节点只需要：

```txt
InstallSnapshot
      ↓
恢复 state
      ↓
继续 replay 后面的 log
```

### 三十、这也是 etcd 为什么和 Raft 有关系

你前面问过分布式锁。

现在可以把它串起来：

```txt
                   etcd
                     │
              ┌──────┴──────┐
              │             │
             Raft         KV Store
              │
      ┌───────┼───────┐
      ↓       ↓       ↓
     E1      E2      E3
```

etcd 用 Raft 维护集群状态的一致性。

然后在此基础上实现：

```txt
Lease
Watch
Election
Mutex
```

所以：

```txt
分布式锁
   ↓
etcd
   ↓
Raft
   ↓
Consensus
```

这就是为什么我之前建议你把 Raft 和 etcd 分布式锁一起学习。

### 三十一、Raft 和数据库是什么关系？

再回到你之前问的分布式事务。

Raft：

```txt
Client
   ↓
Raft
   ↓
Replicated Log
   ↓
State Machine
```

数据库可以作为：

```txt
State Machine
```

例如：

```txt
SET key value
```

最终：

```txt
Raft
 ↓
Log
 ↓
KV State Machine
 ↓
map[key] = value
```

或者：

```txt
Raft
 ↓
Log
 ↓
Metadata State Machine
 ↓
Distributed Storage Metadata
```

所以：

**Raft 是共识层，不是数据库。**

### 三十二、Raft 和分布式事务有什么区别？

这个特别值得你现在区分。

#### Raft

解决：

```txt
多个节点：

A B C

对：

Operation 1
Operation 2
Operation 3

达成一致
```

属于：

**Consensus**

#### Saga

解决：

```txt
Order
 ↓
Inventory
 ↓
Payment
 ↓
Shipping
```

这些不同业务服务之间：

```txt
如何最终达到业务一致状态。
```

属于：

```txt
Distributed Transaction / Business Consistency
```

所以：

```txt
Raft
 ↓
集群内部共识
```

而：

```txt
Saga
 ↓
业务服务之间的一致性
```

两者解决的是不同层次的问题。

### 三十三、Raft 最容易踩的几个理解坑

#### 坑 1：Raft = Leader Election

不对。

Leader Election 只是：

```txt
Raft
 ├── Leader Election
 ├── Log Replication
 ├── Safety
 ├── Membership Changes
 └── Snapshot
```

#### 坑 2：Leader 收到请求就算成功

不对。

通常流程是：

```txt
Client
 ↓
Leader
 ↓
Append Log
 ↓
Replicate
 ↓
Majority
 ↓
Commit
 ↓
Apply
 ↓
Response
```

#### 坑 3：复制到一个节点就能 commit

不对。

需要：

```txt
Majority
```

#### 坑 4：Follower 永远不能有自己的 Log

不准确。

Follower 可以因为网络延迟暂时拥有：

```txt
旧日志
```

或者存在未提交的日志。

关键是：

**Leader 最终会通过 AppendEntries 修复它。**

#### 坑 5：Leader 永远不会改变

不对。

Leader A
↓
Crash
↓
Leader B

Term 也会增加。

### 三十四、Raft 最重要的几个变量

如果你以后自己实现 Raft，建议把这几个变量背下来。

```go
type Raft struct {
    // Persistent state
    currentTerm uint64
    votedFor    int
    log         []Entry

    // Volatile state
    commitIndex uint64
    lastApplied uint64

    // Leader only
    nextIndex  []uint64
    matchIndex []uint64
}
```

其中：

`currentTerm`

当前任期。

`votedFor`

这个 term 给谁投票。

`log`

复制日志。

`commitIndex`

最高已经 committed 的日志 index。

`lastApplied`

最高已经应用到 State Machine 的 index。

`nextIndex`

Leader 对每个 follower：

```txt
下一条准备发送的 log 从哪里开始。
```

`matchIndex`

Leader 认为每个 follower：

```txt
已经复制到哪里。
```

### 三十五、如果你准备自己实现 Raft

我建议不要直接写完整版本。

按照：

```txt
Phase 1
Leader Election
```

然后：

```txt
Phase 2
Heartbeat
```

然后：

```txt
Phase 3
Log Replication
```

然后：

```txt
Phase 4
Commit
```

然后：

```txt
Phase 5
Leader Crash
```

然后：

```txt
Phase 6
Follower Crash
```

然后：

```txt
Phase 7
Network Partition
```

最后：

```txt
Phase 8
Persistence
```

```txt
Phase 9
Snapshot
```

```txt
Phase 10
Membership Changes
```

这会比一次性实现完整 Raft 容易很多。

### 三十六、你应该重点理解的“完整时序”

最后我用一个 3 节点例子把整个 Raft 串起来。

初始：

```txt
A     B     C
│     │     │
F     F     F
```

A 超时：

```txt
A → Candidate
term = 1
```

请求投票：

```txt
A ──RequestVote──→ B
A ──RequestVote──→ C
```

获得：

```txt
A = 3 votes
```

于是：

```txt
A = Leader
B = Follower
C = Follower
```

发送 heartbeat：

```txt
A ──AppendEntries──→ B
A ──AppendEntries──→ C
```

客户端：

```txt
SET x = 10
```

请求：

```txt
Client
   ↓
A
```

A：

```txt
Log[1] = SET x 10
```

复制：

```txt
A ─────→ B
A ─────→ C
```

得到：

```txt
A ✓
B ✓
C ✓
```

于是：

```txt
commitIndex = 1
```

Apply：

```txt
A: x = 10
B: x = 10
C: x = 10
```

然后：

```txt
A 💥
```

B 超时：

```txt
B → Candidate
term = 2
```

请求投票：

```txt
B → C
```

C 投给 B：

```txt
B = Leader
```

此时：

```txt
B:
[SET x 10]

C:
[SET x 10]
```

所以：

```txt
x = 10
```

不会丢。

这就是 Raft 从：

```txt
选主
 ↓
复制
 ↓
多数确认
 ↓
提交
 ↓
Leader 故障
 ↓
重新选主
 ↓
保持已提交状态
```

完整走一遍。

### 三十七、推荐学习资源

官方 Raft 网站

[Raft 官方网站](https://raft.github.io/?utm_source=chatgpt.com)

这里有论文、可视化、演讲和实现列表。官方明确提供了 Raft visualization，非常适合刚开始学习。

**原始论文**

[Raft Extended Paper — Ongaro & Ousterhout](https://raft.github.io/raft.pdf?utm_source=chatgpt.com)

这是最重要的资料。建议重点阅读：

```txt
2. Replicated state machines
5. The Raft consensus algorithm
5.1 Raft basics
5.2 Leader election
5.3 Log replication
5.4 Safety
6. Cluster membership changes
7. Log compaction
```

[MIT 6.033 的课程资料](https://web.mit.edu/6.033/www/recitations/20-raft.shtml?utm_source=chatgpt.com)也建议把论文第 5、6 节作为核心阅读，并推荐配合可视化学习。

**中文翻译**

[Raft 中文论文翻译](https://ying-zhang.cn/dist/2014-raft-cn/?utm_source=chatgpt.com)

适合你阅读原论文时对照。

**可视化**

[Raft 官方可视化](https://raft.github.io/?utm_source=chatgpt.com#implementations)

另外还有一个比较直观的[交互式 Raft Lab](https://raftprotocol.com/raft-lab.html?utm_source=chatgpt.com)，可以观察选举、heartbeat、quorum、partition recovery 等过程。

**Go 源码**

[HashiCorp Raft Go 实现](https://github.com/hashicorp/raft?utm_source=chatgpt.com)

如果你准备以后用 Go 真正实现/阅读生产级 Raft，可以把它作为第二阶段源码阅读材料。

**最后给你一张“Raft 全景图”**

```txt
                         Client
                           │
                           ↓
                       ┌───────┐
                       │Leader │
                       └───┬───┘
                           │
                 AppendEntries RPC
                    ┌──────┴──────┐
                    ↓             ↓
                Follower B    Follower C
                    │             │
                    └──────┬──────┘
                           ↓
                      Majority ACK
                           │
                           ↓
                       COMMITTED
                           │
                           ↓
                     State Machine
                           │
                           ↓
                    Replicated State
```

而 Leader 本身又通过：

```txt
Election Timeout
       ↓
    Candidate
       ↓
   RequestVote
       ↓
    Majority
       ↓
     Leader
       ↓
   Heartbeat
       ↓
    维持 Leader
```

整个 Raft 的核心其实就是三个词：

**Leader + Log + Majority**

然后围绕这三个东西解决：

**选谁当 Leader、日志怎么复制、什么情况下算 Commit、Leader 挂了怎么办、网络分区怎么办、旧日志怎么修复、为什么已经 Commit 的数据不会丢。**

如果你准备自己用 Go 实现 Raft，下一步最值得深入的是 `RequestVote` 和 `AppendEntries` 两个 RPC 的完整协议状态机，尤其是 `nextIndex / matchIndex / commitIndex` 三个变量。把这三个变量彻底搞懂，Raft 的实现基本就打通一半了。


----------------------------------------------第二次询问ChatGPT---------------------------------------------------------


# Raft 共识算法：从原理到工程实践

Raft 是分布式系统中非常重要的一种一致性共识算法（Consensus Algorithm）。如果你正在开发 Go 分布式服务、分布式存储、元数据管理服务，或者想理解 etcd、Consul 等系统的实现，Raft 值得深入学习。

学习 Raft，最重要的不是先记住各种规则，而是理解一个核心问题：

当多个节点各自保存一份数据，网络又可能延迟、断开，甚至部分节点宕机时，如何让这些节点对同一批操作达成一致，并且不会因为故障而破坏已经确认的数据？

我会按照「问题背景 → 核心架构 → 选举 → 日志复制 → 安全性 → 故障恢复 → Go 工程实践」的顺序讲解，最后附上论文、官方文档和源码学习资源。

## 一、Raft 要解决什么问题？

假设有 3 个服务器节点，它们共同维护一个分布式 KV 数据库。

分布式 KV 服务

目标：所有节点对已提交的操作保持一致

Node A

Leader

日志副本

Node B

Follower

日志副本

Node C

Follower

日志副本

Leader 负责协调写入，Follower 保存日志副本。真正执行状态机操作之前，需要满足提交条件。

例如，客户端执行：

```
SET user:1001 "Alice"
SET user:1002 "Bob"
DELETE user:1001
```

假设 A、B、C 都需要维护相同的 KV 状态，就会面临几个问题：

1. 谁负责接收和排序写请求？ 如果每个节点都可以独立决定操作顺序，节点之间很容易产生冲突。
2. 如何保证写入一致？ A 已经写入，但 B、C 还没收到，甚至 A 突然宕机，该怎么办？
3. 如何选出新的协调者？ 原来的 Leader 故障之后，剩余节点如何选出新的 Leader？
4. 如何避免已确认的数据丢失？ 新 Leader 不能简单地使用自己的日志覆盖其他节点的数据。

Raft 的思路是：将问题拆解为几个相对独立的部分，使用明确的规则来处理选举、日志复制和安全性。

这里需要先区分两个概念：

- 复制（Replication）：把操作日志复制到多个节点。
- 共识（Consensus）：让节点对哪些日志已经提交、哪些操作的顺序生效达成一致。

日志复制是实现共识的重要组成部分，但复制到某个节点并不意味着该操作已经提交。

## 二、Raft 的整体架构

### 1. 三种节点角色

Raft 中每个节点在任意时刻处于以下三种角色之一。

Leader（领导者）

负责处理客户端写请求、确定日志顺序、向其他节点复制日志，并推动日志提交。正常情况下，一个任期内最多只有一个合法 Leader。

Follower（跟随者）

被动响应 Leader 的 RPC，接收和保存日志。如果收到客户端请求，通常会将请求转发给 Leader，或者告知客户端 Leader 的地址。

Candidate（候选人）

当节点长时间没有收到有效 Leader 的通信时，会发起选举，增加自己的任期，并请求其他节点投票。

典型状态转换如下：

```
             超时 / 开始选举
      ┌────────────────────────┐
      │                        ▼
  Follower ───────────────> Candidate
      ▲                        │
      │                        │ 获得多数票
      │                        ▼
      └──────────────────── Leader
               发现更高任期或有效 Leader
```

实际状态转换还包括 Candidate 转为 Follower、Leader 因发现更高任期而退回 Follower，以及选举失败后重试等情况。

### 2. 两个关键机制

Raft 的设计可以先记成两条主线：

- 选举机制：通过任期和投票选出 Leader。
- 日志复制机制：通过日志匹配、多数派提交和安全规则，使集群对已提交的操作保持一致。

此外，Raft 还需要持久化状态、快照、成员变更等机制，才能成为可用于生产环境的共识协议。

## 三、任期 Term：Raft 的逻辑时间

理解 Raft，首先要理解 Term（任期）。

Raft 将时间划分为连续递增的任期，每个任期用一个整数表示：

```
Term 1          Term 2          Term 3
   |               |               |
   v               v               v
选出 Leader A   选举失败/重试    选出 Leader C
```

每次节点开始新一轮选举时，会增加自己的 `currentTerm`。如果节点收到来自更高任期的有效 RPC，通常需要更新任期并退回 Follower。

任期有点像分布式系统中的逻辑时钟，它可以帮助节点判断消息是否过时，但任期不是现实时间，也不能单独证明某条日志已经提交。

### 1. 为什么需要任期？

假设：

- Term 5 时，A 是 Leader。
- A 与部分节点发生网络分区。
- 另一部分节点选出了 Term 6 的 Leader B。
- 网络恢复后，A 又向其他节点发送 Term 5 的消息。

B 所在的节点看到更高的 `currentTerm = 6` 后，就能识别 A 的旧任期消息，并拒绝继续接受它作为当前 Leader。

这可以避免旧 Leader 在新任期已经建立后继续正常领导集群。

### 2. Raft 节点需要维护哪些状态？

| 状态             | 含义                             | 是否必须持久化    |
| -------------- | ------------------------------ | ---------- |
| `currentTerm`  | 当前已知的最高任期                      | 是          |
| `votedFor`     | 当前任期投给了谁                       | 是          |
| `log[]`        | 复制日志                           | 是          |
| `commitIndex`  | 已知提交的最高日志索引                    | 通常不要求独立持久化 |
| `lastApplied`  | 已应用到状态机的最高索引                   | 取决于实现和恢复设计 |
| `nextIndex[]`  | Leader 认为各 Follower 下一条需要发送的索引 | 否，属于运行时状态  |
| `matchIndex[]` | Leader 确认各 Follower 已匹配的最高索引   | 否，属于运行时状态  |

注意：`commitIndex` 不需要像 `currentTerm` 和 `votedFor` 那样必须单独持久化，但恢复时必须能安全地重建提交进度。具体持久化策略取决于实现。

## 四、Leader 选举：如何选出唯一的领导者？

### 1. 选举超时机制

Raft 使用随机化的选举超时时间来降低多个节点同时发起选举的概率。

假设有三个节点 A、B、C：

- A 的选举超时是 300 ms。
- B 的选举超时是 450 ms。
- C 的选举超时是 380 ms。

这些数字只是示例，并非协议规定的固定值。

如果三个节点都没有收到有效 Leader 的通信，A 最先超时，就会开始选举。

### 2. 选举的具体过程

阶段 1：A 成为 Candidate

选举超时 → currentTerm 加一 → 给自己投票

阶段 2：发送 RequestVote RPC

A 向 B、C 请求投票

B 投赞成票

C 暂未投票

阶段 3：A 获得多数票，成为 Leader

A + B = 2/3，满足多数派条件

Raft 的投票规则包含几个关键约束：

1. 节点只会在同一个任期内投出至多一张赞成票（`votedFor`）。
2. 候选人的日志必须足够新，才有资格获得投票。
3. 候选人必须获得集群配置中严格超过一半的投票，才能成为 Leader。
4. 如果节点发现更高任期，会更新任期并退回 Follower。
5. 如果选举长期无法成功，节点会在后续超时后重试。

### 3. 为什么投票前要检查日志？

假设有三个节点，集群中一条已经提交的日志存在于 A 和 B：

```
A: [1, 2, 3, 4, 5]
B: [1, 2, 3, 4, 5]
C: [1, 2, 3]
```

如果 C 的选举超时最先发生，它不能仅仅因为自己先超时就成为 Leader。A 和 B 会检查 C 的日志是否足够新。

Raft 比较候选人日志新旧的规则是：

1. `lastLogTerm` 更大的日志更新。
2. 如果 `lastLogTerm` 相同，`lastLogIndex` 更大的日志更新。

因此，如果 C 的日志落后，它就不能从持有更新日志的节点那里获得赞成票。

这条规则是 Raft 安全性的关键基础之一。 它与后面要讲的 Leader Completeness（领导者完整性）性质共同保证已提交的日志不会被新 Leader 丢弃。

### 4. 为什么要使用随机超时？

如果所有节点的超时时间完全相同，它们可能同时发起选举，每个节点给自己投票，导致没有任何候选人获得多数票。

这叫作选举分裂（Split Vote）。

随机超时让不同节点更有可能依次发起选举，减少这种情况。它并不能保证每轮选举必然成功，但在网络和节点最终稳定、超时设置合理的情况下，能够帮助集群恢复选举进展。

## 五、日志复制：Raft 如何让数据保持一致？

这是 Raft 最核心的部分之一。

### 1. 日志的结构

Raft 把客户端操作追加到日志中。每条日志至少有一个索引、一个任期，以及操作内容。

例如：

| Log Index | Term | Command    |
| --------- | ---- | ---------- |
| 1         | 1    | `SET x 10` |
| 2         | 1    | `SET y 20` |
| 3         | 2    | `SET x 30` |
| 4         | 2    | `DELETE y` |

日志索引表示操作在序列中的位置，任期标识该条日志由哪个任期的 Leader 创建。

每个节点都保存日志，但在复制期间，节点之间可能暂时存在差异。Raft 的目标不是要求每一时刻的日志完全相同，而是保证已提交的日志具有一致性，并且最终让可恢复的节点日志收敛。

### 2. 一次写入的完整过程

假设客户端向 Leader A 发送：

```
SET account:100 500
```

写入大致经历以下步骤：

① 客户端 → Leader A

提交写请求。

② Leader A 追加日志

生成新日志条目，持久化到自己的稳定存储。

③ Leader A → Follower B、C

通过 AppendEntries RPC 发送日志，并根据响应跟踪复制进度。

④ 多数派复制成功

满足 Raft 的提交规则后，Leader 推进 commitIndex。

⑤ 应用到状态机并响应客户端

提交的日志按顺序应用到 KV 状态机，Follower 随后获知提交进度并应用日志。

这里有一个非常重要的细节：日志被写入本地磁盘、日志复制到其他节点、日志被提交、日志被应用到状态机，是四个不同的事件。

- 追加（Append）：日志进入本地日志序列。
- 复制（Replicate）：其他节点也持久化了相应日志。
- 提交（Commit）：Raft 协议确认这条日志属于已经确定的日志前缀。
- 应用（Apply）：状态机实际执行这条已提交的操作。

不能简单地把“收到两个节点的成功响应”当成全部流程结束；还要考虑任期、日志匹配条件、持久化顺序以及提交规则。

### 3. AppendEntries：不仅用于追加日志，还用于维持 Leader 权威

Leader 通过 `AppendEntries RPC` 向 Follower 发送日志。这个 RPC 还有两个重要作用：

- 定期发送空的心跳请求，让 Follower 知道 Leader 仍然存活。
- 检查前置日志是否匹配，避免在不一致的日志后继续追加。

一个简化的请求结构如下：

```
type AppendEntriesArgs struct {
    Term         uint64
    LeaderID     uint64
    PrevLogIndex uint64
    PrevLogTerm  uint64
    Entries      []LogEntry
    LeaderCommit uint64
}
```

这只是用于讲解的简化结构，不是 etcd Raft 的实际消息定义。

其中最值得关注的是：

- `Term`：Leader 的任期。
- `PrevLogIndex` 和 `PrevLogTerm`：新日志之前的那条日志应该是什么。
- `Entries`：本次需要追加的日志。
- `LeaderCommit`：Leader 当前已知的提交索引。

Follower 收到请求后，会先检查任期，再检查前置日志是否匹配。

例如，Leader 发送：

```
PrevLogIndex = 4
PrevLogTerm  = 2
Entries      = [(index=5, term=3, command="SET x 40")]
```

Follower 只有在自己的第 4 条日志确实存在、且任期为 2 时，才能基于该前缀接受新日志。

如果不匹配，Follower 会拒绝此次请求，Leader 再根据返回信息调整复制位置并重试。

### 4. 日志不一致时如何修复？

假设发生过多次 Leader 更替：

```
Leader A: [1, 2, 3, 4, 5]
Follower B: [1, 2, 3, 4]
Follower C: [1, 2, 3, 9, 10]
```

这里的数字仅用于表示日志内容，实际协议还需要比较每个位置的任期。

新 Leader 会通过前置日志匹配检查发现不一致，并调整发送位置，直到找到共同的日志前缀。随后，Follower 删除冲突的未提交日志及其后续条目，接受新 Leader 提供的日志。

注意：Raft 不允许通过这种方式覆盖已经提交的日志。 如果一条日志已经提交，正确的 Leader 选举与日志匹配规则会确保后续 Leader 保留它。

## 六、commitIndex：为什么多数派复制不等于所有日志都能直接提交？

这是理解 Raft 最容易出错的地方。

### 1. 多数派的计算

一个有 \\(N\\) 个投票节点的集群，需要的多数派数量是：

\\[ Q=\left\lfloor\frac{N}{2}\right\rfloor+1 \\]

## 不同集群规模的容错能力

| 节点总数 | 需要的多数派 | 最多可容忍故障数 |
| ---- | ------ | -------- |
| 3    | 2      | 1        |
| 5    | 3      | 2        |
| 7    | 4      | 3        |

这里假设集群使用固定成员配置，节点均为有投票权的成员。网络分区、磁盘故障等也可能让节点无法参与多数派。

例如，在 3 节点集群中，A 将日志持久化到自己的磁盘，并收到 B 已持久化该日志的确认，就可以满足多数派复制数量。

但还需要考虑日志任期。

### 2. Raft 的关键提交限制

Raft 的标准提交规则可以概括为：

> Leader 可以通过多数派复制来提交自己当前任期的日志条目。提交当前任期的某条日志后，它之前的日志前缀也随之提交。

更严格地说，Leader 通常推进 `commitIndex` 时，需要找到一个索引 \\(N\\)，满足：

\\[ \operatorname{majority}\bigl(matchIndex \ge N\bigr) \\]

并且：

\\[ log[N].term=currentTerm \\]

这是对固定成员配置下标准 Raft 提交规则的简化表达。成员变更时，多数派计算还要遵循相应配置协议。

为什么有第二个条件？看下面的反例。

假设：

```
Term 2:
A (Leader): [1, 2, 3]
B:         [1, 2, 3]
C:         [1, 2]
```

第 3 条日志属于 Term 2，已经在 A、B 两个节点上复制。

现在假设 A 宕机，C 成为 Term 3 的 Leader。C 的日志比较旧，但如果没有合适的投票约束，它可能覆盖 A、B 之前保存的日志。

Raft 通过日志新旧检查阻止这种情况：持有更新日志的节点不会给日志较旧的 C 投票。因此，这种不安全的 Leader 不应该被选出。

再考虑另一种情形：一条旧任期日志曾经复制到多数派，但尚未通过当前任期的提交规则确定提交状态。Raft 不允许 Leader 单凭旧任期日志的多数派复制，就将其直接视为已提交。新 Leader 需要通过当前任期的日志提交规则来确立提交进度。

这正是 Raft 提交规则不能简单概括为“复制到多数节点就行”的原因。

## 七、Raft 的五条安全性核心规则

Raft 的安全性来自多条规则相互配合，而不是某一个单独的机制。官方论文详细讨论了这些性质。

[image](https://www.google.com/s2/favicons?domain=https://raft.github.io\&sz=32)

Raft

+1



| 性质                   | 含义                                    |
| -------------------- | ------------------------------------- |
| Election Safety      | 同一个任期内，最多只能有一个合法 Leader               |
| Leader Append-Only   | Leader 正常运行时，只会向自己的日志追加条目，不会任意改写已有日志  |
| Log Matching         | 如果两个日志在相同索引处的任期相同，那么这两个日志在该位置及之前的前缀相同 |
| Leader Completeness  | 一条已提交日志必须存在于之后任期的 Leader 日志中          |
| State Machine Safety | 不同节点不会在同一个日志索引处应用不同的命令                |

其中最重要的是 Leader Completeness。

它依赖多个条件共同成立：

- 节点投票前会检查候选人的日志是否足够新。
- 同一任期内不能重复投票。
- 多数派集合之间必然存在交集。
- 日志复制和提交遵循 Raft 的规则。

由此才能推导出：一条已提交的日志不会被后续合法 Leader 丢弃。

## 八、故障场景：Leader 宕机之后会发生什么？

假设 A、B、C 组成一个 3 节点集群，A 是 Leader。

### 场景一：A 宕机

1. B、C 一段时间没有收到有效 Leader 通信。
2. 其中一个节点先触发选举超时。
3. 该节点进入 Candidate，增加任期并向其他节点请求投票。
4. 获得多数票后，新 Leader 开始工作。
5. 新 Leader 根据自己的日志与 Follower 的日志匹配情况继续复制和提交。

由于 3 节点集群只需要 2 个节点形成多数派，单节点故障通常不会阻止集群继续工作。

### 场景二：A 与 B、C 发生网络分区

```
分区一：          分区二：

   A                B
 旧 Leader           |
                     C
                  多数派
```

A 无法联系 B、C，而 B、C 之间仍能通信。

B、C 可以选出新 Leader 并继续处理写入；A 无法获得多数派，因此不能安全地提交新的 Raft 写入。

网络恢复后，A 会通过更高任期消息发现自己已经过时，退回 Follower，并根据新 Leader 的日志进行同步。

这里有一个很重要的区别：Raft 保证安全性，不保证在任意网络故障下都能持续提供写服务。 没有多数派时，系统可能停止接受或提交新的写入。

### 场景三：Leader 收到了请求，但客户端超时

假设 A 已经把一条日志复制到多数派并提交，但在响应客户端之前宕机。

客户端只看到超时，并不知道写入是否成功。

这意味着 Raft 的共识安全性不等于业务层的“请求只执行一次”。客户端可能重试同一请求，因此生产系统通常还需要请求 ID、幂等处理或去重机制。

## 九、持久化、快照与成员变更

理解基本共识后，还需要关注三个工程问题。

### 1. 为什么持久化顺序很重要？

Raft 不能只在内存中记录任期和日志，否则节点重启后可能忘记已经投过的票，或丢失已经参与提交判断的日志。

关键要求包括：

- `currentTerm` 和 `votedFor` 必须可靠持久化。
- 日志必须在被计入持久化复制进度之前可靠写入。
- 在依赖某个持久化结果发送响应之前，要确保对应状态确实已经落盘。

如果程序先向 Leader 确认“日志已保存”，实际上却还没完成持久化，之后发生断电，就可能破坏 Raft 的安全性。

### 2. 为什么需要 Snapshot？

假设服务运行几年，日志不断增长：

```
Log: [1, 2, 3, 4, 5, ... 10000000]
```

每次重启都重放全部日志，会越来越慢。

快照（Snapshot）可以保存某个已应用索引处的状态机完整状态，并记录对应的日志索引与任期。系统在确认快照安全、能够恢复后，就可以压缩更早的日志。

恢复过程大致是：

```
加载快照
   ↓
恢复状态机状态
   ↓
加载快照之后保留的日志
   ↓
重放已提交但尚未应用的日志
   ↓
继续正常运行
```

快照不是简单地定期备份数据库文件。它必须与日志索引、任期、提交和应用进度正确衔接。

### 3. 集群成员变更为什么复杂？

假设集群原来有 A、B、C，要变成 A、B、C、D、E。

旧配置需要 2 票，新配置需要 3 票。如果直接切换配置，某些情况下新旧集群可能各自形成多数派，破坏安全性。

Raft 论文使用联合共识（Joint Consensus）解释安全的配置转换：在过渡阶段，需要同时满足旧配置和新配置的多数派条件。

不同实现的具体成员变更协议可能不同。例如，etcd Raft 使用的配置变更方式与论文中的联合共识方案存在差异，但仍然需要确保新旧配置之间有足够的安全重叠。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub

+1



因此，成员变更应当通过 Raft 实现提供的配置变更接口完成，而不是直接修改节点列表。

## 十、Raft 理论与文档资源

下面这些资料值得按顺序学习。我优先选择了原始论文、作者的博士论文、官方站点，以及 Go 生态中的实际实现。

[Raft Protocol. Raft protocol is a consensus algorithm… | by Dylan Wang | Medium](https://images.openai.com/static-rsc-4/J5VgNDr-zTCg9LtsUYl_Fjl_9SaReYWiyVcqQhwwg8E72SZr_AZBODX4TcqMgP0sOZJ2q5_cluEJjVBtqF0_d5wUvi210s-QMOq6BRh99n6Tj9SL9niSFPXaXCGmA60jSQgEfyQnC9l265rofgx4CxBCrDhYmaUc3vfRyvPAkYY?purpose=inline)

1\. Raft 官方网站

建议从这里开始

包括算法介绍、论文、交互式可视化、讲座、实现列表和其他研究论文。

[打开 raft.github.io](https://raft.github.io/index.html?utm_source=chatgpt.com)&#x20;

[image](https://www.google.com/s2/favicons?domain=https://raft.github.io\&sz=32)

Raft



[In Search of an Understandable Consensus Algorithm Diego](https://images.openai.com/static-rsc-4/cp_ilE6uUXq70e554MXcqB_TIKppi23cdlaoZiVK-3-SrayGFo_Y-y-ckeIxN90qvGcuMS1SNTKGnxjxH_OIaClNMW-xfAApVEYEU8Ggt5Z7VlUHM3xeYrchKcr2BF01YtR5DAUQYOP5rUxRZ8vcmIq1fDXYLEL-A30rbKm6BuA?purpose=inline)

2\. Raft 原始论文（Extended Version）

作者：Diego Ongaro、John Ousterhout。2014 年论文的扩展版本，是理解协议细节最重要的资料。

[在线阅读 PDF](https://raft.github.io/raft.pdf?utm_source=chatgpt.com)

[在线阅读 HTML 版本](https://ying-zhang.cn/dist/2014-raft.html?utm_source=chatgpt.com)&#x20;

[image](https://www.google.com/s2/favicons?domain=https://raft.github.io\&sz=32)

Raft

+1



3\. 博士论文：Consensus: Bridging Theory and Practice

深入研究选举、日志复制、正确性证明、成员变更、日志压缩与 TLA+ 形式化规范。

[论文源码与 PDF 下载入口](https://github.com/ongardie/dissertation?utm_source=chatgpt.com)

[在线阅读版 PDF](https://github.com/ongardie/dissertation/blob/master/online.pdf?utm_source=chatgpt.com)&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub



[raft/raft.go at main · etcd-io/raft · GitHub](https://images.openai.com/static-rsc-4/o75VQY_CmI7isbwRTCnr_-1RsGUgQlgVjQXMjm1ty07_gvQih9waVzCK2d0ZEPdTNZmfsX-cSUFFt_4I_0mccuZGeivINL4OoVUISxgZOSgFiaedHEd-LGMmJ3fqzOfDXotZbKPfRHbIUSlSgJTir3kXAFxeDWtD6k8FPRAjX1A?purpose=inline)

4\. etcd-io/raft：Go 语言 Raft 实现

理解 Raft 在生产代码中的实现。重点研究 `raft.go`、`log.go`、`node.go`、`storage.go` 和测试。

[GitHub 源码仓库](https://github.com/etcd-io/raft?utm_source=chatgpt.com)

[Go 包文档](https://pkg.go.dev/go.etcd.io/raft/v3?utm_source=chatgpt.com)&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub

+1



[Top Leader Election Algorithms in Distributed Databases](https://images.openai.com/static-rsc-4/sCPUnbCIm7FoREor9kkaZ3c2OZ6i4Mppu6dMfK8uThSnfOLVLCTupugHVE6hisCLvhyzu8_Na4OEfgbb1-IklFZLUk8j4JvRKZhCNCOqRGgjkYKu1qK-pIjHz1IDxta2C3zbqxT-DrsMbOTOaTEqJm0gri_ORXy-EZH8DWe9ZGE?purpose=inline)

5\. The Secret Lives of Data：Raft 可视化

用动画直观理解 Leader 选举、日志复制、任期变化和节点故障，适合阅读论文前后辅助学习。

[打开交互式讲解](https://thesecretlivesofdata.com/raft/?utm_source=chatgpt.com)&#x20;

[image](https://www.google.com/s2/favicons?domain=https://raft.github.io\&sz=32)

Raft



### 论文应该重点读哪些章节？

如果你想理解理论，而不是只知道大致流程，我建议按这个顺序阅读：

| 论文章节                                | 学习目标                        |
| ----------------------------------- | --------------------------- |
| §2 Replicated state machines        | 理解复制状态机与日志的关系               |
| §5.1 Server states                  | 理解角色和任期                     |
| §5.2 Leader election                | 掌握投票规则和选举超时                 |
| §5.3 Log replication                | 理解日志匹配、冲突修复与提交              |
| §5.4 Safety                         | 理解 Leader Completeness 及其证明 |
| §5.5 Follower and candidate crashes | 理解故障恢复                      |
| §7 Cluster membership changes       | 理解安全地变更成员                   |
| §8 Client interaction               | 理解客户端请求与响应语义                |

如果只选一份资料，先读 [Raft 扩展论文](https://raft.github.io/raft.pdf?utm_source=chatgpt.com)。如果想更深入理解证明和形式化规范，再读作者的博士论文。

## 十一、结合 Go 源码学习 Raft

既然你关注 Go 项目，我建议理论读到日志复制和安全性之后，就开始看 `etcd-io/raft`，不要等读完整篇论文才开始实践。

### 1. 建议的源码阅读顺序

第一步：`doc.go`

先理解该库的设计原则，以及算法核心与网络、磁盘 IO 的边界。

[查看源码说明](https://github.com/etcd-io/raft/blob/main/doc.go?utm_source=chatgpt.com)

第二步：`raft.go`

重点看角色切换、任期、RequestVote、AppendEntries 对应的消息处理、提交索引和复制进度。

[查看 raft.go](https://github.com/etcd-io/raft/blob/main/raft.go?utm_source=chatgpt.com)

第三步：`log.go`

研究日志索引、任期比较、提交边界、日志冲突和压缩相关逻辑。

[查看 log.go](https://github.com/etcd-io/raft/blob/main/log.go?utm_source=chatgpt.com)

第四步：`node.go` 和 `storage.go`

理解节点驱动接口、消息传递、Ready/Advance 流程以及存储接口。

[查看 node.go](https://github.com/etcd-io/raft/blob/main/node.go?utm_source=chatgpt.com) · [storage.go](https://github.com/etcd-io/raft/blob/main/storage.go?utm_source=chatgpt.com)

第五步：测试代码

通过测试理解论文中的规则如何转化为具体的边界条件，尤其是选举分裂、日志冲突和快照恢复。

[查看 Raft 测试](https://github.com/etcd-io/raft/tree/main?utm_source=chatgpt.com)

### 2. 特别注意 etcd Raft 的设计

`etcd-io/raft` 并不是一个开箱即用的完整分布式数据库。它实现 Raft 核心状态机，而网络传输和持久化由集成方负责。

[image](https://www.google.com/s2/favicons?domain=https://go.etcd.io\&sz=32)

Go Packages

+1



实际接入时，需要把几个部分配合好：

- Raft 核心：选举、复制、提交等协议逻辑。
- Transport：节点之间的 RPC 或消息传输。
- Storage：日志、任期、投票和快照的可靠存储。
- State Machine：应用已提交的日志，维护业务状态。
- Client API：处理请求转发、超时、幂等和响应。

特别要注意 `Ready` 处理和持久化顺序：不能在相关日志尚未可靠保存时，就把依赖它们的消息当作已安全发送；异步存储模式下更需要严格遵守库规定的完成顺序。

## 十二、推荐的学习路线

如果你的目标是从理论深入到能够自己实现或排查 Raft 问题，可以按以下顺序推进：

1. 理解复制状态机：搞清楚为什么日志顺序一致，就能让确定性的状态机得到相同结果。
2. 掌握选举：弄懂 Term、投票限制、随机超时，以及为什么日志新旧会影响选举。
3. 掌握日志复制：弄懂 `PrevLogIndex`、`PrevLogTerm`、`nextIndex`、`matchIndex` 和 `commitIndex` 的区别。
4. 研究安全性证明：重点理解多数派交集为什么还不够，以及 Leader Completeness 如何依赖其他规则。
5. 自己实现一个 3 节点 KV 服务：先支持选举、日志复制、提交和状态机应用，再添加持久化、快照与成员变更。
6. 阅读 etcd Raft：对照论文研究真实代码如何处理消息、磁盘 IO、重试、日志冲突和恢复。

最后，建议你牢牢记住三个区别：

1. Leader 不等于数据已经提交：Leader 可以存在未提交日志。
2. 多数派复制不等于所有任期的日志都能直接提交：当前任期规则很重要。
3. 共识安全性不等于请求恰好执行一次：客户端超时、重试和业务幂等需要另外设计。

理解这三点，再结合原始论文的安全性章节和 etcd Raft 的实现，你就能从“知道 Raft 的工作流程”进一步走到“能够解释为什么这些规则不会破坏一致性”。
