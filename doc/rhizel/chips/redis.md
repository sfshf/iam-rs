# Redis

## https://redis.io/docs/latest/develop/programmability/eval-intro/ 给我讲解一下该篇redis文章

ChatGPT 答：

这篇 Redis 官方文档讲的是 Redis Lua Script，也就是 EVAL / EVALSHA 脚本机制。它其实是理解 Redis 分布式锁、限流、库存扣减、队列操作等高级用法的基础。

官方原文：[Redis — Scripting with Lua](https://redis.io/docs/latest/develop/programmability/eval-intro/?utm_source=chatgpt.com)

我建议你不要把它当成“Lua 教程”看，而要抓住一个核心：

**把原本需要客户端执行的多条 Redis 命令，搬到 Redis 服务端一次性执行，并且整个脚本具有原子性。**

### 1. 为什么 Redis 需要 Lua？

先看一个很典型的问题。

假设我们有一个库存：

```txt
stock = 10
```

现在要实现：

```txt
如果库存 > 0
    库存 - 1
    返回成功
否则
    返回失败
```

你可能首先写成：

```txt
GET stock

if stock > 0 {
    DECR stock
}
```

看起来没问题。

但是有两个客户端：

```txt
Client A             Client B

GET stock -> 1
                     GET stock -> 1

DECR stock -> 0
                     DECR stock -> -1
```

最终库存变成：

```txt
-1
```

这就是典型的 **check-then-act race condition**。

### 2. Lua Script 解决的核心问题

我们可以把整个逻辑放到 Redis：

```lua
local stock = redis.call('GET', KEYS[1])

if tonumber(stock) > 0 then
    redis.call('DECR', KEYS[1])
    return 1
end

return 0
```

客户端：

```txt
EVAL script 1 stock
```

Redis 会：

```txt
进入 Lua Script
       ↓
GET stock
       ↓
判断
       ↓
DECR stock
       ↓
返回
       ↓
Script 完成
```

整个过程中 Redis 不会让其他客户端的命令插进来。

官方文档明确说明，Lua script 执行期间 Redis 会阻塞其他服务器活动，因此整个 script 是原子的。

这就是 Redis Lua 最重要的价值：

**把多个 Redis 操作组合成一个原子操作。**

### 3. EVAL 到底是什么？

最简单：

```sh
EVAL "return 'Hello, scripting!'" 0
```

返回：

```txt
"Hello, scripting!"
```

EVAL 的基本格式是：

```sh
EVAL script numkeys [key ...] [arg ...]
```

例如：

```sh
EVAL "return redis.call('GET', KEYS[1])" 1 username
```

这里：

```txt
EVAL
 ↓
Lua script
 ↓
1
 ↓
username
```

其中：

```txt
1
```

表示：

```txt
后面有 1 个参数属于 Redis Key。
```

所以：

```txt
KEYS[1] = username
```

### 4. KEYS 和 ARGV 是这篇文章最重要的概念之一

比如：

```txt
EVAL "
return {
    KEYS[1],
    KEYS[2],
    ARGV[1],
    ARGV[2]
}
" 2 user:1 user:2 hello world
```

那么：

```lua
KEYS[1] = "user:1"
KEYS[2] = "user:2"

ARGV[1] = "hello"
ARGV[2] = "world"
```

可以记成：

```txt
EVAL
 │
 ├── script
 │
 ├── numkeys
 │
 ├── KEYS
 │    ├── KEYS[1]
 │    ├── KEYS[2]
 │    └── ...
 │
 └── ARGV
      ├── ARGV[1]
      ├── ARGV[2]
      └── ...
```

官方特别强调：

**脚本访问的 Redis key，应该全部通过 KEYS 显式传入。**

这是 Redis Cluster 下非常重要的规则。

### 5. 为什么一定要区分 KEYS 和 ARGV？

这不是 Redis 故意搞复杂。

因为 Redis Cluster。

假设：

```txt
user:100
user:200
```

可能属于不同的 Cluster slot。

Redis 必须在执行 Lua Script 之前知道：

```txt
这个 script 会操作哪些 key？
```

因此：

```sh
EVAL script 2 user:100 user:200 ...
```

Redis 可以提前知道：

```txt
KEYS[1] -> user:100
KEYS[2] -> user:200
```

从而判断这些 key 是否可以在当前 Cluster 节点执行。

所以：

**KEYS**

代表：

```txt
Redis 数据库中的 key。
```

**ARGV**

代表：

```txt
普通参数 / value。
```

例如：

```lua
redis.call("SET", KEYS[1], ARGV[1])
```

对应：

```txt
EVAL script 1 username Gavin
```

就是：

```txt
KEYS[1] = username
ARGV[1] = Gavin
```

官方也明确要求脚本访问的 key 必须作为 key 参数提供，而不要在 Lua 内动态构造 key。

### 6. redis.call() 是什么？

这是 Lua 和 Redis 沟通的桥梁。

例如：

```lua
redis.call('SET', 'name', 'Gavin')
```

相当于：

```sh
SET name Gavin
```

再比如：

```lua
local value = redis.call('GET', KEYS[1])
```

相当于：

```sh
GET xxx
```

所以可以理解成：

```txt
Lua
 │
 │ redis.call()
 ↓
Redis Command
 │
 ↓
Redis Database
```

### 7. redis.call 和 redis.pcall

两者很像：

```lua
redis.call(...)
```

和：

```lua
redis.pcall(...)
```

区别主要是 **错误处理**。

**redis.call**

如果 Redis command 出错：

```lua
redis.call("INCR", "hello")
```

假设：

```txt
hello = "abc"
```

那么 `INCR` 会报错。

`redis.call()` 会直接让错误返回给客户端。

**redis.pcall**

```lua
local result = redis.pcall("INCR", "hello")
```

错误会作为结果返回给 Lua：

```lua
local result = redis.pcall(...)

if result.err then
    -- 自己处理错误
end
```

所以：

```txt
redis.call
    ↓
Redis error
    ↓
直接抛给客户端


redis.pcall
    ↓
Redis error
    ↓
返回给 Lua
    ↓
Lua 自己决定怎么办
```

官方文档也是这样区分两者的。

### 8. 为什么不能动态生成 Lua Script？

这是这篇文章里面非常值得注意的一点。

错误方式：

```sh
EVAL "return 'Hello Gavin'" 0
EVAL "return 'Hello Alice'" 0
EVAL "return 'Hello Bob'" 0
```

虽然逻辑没问题，但 Redis 会把不同的 script 放入 script cache。

也就是说：

```txt
script 1 -> SHA1 A
script 2 -> SHA1 B
script 3 -> SHA1 C
...
```

如果你不断生成不同的 script：

```txt
script_1
script_2
script_3
...
script_1000000
```

就可能导致 script cache 不断增长。

官方明确把这种动态生成 script 的方式称为 **anti-pattern**。

### 9. 正确方式：参数化 Script

不要：

```lua
return "Hello Gavin"
```

而是：

```lua
return ARGV[1]
```

然后：

```sh
EVAL "return ARGV[1]" 0 Gavin
```

再：

```sh
EVAL "return ARGV[1]" 0 Alice
```

再：

```sh
EVAL "return ARGV[1]" 0 Bob
```

实际上只有一个 script：

```lua
return ARGV[1]
```

变化的只是：

```txt
ARGV[1]
```

因此：

**Script 固定，数据通过 KEYS / ARGV 传入。**

这也是实际项目中写 Redis Lua 时非常重要的原则。

### 10. EVALSHA 是干什么的？

假设你有：

```lua
local stock = redis.call('GET', KEYS[1])

if tonumber(stock) > 0 then
    redis.call('DECR', KEYS[1])
    return 1
end

return 0
```

如果每次：

```sh
EVAL "一大坨Lua代码..." 1 stock
```

那么每次都需要把 Lua 源代码通过网络发送给 Redis。

所以 Redis 提供：

```sh
SCRIPT LOAD
```

例如：

```sh
SCRIPT LOAD "return ARGV[1]"
```

Redis 返回：

```txt
c664a3bf70bd1d45c4284ffebb65a6f2299bfc9f
```

这是这个 script 的 SHA1。

之后：

```sh
EVALSHA c664a3bf70bd1d45c4284ffebb65a6f2299bfc9f 0 Gavin
```

就可以执行。

整个流程：

```txt
第一次

Client
  │
  │ SCRIPT LOAD script
  ↓
Redis
  │
  └── SHA1


以后

Client
  │
  │ EVALSHA SHA1
  ↓
Redis
  │
  ↓
执行 Script
```

这样可以减少网络传输。

官方文档就是通过 `SCRIPT LOAD + EVALSHA` 介绍这种模式的。

### 11. 但是 EVALSHA 有一个坑

Script Cache 不是持久化数据。

Redis 重启：

```txt
Redis restart
     ↓
script cache 清空
```

主从故障转移：

```txt
Master
   ↓
Replica promoted
   ↓
新的 Redis
   ↓
script cache 可能没有
```

所以：

```sh
EVALSHA xxx 0
```

可能得到：

```txt
NOSCRIPT No matching script
```

官方明确说明 script cache 是 volatile，不属于 Redis 数据库持久化内容。

因此客户端通常应该：

```txt
EVALSHA
   ↓
NOSCRIPT?
   ↓ yes
SCRIPT LOAD
   ↓
EVALSHA
```

很多 Redis client 会自动帮你处理这个流程。

### 12. 这里有一个非常容易误解的地方

很多人会以为：

```sh
SCRIPT LOAD
```

就是：

```txt
把 Lua 永久保存到 Redis。
```

实际上不是。

它只是：

```txt
Redis 内存中的 script cache
```

不是：

```txt
RDB
AOF
```

所以：

```txt
Redis restart
     ↓
script cache 消失
```

但是：

```txt
Redis key/value data
```

可以通过 RDB/AOF 持久化。

这两个完全是两回事。

### 13. Lua Script 为什么比 MULTI/EXEC 有优势？

这是理解 Redis Lua 非常重要的一点。

假设：

```txt
GET stock
DECR stock
GET user_balance
INCR ...
```

你可以考虑：

```txt
MULTI
GET stock
DECR stock
...
EXEC
```

但是事务本身不提供：

```txt
if stock > 0 then
    DECR
else
    ...
end
```

这种复杂的程序逻辑。

Lua 可以：

```lua
local stock = redis.call("GET", KEYS[1])

if tonumber(stock) > 0 then
    redis.call("DECR", KEYS[1])
    return 1
end

return 0
```

也就是说：

```txt
MULTI/EXEC

Redis Commands
     +
简单事务语义


Lua Script

Redis Commands
     +
if
for
local variable
计算
条件判断
业务逻辑
```

因此 Lua 更像：

**在 Redis 内部写一个小型原子业务函数。**

### 14. 一个实际非常经典的例子：限流

比如：

```txt
一个用户一分钟最多请求 10 次
```

可以用：

```lua
local count = redis.call('INCR', KEYS[1])

if count == 1 then
    redis.call('EXPIRE', KEYS[1], 60)
end

if count > 10 then
    return 0
end

return 1
```

调用：

```sh
EVAL script 1 rate:user:123
```

逻辑：

```txt
INCR
 ↓
第一次？
 ↓
设置 EXPIRE 60
 ↓
count > 10?
 ↓
拒绝 / 允许
```

为什么需要 Lua？

因为：

```txt
INCR
EXPIRE
判断
```

必须作为一个整体执行。

否则多个客户端同时进来，可能出现竞态。

### 15. 再比如你之前遇到的 Redis Lua Lock

你之前在 Rust 里面做 Redis Lua script / lock，其实就是这个思想。

经典的释放锁：

```lua
if redis.call("GET", KEYS[1]) == ARGV[1] then
    return redis.call("DEL", KEYS[1])
end

return 0
```

假设：

```txt
KEYS[1] = lock:user:123

ARGV[1] = 当前客户端的 token
```

为什么不能写成：

```txt
GET lock:user:123

if value == token:
    DEL lock:user:123
```

因为：

```txt
Client A

GET
 ↓
发现 token 是自己的

        Client B
        DEL lock
        SET lock = B

Client A
 ↓
DEL
```

于是 A 把 B 的锁删除了。

Lua：

```lua
if GET == token then
    DEL
end
```

则：

```txt
Redis
┌──────────────────────────────┐
│ GET                          │
│ ↓                            │
│ 判断 token                   │
│ ↓                            │
│ DEL                          │
└──────────────────────────────┘
```

中间不会插入其他 Redis client 的命令。

这就是 Redis 分布式锁释放脚本的核心原理。

### 16. Redis Lua 的一个巨大特点：单线程阻塞

这个一定要理解。

Redis Lua：

```lua
for i = 1, 1000000000 do
end
```

不是：

```txt
Lua Script
     ↓
后台线程
```

而是：

```txt
Redis event loop
       ↓
执行 Lua
       ↓
整个 Redis 被阻塞
```

官方明确说：

**script 执行期间，Redis 的服务器活动都会被阻塞。**

所以 Lua Script 必须短小、快速。

不要写：

```lua
for i = 1, 1000000000 do
    ...
end
```

更不要把复杂业务逻辑塞进去。

正确思路：

```txt
Redis Lua
    ↓
几次 Redis command
    ↓
简单判断
    ↓
快速返回
```

而不是：

```txt
Redis Lua
    ↓
复杂算法
    ↓
大量循环
    ↓
几十毫秒 / 几秒
```

因为你的 Redis：

```txt
所有其他请求
      ↓
都在等这个 Script
```

### 17. SCRIPT KILL 为什么又有特殊限制？

假设：

```txt
Script 执行了 10 秒
```

Redis 被卡住。

可以：

```txt
SCRIPT KILL
```

但是有一个重要限制：

**如果 Script 已经修改数据**

Redis 不能简单杀掉它。

因为这样可能导致：

```txt
操作 1 成功
操作 2 成功
操作 3 没执行
```

原子性被破坏。

所以官方规定，`SCRIPT KILL` 可以终止一个尚未修改数据的长时间运行 script；已经修改数据的 script 不能通过这种方式简单中断。

### 18. Redis 7 之后还有一个重要变化

你现在看 Redis 文档，必须注意：

```txt
Redis 7 已经有 Redis Functions。
```

官方现在把 Functions 定义为 Redis 7+ 的另一种 server-side programmability 方式，并且称其为对过去 EVAL 使用方式的替代方案。

简单理解：

**EVAL**

```txt
应用程序
   ↓
EVAL Lua
   ↓
Redis
```

Script 更像：

```txt
客户端应用程序的一部分。
```

它的 source code 由客户端保存，需要的时候加载。

**Redis Functions**

```txt
管理员 / 部署
      ↓
加载 Function
      ↓
Redis
      ↓
所有 Client 都可以调用
```

Function 更像：

```txt
Redis 服务端的一部分。
```

所以：

```txt
EVAL Script

application-owned
ephemeral
需要客户端管理
```

而：

```txt
Redis Function

server-owned
持久化管理
独立部署
```

这是 Redis 7+ 很重要的变化。

### 19. 这篇文章其实可以浓缩成一张图

你可以把整篇文章记成：

```txt
                    Redis Programmability
                            │
              ┌─────────────┴─────────────┐
              │                           │
          Lua Script                 Redis Functions
              │                         Redis 7+
              │
          EVAL / EVALSHA
              │
       ┌──────┴──────┐
       │             │
     KEYS           ARGV
       │             │
 Redis Keys       普通参数
       │             │
       └──────┬──────┘
              │
        redis.call()
              │
              ↓
       Redis Commands
              │
              ↓
       Atomic Execution
              │
              ↓
       一次性完成业务逻辑
```

### 20. 你真正应该掌握的 8 个知识点

如果你是在学习 Redis，而不是单纯看文档，我建议把重点放在这几个地方：

| 知识                   | 重要程度   |
| ---------------------- | ---------- |
| EVAL                   | ⭐⭐⭐⭐⭐ |
| KEYS / ARGV            | ⭐⭐⭐⭐⭐ |
| redis.call()           | ⭐⭐⭐⭐⭐ |
| Lua 原子性             | ⭐⭐⭐⭐⭐ |
| EVALSHA                | ⭐⭐⭐⭐   |
| Script Cache           | ⭐⭐⭐⭐   |
| SCRIPT LOAD / NOSCRIPT | ⭐⭐⭐     |
| Redis Functions        | ⭐⭐⭐⭐   |

其中最核心的是：

```txt
EVAL
 ↓
KEYS / ARGV
 ↓
redis.call()
 ↓
原子执行
```

### 最后给你一个非常实用的思维模型

以后你看到这样的代码：

```txt
GET
判断
SET
EXPIRE
INCR
ZADD
...
```

如果这些操作之间存在：

**必须保证中间不能被其他 Redis 客户端插入操作**

那么你就应该想到：

```txt
Lua Script
```

例如：

```txt
库存扣减
分布式锁
限流
排行榜更新
条件删除
条件更新
多个 key 的原子操作
```

都非常适合这个模式。

而如果只是：

```txt
SET
GET
INCR
HSET
```

这种单条 Redis 命令本身已经是原子的，就不要为了“高级”而上 Lua。

## https://redis.io/docs/latest/develop/clients/patterns/distributed-locks/ 给我讲解一下这篇redis文章

这篇是 Redis 官方讲 **分布式锁（Distributed Lock）以及 Redlock 算法** 的文章。它和你上一条看的 Lua Script 文档其实是直接连起来的：**Lua/`DELEX` 负责“安全释放锁”，而这篇文章负责解释“怎样设计一个分布式锁”**。

我建议你重点理解下面这条演进路线：

```txt
普通 SET
   ↓
单 Redis 实例锁
   ↓
SET NX PX
   ↓
随机 token
   ↓
安全释放
   ↓
Redis 主从为什么不够
   ↓
Redlock
   ↓
5 个独立 Redis
   ↓
多数派
   ↓
TTL + 时间窗口
   ↓
fencing token / 一致性问题
```

### 1. 首先：什么叫分布式锁？

假设你的系统有两个 Go 服务：

```txt
Service A
Service B
```

它们同时操作：

```txt
订单 123
```

你希望：

```txt
Service A ────────┐
                  │
                  ↓
              [订单123]
                  ↑
                  │
Service B ────────┘
```

任何时刻只能有一个服务执行：

```txt
修改订单
生成账单
扣库存
执行定时任务
处理同一用户任务
```

于是需要一个共享的：

```txt
lock:order:123
```

这就是分布式锁。

Redis 官方把它需要满足的东西拆成三个性质：

#### Safety

**Mutual Exclusion**

同一时间只能有一个客户端持有锁。

#### Liveness A

**Deadlock Free**

持锁客户端挂掉之后，最终其他客户端仍然能够拿到锁。

#### Liveness B

**Fault Tolerance**

只要 Redis 节点的大多数还活着，就仍然可以获取/释放锁。

这三个定义是理解全文的钥匙。

### 2. 最简单的 Redis 锁其实非常简单

最基础的写法：

```sh
SET resource_name my_random_value NX PX 30000
```

拆开：

```txt
SET
 ↓
resource_name
 ↓
my_random_value
 ↓
NX
 ↓
PX 30000
```

意思：

```txt
如果 resource_name 不存在，就设置它，并且 30 秒后自动过期。
```

**NX**

```txt
NX = Not Exists
```

相当于：

```txt
if key does not exist:
    SET key
```

所以：

```sh
SET lock:order:123 abc NX
```

只有一个客户端能够成功。

假设：

```txt
Client A
SET lock:order:123 AAA NX
→ OK

Client B
SET lock:order:123 BBB NX
→ nil
```

于是：

```txt
A = 获得锁
B = 获取失败
```

### 3. 为什么一定要 PX？

如果没有：

```txt
PX 30000
```

那么：

```txt
Client A
    ↓
获得锁
    ↓
程序崩溃
    ↓
lock 永远存在
```

以后：

```txt
B → 永远拿不到
C → 永远拿不到
D → 永远拿不到
```

这就是：

```txt
Deadlock
```

所以要：

```sh
SET lock:xxx token NX PX 30000
```

即：

```txt
获得锁
   ↓
30 秒自动释放
```

Redis 官方把这个 TTL 同时看作：

1. 客户端崩溃后的自动释放时间。
2. 持锁客户端应该完成工作的时间窗口。

### 4. 为什么 value 不能随便写？

这里是整篇文章非常重要的一点。

很多初学者会写：

```sh
SET lock:123 "locked" NX PX 30000
```

然后释放：

```sh
DEL lock:123
```

这是有问题的。

假设：

```txt
时间 →

A:
获得锁
│
│
│ 30秒过去
│
X 锁自动过期
│
│
B:
获得锁
│
│
A:
DEL lock
```

结果：

```txt
A 把 B 的锁删掉了！
```

这就违反：

```txt
Mutual Exclusion
```

### 5. 所以 Redis 锁必须带“身份证”

例如：

```txt
Client A token = 8f3a...
Client B token = 91cd...
```

A：

```sh
SET lock:123 8f3a... NX PX 30000
```

B：

```sh
SET lock:123 91cd... NX PX 30000
```

此时：

```txt
lock:123
    ↓
8f3a...
```

只有 A 知道这个 token。

### 6. 释放锁不能简单 DEL

不能：

```sh
DEL lock:123
```

而必须：

```sh
if GET(lock:123) == my_token:
    DEL(lock:123)
```

这两个操作必须是原子的。

以前 Redis 官方文档给出的 Lua：

```lua
if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("del", KEYS[1])
else
    return 0
end
```

也就是：

```txt
GET
 ↓
比较 token
 ↓
相同？
 ├── YES → DEL
 └── NO  → 什么都不做
```

这正好对应你上一条看的 Lua Script。

而当前文档还特别更新了：

```txt
Redis 8.4 引入了 `DELEX`，可以用 `DELEX key IFEQ my_random_value` 做这个条件删除；旧版本可以用 Lua。
```

所以如果你使用 Redis 8.4+，可以关注：

```txt
DELEX lock:123 IFEQ <token>
```

### 7. 这一套其实已经可以做一个“单 Redis 锁”

完整模型：

```txt
获取：

SET lock:resource token NX PX 30000
                │
                └── 唯一 token


释放：

if GET(lock) == token:
    DEL(lock)
```

所以：

```txt
              Redis
                │
       ┌────────┴────────┐
       │                 │
    Client A          Client B
       │                 │
 SET lock NX         SET lock NX
       │                 │
      OK              FAIL
       │
    执行业务
       │
    check token
       │
      DEL
```

这已经是一个**相当实用的单实例 Redis 分布式锁**。

Redis 官方也明确指出，这种单实例方案在一些允许故障时偶发竞争的场景下是可接受的。 

### 8. 那为什么还要 Redlock？

问题来了。

假设：

```txt
Redis Master
    +
Redis Replica
```

看起来很合理：

```txt
        Master
          │
       replicate
          ↓
       Replica
```

Master 挂掉：

```txt
Replica → promoted
```

但是 Redis replication 是异步的。

于是可能发生：

```txt
T1

Client A
   ↓
Master
   ↓
SET lock = AAA
   ↓
成功
```

但是：

```txt
AAA
```

还没有复制到 Replica。

突然：

```txt
Master 崩溃
```

Replica：

```txt
没有 lock
```

于是：

```txt
Client B
   ↓
Replica
   ↓
SET lock = BBB NX
   ↓
成功
```

现在：

```txt
A 持有锁
+
B 也持有锁
```

发生：

**Safety violation**

官方专门用这个例子说明，简单的 Redis 主从 failover 无法保证严格的互斥。 

### 9. 注意：这里容易产生一个误区

很多人会说：

```txt
“Redis 有 Sentinel/Cluster，所以分布式锁天然高可用。”
```

不对。

Redis Sentinel / replication 解决的是：

```txt
Redis 服务高可用
```

但它不自动等于：

```txt
严格的 distributed lock correctness
```

因为复制可能存在：

```txt
write
 ↓
master
 ↓
还没复制
 ↓
master crash
 ↓
replica promote
```

这就是文章为什么要进一步提出：

```txt
多个独立 Redis master + majority
```

也就是 Redlock。

### 10. Redlock 到底是什么？

假设我们有：

```txt
Redis 1
Redis 2
Redis 3
Redis 4
Redis 5
```

注意：

```txt
这不是一个 Redis Cluster。
```

而是：

```txt
5 个独立 Redis master
```

官方示例使用：

```txt
N = 5
```

并要求它们尽可能运行在故障独立的机器/VM 上。

### 11. Redlock 获取锁的过程

假设：

```txt
TTL = 10 秒
```

客户端：

```txt
Client A
```

首先生成：

```txt
token = ABC123
```

然后记录：

```txt
start = 当前时间
```

接着尝试：

```txt
             ┌── Redis 1 → SET NX PX 10s
             │
Client A ────┼── Redis 2 → SET NX PX 10s
             │
             ├── Redis 3 → SET NX PX 10s
             │
             ├── Redis 4 → SET NX PX 10s
             │
             └── Redis 5 → SET NX PX 10s
```

最好并行执行，而不是：

```txt
R1
 ↓
R2
 ↓
R3
 ↓
R4
 ↓
R5
```

因为串行执行会浪费大量 TTL。官方也建议使用 multiplexing 尽快向多个节点发送 `SET`。

### 12. 什么叫 majority？

5 个 Redis：

```txt
N = 5
```

多数：

```txt
N / 2 + 1
```

所以：

```txt
5 / 2 + 1 = 3
```

也就是至少：

```txt
3 / 5
```

成功。

例如：

```txt
Redis 1 → OK
Redis 2 → OK
Redis 3 → OK
Redis 4 → FAIL
Redis 5 → FAIL
```

那么：

```txt
3 / 5
```

达到 majority。

可以认为：

```txt
锁获取成功
```

### 13. 但“3 个成功”还不够

这点特别重要。

假设：

```txt
TTL = 10s
```

然后：

```txt
开始获取锁
      ↓
经过 9.9 秒
      ↓
终于 3 个 Redis 成功
```

虽然：

```txt
3/5
```

但是：

```txt
剩余 TTL ≈ 0.1 秒
```

这种锁实际上没有什么意义。

所以 Redlock 要同时满足：

```txt
成功节点 >= majority
```

并且：

```txt
获取锁所花时间 < TTL
```

也就是：

```txt
lock_validity = TTL - elapsed_time
```

如果：

```txt
elapsed = 30ms
TTL = 10s
```

那么：

```txt
validity ≈ 9970ms
```

如果：

```txt
elapsed = 10.2s
TTL = 10s
```

那么：

```txt
validity < 0
```

锁获取失败。

官方明确要求同时满足多数节点成功以及总耗时小于 lock validity。

### 14. 所以 Redlock 的核心公式

你可以直接记：

```txt
成功条件：

success_count >= floor(N/2) + 1
AND
elapsed < TTL
```

对于：

```txt
N = 5
TTL = 10s
```

就是：

```txt
success_count >= 3
AND
elapsed < 10s
```

最终有效时间：

```txt
validity = TTL - elapsed - clock_drift
```

文章后面在安全性分析中把这个思想进一步形式化成：

```txt
MIN_VALIDITY =
TTL - (T2 - T1) - CLOCK_DRIFT
```

这是 Redlock 理论推导中非常核心的公式。

### 15. 如果只成功了 2 个怎么办？

例如：

```txt
R1 → OK
R2 → OK
R3 → FAIL
R4 → FAIL
R5 → FAIL
```

那么：

```txt
2 / 5
```

不是 majority。

所以：

```txt
获取失败
```

但有一个非常重要的动作：

```txt
把自己可能已经获得的锁尽快释放掉。
```

也就是说：

```txt
R1 → DEL if token == ours
R2 → DEL if token == ours
R3 → DEL if token == ours
R4 → DEL if token == ours
R5 → DEL if token == ours
```

为什么？

因为 R1/R2 上的锁如果不释放，就只能：

```txt
等待 TTL 自动过期
```

会降低系统可用性。

官方特别强调失败后应尽快释放部分成功的锁。

### 16. 为什么释放所有节点？

这里很多人第一次实现会犯错：

```txt
“R3/R4/R5 返回失败，所以不用释放。”
```

其实更安全的做法是：

```txt
release R1
release R2
release R3
release R4
release R5
```

每一个节点都执行：

```txt
if token == my_token:
    delete
```

所以即使：

```txt
你认为自己没有拿到 R3
```

也可以尝试删除。

因为：

```txt
token 校验
```

会保护其他客户端的锁。

### 17. Redlock 为什么要随机 token？

还是那个原因：

```txt
Client A
token = AAA

Client B
token = BBB
```

假设：

```txt
A 的 TTL 到期
 ↓
B 获取锁
 ↓
A 恢复执行
```

A 不能：

```txt
DEL lock
```

因为可能删除 B。

必须：

```txt
DEL IF value == AAA
```

于是：

```txt
Redis value = BBB

AAA != BBB

→ 不删除
```

这就是：

```txt
lock ownership
```

所以一个 Redis lock 不应该只是：

```txt
key = lock
value = true
```

而应该：

```txt
key = lock
value = unique token
```

### 18. 这和你之前看的 Lua 正好串起来

你刚刚看的上一篇：

```txt
redis.call()
KEYS
ARGV
EVAL
```

现在全部用上了。

释放锁：

```lua
if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("del", KEYS[1])
end

return 0
```

这里：

```txt
KEYS[1]
    ↓
lock:order:123

ARGV[1]
    ↓
AAA-BBB-CCC
```

所以：

```txt
Lua
 │
 ├── GET lock
 │
 ├── compare token
 │
 └── DEL lock
```

是一个原子操作。

这就是为什么你上一篇文章和这一篇要一起看。

### 19. Redlock 的完整流程

把前面全部合起来：

```txt
                   Client
                     │
                     │ generate token
                     ↓
              token = UUID/random
                     │
                     ↓
          record start timestamp
                     │
          ┌──────────┼──────────┐
          ↓          ↓          ↓
        Redis1     Redis2     Redis3
          │          │          │
          ↓          ↓          ↓
        SET NX     SET NX     SET NX
        PX TTL     PX TTL     PX TTL
          │          │          │
          └──────────┼──────────┘
                     ↓
                count success
                     │
             ┌───────┴────────┐
             │                │
       majority +             no majority
       elapsed < TTL                │
             │                      ↓
             ↓                 release all
        LOCK ACQUIRED
             │
             ↓
          do work
             │
             ↓
        release all
```

### 20. 为什么每个 Redis 请求必须设置很短的 timeout？

这个也很重要。

假设：

```txt
TTL = 10 秒
```

但：

```txt
Redis 1
```

已经挂了。

如果客户端傻等：

```txt
connect Redis1
   ↓
等待 10 秒
```

然后再：

```txt
Redis2
Redis3
...
```

那么：

```txt
TTL = 10s
elapsed = 10s+
```

最后：

```txt
锁无效
```

所以文章建议单节点请求 timeout 要远小于整个 TTL，例如 TTL 10 秒时，单节点 timeout 可以是大约：

```txt
5~50 ms
```

具体当然要根据实际网络环境调整。

### 21. 为什么失败重试要随机等待？

假设：

```txt
Client A
Client B
Client C
```

同时竞争：

```txt
lock:order:123
```

如果全部：

```txt
失败
 ↓
立即 retry
```

那么：

```txt
A ─┐
B ─┼── 同时 retry
C ─┘
```

下一轮继续碰撞。

所以：

```txt
A → random 20ms
B → random 57ms
C → random 13ms
```

让竞争者错开。

这就是：

```txt
random backoff
```

官方也建议失败后使用随机延迟来减少同时竞争造成的 split-brain/竞争窗口。

### 22. 这篇文章最重要的一个限制：TTL

假设：

```txt
TTL = 10s
```

你拿到锁：

```txt
T=0
```

然后开始执行：

```txt
业务处理
```

结果：

```txt
T=9.9s
```

还没做完。

到了：

```txt
T=10s
```

锁自动过期。

此时：

```txt
Client B
   ↓
获得锁
```

于是：

```txt
A → 还在干活
B → 也拿到了锁
```

这意味着：

```txt
Redis 锁不是“我进程活着就永远持有”。
```

而是：

```txt
我在一个有限的 lease validity window 内拥有锁。
```

官方特别强调了这一点。

### 23. 所以文章后面讲“延长锁”

如果任务可能比较长：

```txt
TTL = 10s
任务 = 30s
```

可以：

```txt
获取锁
   ↓
10s TTL
   ↓
剩余很少
   ↓
extend
   ↓
继续工作
   ↓
再次 extend
```

延长时不能简单：

```sh
EXPIRE lock 10000
```

因为可能：

```txt
A 的锁已经过期

B 获得锁

A:
EXPIRE lock 10000
```

A 就把 B 的锁续命了。

所以仍然要：

```lua
if value == my_token:
    EXPIRE
```

也就是文章说的：

使用 Lua script 检查 key 仍然存在且 value 仍然是自己的随机值，然后延长 TTL。

### 24. 但是延长锁也不是无限续

否则：

```txt
Client A
一直续
一直续
一直续
一直续
```

那么：

```txt
Client A 永远占着锁
```

这会破坏：

```txt
Liveness
```

所以官方建议：

```txt
锁续期次数应该有限制。
```

也就是说：

```txt
Lock
 ↓
extend
 ↓
extend
 ↓
extend
 ↓
达到最大次数
 ↓
必须结束
```

### 25. 最后一个章节非常重要：Redlock 并不是“绝对安全”

这是我认为你读这篇文章时最应该注意的地方。

官方当前文档自己就提醒：

```txt
如果你非常关心 consistency/correctness，需要认真看后面的 Redlock analysis。 
```

尤其是：

**Fencing Tokens**

以及：

**Redis TTL 使用 wall clock，而不是 monotonic clock**

这两个问题很重要。

### 26. Fencing Token 是什么？

假设：

```txt
Client A
```

获得锁：

```txt
token = 100
```

然后 A 因为：

```txt
GC pause
网络延迟
CPU starvation
```

卡住了。

TTL 到期：

```txt
A 的锁过期
```

然后：

```txt
Client B
```

获得锁：

```txt
token = 101
```

B 开始操作资源。

这时 A 突然恢复：

```txt
A 继续执行旧任务
```

于是：

```txt
A ───────→ Resource
B ───────→ Resource
```

即使 Redis 锁本身已经正常过期/重新获取，**旧客户端仍可能继续向真正的资源发送操作**。

所以真正强一致的系统经常需要：

```txt
Lock
 +
Fencing Token
```

例如：

```txt
A → token 100
B → token 101
```

资源服务规定：

```txt
token < 当前最大 token
```

直接拒绝。

于是：

```txt
A(token=100) → reject
B(token=101) → accept
```

这样即使 A“死而复生”，也不能覆盖 B 的操作。

Redis 当前文档也明确建议，对于特别是执行时间可能较长的进程，应考虑 fencing tokens，不能仅仅假设“进程还活着，所以锁还在”。

### 27. 所以你应该把“Redis 分布式锁”分成三个层次理解

#### 第一层：普通单 Redis 锁

```txt
SET NX PX
+
unique token
+
Lua/DELEX conditional delete
```

适合大量普通业务。

#### 第二层：Redlock

```txt
Redis1
Redis2
Redis3
Redis4
Redis5

       ↓

majority
```

主要解决：

```txt
单 Redis 实例故障
```

带来的锁安全问题。

#### 第三层：真正严格的一致性场景

```txt
Distributed Lock
       +
Fencing Token
       +
Resource-side validation
```

例如：

```txt
支付
银行账户
订单状态机
分布式任务
重要文件写入
强一致数据库操作
```

这种场景不能简单认为：

```txt
Redis lock = absolute ownership
```

### 28. 结合你之前的 Rust Redis 代码来看

你之前写的 Rust Lua，大概就是：

```txt
Rust
 │
 │ Script::new(...)
 ↓
redis-rs
 │
 │ EVAL
 ↓
Redis
 │
 │ Lua
 │
 ├── KEYS[1]
 │
 ├── ARGV[1]
 │
 ├── GET
 │
 ├── compare
 │
 └── DEL
 ↓
RESP result
 ↓
Rust FromRedisValue
```

如果你正在实现一个 Redis Lock，我建议你的基础版本至少是：

```txt
acquire:

SET key token NX PX ttl
```

然后：

```txt
release:

DELEX key IFEQ token
```

如果 Redis < 8.4：

```lua
if redis.call("GET", KEYS[1]) == ARGV[1] then
    return redis.call("DEL", KEYS[1])
end

return 0
```

而不是：

```sh
SET key "locked"
DEL key
```

这是两个完全不同安全级别的实现。

### 29. 最后给你一张“考试级”总结图

```txt
                Redis Distributed Lock
                         │
          ┌──────────────┴──────────────┐
          │                             │
     Single Redis                  Redlock
          │                             │
          │                      N independent Redis
          │                             │
   SET key token                        │
   NX PX TTL                            │
          │                      majority = N/2+1
          │                             │
          │                      elapsed < TTL
          │                             │
          ↓                             ↓
     Acquire Lock                 Acquire Lock
          │                             │
          │                             │
     unique token                 same token on
          │                        all nodes
          ↓                             │
    Release Lock                        ↓
          │                        Release all
          │                        with token
          ↓
   GET == token ?
          │
      ┌───┴───┐
      │       │
     YES      NO
      │       │
     DEL    nothing
```

而**真正需要牢记的不是 Redlock 的代码，而是这 6 个原则**：

```txt
① SET NX：竞争者只有一个能成功

② PX/TTL：客户端挂掉后锁最终会释放

③ unique token：锁必须有 owner identity

④ conditional delete：
   只能删除自己的锁

⑤ Redlock majority：
   N 个独立节点至少 N/2+1

⑥ TTL 是有限租约：
   不能假设“进程还活着 = 锁还在”
```

尤其是最后一点，再加上 fencing token，是从“会写 Redis 锁”进入“真正理解分布式锁”的关键。

## redlock 在向多个redis实例取锁后，majority是由客户端自己计算嘛？

ChatGPT 答：

对，Redlock 的 majority 是由客户端自己计算的。Redis 服务端并不知道“这是一次 Redlock 操作”，也没有一个 Redis 节点负责统计多数派。

例如你配置了 5 个独立 Redis：

```txt
N = 5
majority = floor(5 / 2) + 1 = 3
```

客户端自己向：

```txt
Redis 1
Redis 2
Redis 3
Redis 4
Redis 5
```

发送：

```sh
SET lock:xxx <random-token> NX PX 10000
```

然后客户端收集结果：

```txt
Redis 1 → OK
Redis 2 → OK
Redis 3 → nil
Redis 4 → OK
Redis 5 → timeout
```

客户端自己统计：

```txt
成功 = 3
```

然后：

```txt
3 >= 3
```

所以认为获得了 Redlock。

同时还要计算整个获取过程耗时：

```txt
elapsed = T_end - T_start
```

然后判断：

```txt
success >= majority
        &&
elapsed < TTL
```

例如：

```txt
N = 5
majority = 3
TTL = 10s

成功 3 个
获取耗时 35ms

=> Lock acquired
```

### Redis 实例之间其实不知道彼此发生了什么

这一点很关键。

Redlock 的架构实际上是：

```txt
                 Client
                   │
       ┌───────────┼───────────┐
       ↓           ↓           ↓
    Redis A     Redis B     Redis C
       ↓           ↓           ↓
      OK          OK          OK

       ┌───────────┐
       ↓           ↓
    Redis D     Redis E
       ↓           ↓
      nil        timeout
```

Redis A 不知道：

```txt
Redis B 成功了
Redis C 成功了
```

Redis B 也不知道：

```txt
Redis A 成功了
```

它们各自只是执行普通的：

```sh
SET key token NX PX ttl
```

Redlock 的“算法”主要存在于客户端。

### 那为什么要求“独立 Redis”？

因为如果是：

```txt
Redis Master
     │
     ├── Replica 1
     ├── Replica 2
     └── Replica 3
```

这几个实际上存在共同故障域。

而 Redlock 理论上的模型更接近：

```txt
Redis 1     Redis 2     Redis 3     Redis 4     Redis 5
   │           │           │           │           │
独立故障域  独立故障域  独立故障域  独立故障域  独立故障域
```

这样即使：

```txt
Redis 1 挂了
Redis 2 挂了
```

剩下：

```txt
Redis 3
Redis 4
Redis 5
```

仍然有多数派。

### 还有一个容易忽略的地方

客户端判断“获取成功”后，并不是说其他 Redis 节点必须返回成功。

例如：

```txt
5 个 Redis

R1 OK
R2 OK
R3 OK
R4 FAIL
R5 FAIL
```

客户端认为：

```txt
Redlock acquired
```

但 R4、R5 上可能根本没有这个 lock。

所以之后释放锁时，客户端仍然会向所有节点发送释放操作：

```txt
R1 → release
R2 → release
R3 → release
R4 → release
R5 → release
```

每个节点自己检查：

```txt
if GET(lock) == my_token:
    DEL(lock)
```

所以可以把 Redlock 理解成：

**Redis 实例负责提供“单节点锁”，客户端负责把多个单节点锁组合成“多数派锁”。**

也就是：

```txt
        单节点 Redis
             │
      SET NX PX TTL
             │
             ↓
       获得/没获得
             │
             ↓
        ┌─────────┐
        │ Client  │
        │ 统计 OK │
        └────┬────┘
             │
       OK >= N/2+1 ?
          /       \
        YES        NO
         │          │
      acquired    failed
```

这也是为什么实现 Redlock 时，客户端本身实际上就是算法的核心组件。

## 那这样子的话，如果redis采用的是副本模式，就没必要使用redlock，而只要使用 普通单redis锁就行了吧？

ChatGPT 答：

不完全是。关键要看你对锁的正确性要求。

如果 Redis 是：

```txt
Master
   │
   ├── Replica 1
   └── Replica 2
```

你完全可以使用普通的：

```sh
SET lock:xxx <token> NX PX 30000
```

但是这和 Redlock 解决的是不同的问题。

### 1. 普通单 Redis 锁

如果 Master 正常：

```txt
Client A
   ↓
Master
   ↓
SET lock token NX PX
   ↓
OK
```

然后 A 工作。

这时候 Replica 只是复制数据：

```txt
Master
   │
   ├── Replica 1
   └── Replica 2
```

**锁的获取仍然只发生在 Master 上。**

所以对于很多业务：

```txt
任务去重
定时任务
缓存更新
一般性的并发控制
```

普通单 Redis 锁就够用了。

### 2. 真正的问题发生在 Master 故障转移

例如：

```txt
T1:
Client A
    ↓
Master
    ↓
SET lock = AAA
    ↓
OK
```

但是此时：

```txt
Master
   │
   └── Replica
        ↓
      还没复制到 AAA
```

然后：

```txt
T2:
Master 崩溃
    ↓
Replica promoted
```

新 Master 里面：

```txt
lock:xxx = 不存在
```

于是：

```txt
T3:

Client B
   ↓
New Master
   ↓
SET lock = BBB NX PX
   ↓
OK
```

现在：

```txt
A 认为自己有锁
B 也认为自己有锁
```

这就是 Redis 官方文章为什么说：

```txt
Redis replication/failover 本身不能保证分布式锁的互斥性。
```

### 所以你的理解应该稍微修改一下

不是：

```txt
“有副本模式，所以没必要 Redlock。”
```

而是：

```txt
“如果我可以接受 Redis 故障转移时极小概率出现锁安全性问题，那么普通单 Redis 锁通常就足够；如果我要求在 Redis 节点故障时仍尽可能维持锁的互斥语义，才考虑 Redlock。”
```

可以这样看：

|方案| 正常运行| Master 故障| 实现复杂度|
|单 Redis| ✅| ⚠️ 可能丢锁| 低|
|Redis Master + Replica| ✅| ⚠️ failover 期间可能丢锁| 低|
|Redlock 5 独立 Redis| ✅| 设计目标是多数节点故障下仍保持锁语义| 高|

### 还有一个非常重要的区别

Redis Replica 的目的主要是：

```txt
高可用
数据复制
故障转移
```

而 Redlock 的多个 Redis：

```txt
Redis 1
Redis 2
Redis 3
Redis 4
Redis 5
```

不是为了做：

```txt
数据副本
```

而是为了形成：

```txt
独立故障域 + majority
```

这是两个完全不同的设计思想。

所以你可以把它记成：

```txt
Master + Replica
       ↓
解决 Redis 服务本身的 HA


Redlock
       ↓
解决分布式锁在节点故障情况下的安全性
```

不过还有一个更深的问题：即使使用 Redlock，也不能简单理解成“绝对不会出现两个客户端同时执行临界区”。如果你的资源是数据库、文件、外部 API 等，真正要求强一致时，还要考虑前面说的 fencing token。

如果你愿意，我可以接着给你讲一个非常关键的问题：为什么“Redis 主从 + 普通锁”在实际互联网业务中反而经常比 Redlock 更常见，以及
Redlock 到底适合什么场景。

## 什么场景需要使用多个独立故障域的redis？多个独立故障域的redis的场景，每个使用到redis的服务器，在使用redis进行缓存时，是要明确指定其某个业务使用了哪个redis嘛？

ChatGPT 答：

对，这里需要把 “Redis 集群/副本” 和 “多个独立 Redis 实例组成 Redlock” 彻底区分开。你问的第二个问题也非常关键：**业务服务器通常不需要因为 Redlock 而把普通缓存数据人为分配到某个 Redis。**

### 1. 什么场景需要多个独立故障域 Redis？

核心场景是：

```txt
这个锁本身非常重要，即使某一个 Redis 故障，也不希望因为故障转移而出现两个客户端同时认为自己持有锁。
```

例如：

```txt
                    业务资源
                       │
             ┌─────────┴─────────┐
             │                   │
          Client A            Client B
             │                   │
             └─────────┬─────────┘
                       │
                  分布式锁
                       │
       ┌───────────────┼───────────────┐
       ↓               ↓               ↓
    Redis A         Redis B         Redis C
   故障域 1        故障域 2        故障域 3
```

典型例子可能是：

- 同一批任务绝对不能被两个 worker 同时执行
- 多个服务操作同一个外部资源，而这个操作没有幂等机制
- 某些关键资源只有一个消费者能够处理
- Redis 本身发生节点故障时，仍然希望锁的安全性尽量保持

但这里有个非常重要的现实问题：

```txt
不是“用了 Redis 就应该 Redlock”。
```

如果你的业务只是：

```txt
缓存
Session
排行榜
计数器
限流
普通任务去重
```

通常没有必要为了这些东西部署 5 个独立 Redis。

### 2. “独立故障域”到底是什么意思？

假设你部署：

```txt
Redis 1
Redis 2
Redis 3
Redis 4
Redis 5
```

如果它们全部：

```txt
同一台物理机
同一个 Docker Host
同一个 Kubernetes Node
同一个机房
同一个交换机
```

那么严格来说，它们并没有真正形成很好的独立故障域。

例如：

```txt
              物理服务器
                  │
       ┌──────────┼──────────┐
       ↓          ↓          ↓
    Redis1     Redis2      Redis3
```

服务器挂掉：

```txt
Redis1 ❌
Redis2 ❌
Redis3 ❌
```

全部没了。

Redlock 的设计思想更接近：

```txt
机房/节点 A
    Redis 1

机房/节点 B
    Redis 2

机房/节点 C
    Redis 3

机房/节点 D
    Redis 4

机房/节点 E
    Redis 5
```

当然，实际部署不一定真的跨五个机房，但核心思想就是：

```txt
这些 Redis 的故障应该尽可能相互独立。
```

### 3. 那业务服务器是不是要指定“这个业务用 Redis 1”？

这里要分两种情况。

**普通 Redis 缓存**

通常不是：

```txt
订单服务 → Redis 1
用户服务 → Redis 2
商品服务 → Redis 3
```

而是：

```txt
                 Redis 集群
                    │
        ┌───────────┼───────────┐
        ↓           ↓           ↓
      用户服务     订单服务     商品服务
```

由 Redis Cluster、Sentinel、代理层或者客户端负责：

```txt
key → Redis node
```

业务代码一般只需要：

```go
redis.Get(ctx, "user:123")
```

不需要自己决定：

```txt
user:123 → Redis 3
```

### 4. Redlock 则完全不同

如果你实现 Redlock：

```txt
Redis 1
Redis 2
Redis 3
Redis 4
Redis 5
```

那么使用这个锁的客户端，需要知道这 5 个 Redis 的地址。

例如：

```go
redlock := NewRedlock(
    redis1,
    redis2,
    redis3,
    redis4,
    redis5,
)
```

然后：

```go
lock, err := redlock.Lock(
    ctx,
    "order:123",
    30*time.Second,
)
```

内部大概做：

```txt
Client
  │
  ├── SET order:123 token NX PX 30s → Redis1
  ├── SET order:123 token NX PX 30s → Redis2
  ├── SET order:123 token NX PX 30s → Redis3
  ├── SET order:123 token NX PX 30s → Redis4
  └── SET order:123 token NX PX 30s → Redis5
                       │
                       ↓
                 Client 统计结果
                       │
                  3/5 成功？
```

所以：

```txt
Redlock 的客户端确实需要知道这一组 Redis 节点。
```

这和普通 Redis Cluster 客户端的使用方式不一样。

### 5. 但是“缓存 Redis”和“锁 Redis”最好分开考虑

这是架构设计里面非常重要的一点。

比如你有：

```txt
Redis Cluster
    ↓
缓存
```

存：

```txt
user:123
product:456
article:789
```

然后另外部署：

```txt
Redlock Redis 1
Redlock Redis 2
Redlock Redis 3
Redlock Redis 4
Redlock Redis 5
```

专门用来：

```txt
distributed lock
```

那么业务代码可能是：

```txt
                  Order Service
                   /        \
                  /          \
                 ↓            ↓
          Redis Cluster     Redlock
              Cache          Lock
                │              │
                │       ┌──────┼──────┐
                │       ↓      ↓      ↓
                │      R1     R2     R3...
                │
           user/product
```

这样反而比较清晰。

### 6. 为什么不直接拿一个 Redis Cluster 做 Redlock？

这是一个非常好的问题。

比如：

```txt
Redis Cluster

Node A
Node B
Node C
Node D
Node E
```

你可能想：

```txt
“这不就是 5 个 Redis 吗？我直接拿它们做 Redlock 不就行了？”
```

问题是：

```txt
它们不是 Redlock 所要求的“独立 Redis master 实例”模型。
```

Redis Cluster 本身有：

```txt
slot
sharding
replication
failover
```

节点之间存在集群拓扑关系。

而 Redlock 的模型是：

```txt
Independent Redis masters

R1     R2     R3     R4     R5
│      │      │      │      │
独立   独立   独立   独立   独立
```

Redlock 依靠的是：

```txt
majority
```

而不是 Redis Cluster 的：

```txt
slot ownership
```

所以不要把：

```txt
Redis Cluster
```

理解成：

```txt
5 个 Redis = Redlock
```

### 7. 那一个业务系统可能同时使用两套 Redis 吗？

完全可以，而且实际上架构上很容易理解。

比如：

```txt
                    Order Service
                         │
             ┌───────────┴───────────┐
             │                       │
             ↓                       ↓
       Redis Cluster              Redlock
          Cache                    Lock
             │                       │
             ↓              ┌────────┼────────┐
       order:123           R1       R2       R3...
       user:123
       product:123
```

代码层面：

```go
cacheRedis.Get(...)
```

和：

```go
distributedLock.Lock(...)
```

是两个不同的组件。

### 8. 但是有一个更重要的问题：你的业务真的需要 Redlock 吗？

比如：

```txt
订单处理
```

你可能觉得：

```txt
必须用 Redlock！
```

但是如果数据库本身可以做：

```sql
UPDATE orders
SET status = 'processing'
WHERE id = 123
  AND status = 'pending';
```

然后：

```txt
affected_rows == 1
```

就表示：

```txt
我成功抢到了订单
```

那可能根本不需要 Redis 分布式锁。

这就是分布式系统里面一个非常重要的原则：

```txt
不要为了“防止并发”自动选择分布式锁；优先考虑业务本身能不能通过数据库条件更新、唯一约束、幂等 ID、消息队列等方式解决。
```

### 9. Redlock 最适合的地方

可以想象这样一个场景：

```txt
Worker A
Worker B
Worker C
```

需要保证：

```txt
同一个外部资源
同一时间只有一个 worker 操作
```

而外部资源：

```txt
没有事务
没有 CAS
没有唯一约束
没有幂等机制
```

这时候分布式锁的价值就比较明显。

例如：

```txt
┌──────────────┐
│ 外部设备      │
│ 只能一个连接 │
└──────┬───────┘
       │
       │ lock
       ↓
    Redis
```

### 10. 你可以这样理解整个架构

**普通缓存**

```txt
业务服务
    │
    ↓
Redis Cluster
    │
    ├── Cache
    ├── Session
    ├── Counter
    └── ...
```

业务通常不关心具体 Redis 节点。

**Redlock**

```txt
业务服务
    │
    ↓
Redlock Client
    │
    ├──── Redis A
    ├──── Redis B
    ├──── Redis C
    ├──── Redis D
    └──── Redis E
```

Redlock Client 明确知道这组独立 Redis，然后自己计算：

```txt
成功数 >= majority
```

### 最关键的一句话

你刚才的理解可以修正成：

```txt
普通 Redis 副本/Cluster 是为了让“Redis 服务”高可用；Redlock 是让“分布式锁”不要因为某一个 Redis 故障转移而轻易丢失互斥性。
```

所以它们不是互相替代的关系。

而在实际系统中完全可能是：

```txt
                业务服务
                   │
        ┌──────────┴──────────┐
        ↓                     ↓
   Redis Cluster          Redlock
     （缓存）              （锁）
        │                     │
   自动路由/分片          R1 R2 R3 R4 R5
```

**缓存业务不需要指定某一个 Redlock Redis；只有使用 Redlock 的锁客户端需要知道那组独立 Redis。**

另外，如果你正在学习这个问题，我建议下一步重点理解 “Redis Cluster + Sentinel + Replication + Redlock 四者到底是什么关系”。这四个概念特别容易混在一起，但把它们画成一张架构图之后就非常清楚了。
