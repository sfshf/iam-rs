# 分布式事务

## 给我讲解一下分布式事务（如TCC/Saga），并给我资源地址

ChatGPT答：

可以。分布式事务最好不要一上来就背 TCC / Saga / 2PC，先理解一个核心问题：

```txt
一个业务操作跨越多个独立服务/数据库时，怎样让它最终处于一个业务上正确的状态？
```

### 1. 为什么会出现分布式事务？

单体应用里可能是：

```txt
Order Service
     │
     ↓
   MySQL
 ┌───────────────┐
 │ orders        │
 │ inventory     │
 │ payment       │
 └───────────────┘
```

例如下单：

```txt
BEGIN;

创建订单
扣库存
扣账户余额

COMMIT;
```

只要都在同一个数据库事务里，MySQL 可以保证：

```txt
全部成功
   OR
全部回滚
```

这就是传统 ACID。

但是微服务以后：

```txt
                  下单
                   │
        ┌──────────┼──────────┐
        ↓          ↓          ↓
   Order Service Inventory  Payment
        │          │          │
       DB         DB         DB
```

现在：

```txt
创建订单
   ↓
扣库存
   ↓
扣余额
```

分别操作三个数据库。

问题来了：

```txt
创建订单       ✅
扣库存         ✅
扣余额         ❌
```

怎么办？

订单已经创建了，库存也扣了，但是支付失败。

这就是分布式事务/分布式数据一致性要解决的问题。

在微服务架构下，每个服务通常拥有自己的数据库，因此一个跨服务操作天然跨越多个本地事务边界。（[microservices.io saga](https://microservices.io/patterns/data/saga.html?utm_source=chatgpt.com)）

### 2. 先理解一个非常重要的变化

传统事务追求：

```txt
Atomicity
Consistency
Isolation
Durability
```

尤其是：

```txt
BEGIN
 ↓
A
 ↓
B
 ↓
C
 ↓
COMMIT
```

失败：

```txt
ROLLBACK
```

而微服务中的 TCC / Saga 很多时候变成：

```txt
A 成功
 ↓
B 成功
 ↓
C 失败
 ↓
补偿 A
 ↓
补偿 B
```

也就是说：

```txt
从“数据库自动回滚”变成“业务层设计补偿”。
```

这正是 Saga 最核心的思想。Saga 是一系列本地事务；如果后面的步骤失败，就执行之前步骤对应的补偿事务。

### 3. 先看 2PC，再理解 TCC

理解 TCC 前，建议先理解 2PC（Two-Phase Commit）。

假设：

```txt
   Transaction Coordinator
             │
   ┌─────────┼─────────┐
   ↓         ↓         ↓
   DB1       DB2       DB3
```

第一阶段：

```txt
PREPARE

DB1 → OK
DB2 → OK
DB3 → OK
```

协调器：

```txt
全部 OK
 ↓
COMMIT
```

第二阶段：

```txt
COMMIT

DB1 → COMMIT
DB2 → COMMIT
DB3 → COMMIT
```

如果某个参与者不能 prepare：

```txt
DB1 → OK
DB2 → OK
DB3 → FAIL
        ↓
     ROLLBACK
```

**但是 2PC 有明显问题**

比如：

```txt
Coordinator
     │
     ├── DB1 PREPARE
     ├── DB2 PREPARE
     └── DB3 PREPARE
             ↓
          全部 OK
             ↓
          COMMIT
```

如果协调器在这里：

```txt
          COMMIT
             ↓
        Coordinator 💥
```

参与者可能已经进入 prepared 状态，却不知道最终应该 commit 还是 rollback。

因此 2PC 会产生：

- 阻塞
- 协调器依赖
- 性能问题
- 长事务问题
- 故障恢复复杂

所以现代微服务里经常会采用 Saga、TCC 等方式，而不是简单地把所有业务都做成 2PC。AWS 和 microservices.io 都把 Saga 定位为跨多个微服务/数据存储维护一致性的模式。（[AWS Documents 1](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/saga-choreography.html?utm_source=chatgpt.com)）

### 4. TCC 到底是什么？

TCC：

```txt
Try
Confirm
Cancel
```

它实际上可以理解为：

```txt
把一个业务操作人为拆成“预留资源、确认、取消”三个业务接口。
```

例如电商买商品。

原始操作：

```txt
购买商品
```

拆成：

```txt
Try
 ↓
预留库存

Confirm
 ↓
真正扣库存

Cancel
 ↓
释放库存
```

### 5. 一个完整 TCC 例子

假设：

```txt
用户余额 = 1000
商品库存 = 10
商品价格 = 100
```

用户买一个商品。

#### Try 阶段

库存服务：

```txt
TryReserveStock()
```

结果：

```txt
库存：

10
 ↓
reserved = 1
available = 9
```

账户服务：

```txt
TryFreezeMoney(100)
```

结果：

```txt
balance = 1000
frozen  = 100
available = 900
```

**注意：**

```txt
Try 并不是最终提交，而是“检查 + 预留”。
```

[Seata 官方文档](https://seata.apache.org/docs/v2.2/user/mode/tcc/?utm_source=chatgpt.com)也是这样定义 TCC 的：Try 负责检查和预留资源，Confirm 执行提交，Cancel 取消预留并恢复状态。

### 6. Try 全部成功

```txt
                 Transaction
                      │
          ┌───────────┴───────────┐
          ↓                       ↓
   Inventory Try             Account Try
       OK                         OK
```

然后：

```txt
Confirm
```

库存：

```txt
reserved 1
   ↓
真正扣减
```

账户：

```txt
frozen 100
   ↓
真正扣款
```

最终：

```txt
库存 = 9
余额 = 900
```

### 7. 如果 Try 阶段失败

比如：

```txt
Inventory Try     OK
Account Try       FAIL
```

那么：

```txt
Cancel Inventory
```

释放之前预留的库存：

```txt
reserved = 1
available = 9

        ↓ Cancel

reserved = 0
available = 10
```

于是恢复业务状态。

### 8. TCC 和 Saga 的关键区别

这是最值得理解的地方。

**TCC**

思路：

```txt
Try
 ↓
资源预留
 ↓
Confirm / Cancel
```

例如：

```txt
Try:
冻结 100 元

Confirm:
真正扣 100 元

Cancel:
解冻 100 元
```

**Saga**

思路是：

```txt
本地事务 A
    ↓
本地事务 B
    ↓
本地事务 C
```

如果 C 失败：

```txt
C ❌
↓
Compensate B
↓
Compensate A
```

例如：

```txt
创建订单
   ↓
扣库存
   ↓
扣余额
   ↓
支付失败
```

**Saga：**

```txt
支付失败
   ↓
补偿扣余额
   ↓
恢复库存
   ↓
取消订单
```

所以：

```txt
TCC 更像“预留 → 确认/取消”。


Saga 更像“先做 → 出错以后执行业务补偿”。
```

### 9. Saga 的两种实现方式

Saga 又可以分成：

```txt
Choreography
Orchestration
```

#### 方式一：Choreography

没有中央协调器。

```txt
Order
  │
  │ OrderCreated
  ↓
Inventory
  │
  │ StockReserved
  ↓
Payment
  │
  │ PaymentFailed
  ↓
Inventory
  │
  │ StockReleased
  ↓
Order
  │
  │ Cancelled
```

每个服务监听事件，然后触发下一步。

这种方式比较松耦合，但服务多了以后，整个业务流程可能变得非常难追踪。AWS 也指出，参与者较少时 choreography 比较简单，但参与者增加后依赖关系会变复杂。（[AWS Document 2](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/saga-patterns.html?utm_source=chatgpt.com)）

#### 方式二：Orchestration

有一个 Saga Coordinator：

```txt
             Saga Coordinator
                    │
          ┌─────────┼─────────┐
          ↓         ↓         ↓
       Order     Inventory   Payment
```

Coordinator：

```txt
1. CreateOrder
       ↓
2. ReserveStock
       ↓
3. Pay
       ↓
4. ConfirmOrder
```

如果 Payment 失败：

```txt
Payment ❌
   ↓
ReleaseStock
   ↓
CancelOrder
```

这种方式的优点是：

```txt
整个业务流程非常清晰。
```

AWS 对 Saga orchestration 的描述也是由 orchestrator 管理整个事务生命周期，并根据参与者返回结果决定下一步操作。（[AWS -- Saga Pattern](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/saga-patterns.html?utm_source=chatgpt.com)）

### 10. TCC 和 Saga 的一个直观对比

|                  | TCC                    | Saga                |
| ---------------- | ---------------------- | ------------------- |
| 核心思想         | Try/Confirm/Cancel     | 本地事务 + 补偿事务 |
| 是否需要业务改造 | 很高                   | 较高                |
| 是否需要设计补偿 | Cancel                 | Compensation        |
| 是否强调资源预留 | 是                     | 不一定              |
| 适合             | 强业务一致性、资源预留 | 长事务、业务流程    |
| 典型场景         | 库存、余额、额度       | 订单、支付、物流    |
| 数据库依赖       | 可以较弱               | 通常依赖本地事务    |
| 实现复杂度       | 高                     | 中高                |
| 长流程           | 不太适合               | 很适合              |

不过不要把它理解成：

```txt
TCC = 强一致
Saga = 弱一致
```

这么简单。

实际上二者都是在分布式环境下通过业务协议维持最终一致性/业务一致性，具体语义取决于你的业务设计。

### 11. 一个非常现实的问题：Saga 并不能“真正回滚”

这是学习分布式事务最重要的坑之一。

假设：

```txt
扣款 100 元
```

然后：

```txt
支付成功
```

之后：

```txt
物流失败
```

你不能真的：

```txt
ROLLBACK
```

因为：

```txt
银行系统
第三方支付
物流系统
```

可能已经发生真实世界的副作用。

所以 Saga 只能：

```txt
原操作
   ↓
补偿操作
```

比如：

```txt
扣款 100
   ↓
退款 100
```

但是：

```txt
扣款 100 + 退款 100
```

并不严格等价于数据库：

```txt
ROLLBACK
```

这就是分布式事务和传统数据库事务非常本质的区别。

Saga 也存在隔离性不足的问题，因此需要通过业务设计、幂等、重试、状态机等机制处理并发数据异常。（[microservice.io saga](https://microservices.io/patterns/data/saga.html?utm_source=chatgpt.com)）

### 12. 为什么幂等性特别重要？

假设：

```txt
ConfirmPayment()
```

执行成功：

```txt
扣款 100
```

但是响应丢了：

```txt
Payment Service → Coordinator
                     ❌
```

Coordinator 不知道成功还是失败，于是：

```txt
Retry ConfirmPayment()
```

如果你的代码：

```go
func ConfirmPayment() {
    balance -= 100
}
```

那么：

```txt
第一次：1000 → 900
第二次：900 → 800

💥
```

所以必须设计成幂等：

```txt
if transactionID already processed {
    return success
}

performPayment()

mark transactionID processed
```

这也是 TCC/Saga 实现中非常重要的一部分。

### 13. 还有一个经常和 Saga 一起出现的东西：Transactional Outbox

比如：

```txt
Order Service
```

需要同时：

```txt
UPDATE orders
INSERT event
```

然后发送：

```txt
OrderCreated
```

如果你：

```txt
DB COMMIT
   ↓
send Kafka
```

中间：

```txt
DB COMMIT
   ↓
💥 Service Crash
```

那么：

```txt
数据库：成功
Kafka：没有消息
```

Saga 就断了。

所以常见方案是：

```txt
MySQL
┌────────────────────────┐
│ orders                 │
│ outbox_events          │
└────────────────────────┘
          ↓
       COMMIT
          ↓
     Outbox Relay
          ↓
        Kafka
```

把：

```txt
修改业务数据
+
记录事件
```

放到同一个本地数据库事务里。

这就是 Transactional Outbox。 （[microservices.io transactional outbox](https://microservices.io/patterns/data/transactional-outbox?utm_source=chatgpt.com)）

### 14. 如果你想真正理解分布式事务，我建议这样学

按照这个顺序：

```txt
        ACID
         ↓
     本地事务
         ↓
       2PC
         ↓
       3PC
         ↓
     TCC / Saga
         ↓
  Transactional Outbox
         ↓
      幂等 / 重试
         ↓
    消息最终一致性
         ↓
  分布式事务框架实现
```

不要一开始直接看 Seata 源码。

先自己实现一个非常小的：

```txt
Order
Inventory
Payment
```

三个 Go 服务。

### 15. 我特别推荐你看的资源

#### ① Chris Richardson：Saga Pattern

这是我最推荐的 Saga 入门资料。

[Saga Pattern — microservices.io](https://microservices.io/patterns/data/saga.html?utm_source=chatgpt.com)

里面直接讲：

- Saga 是什么
- 为什么需要 Saga
- Choreography
- Orchestration
- Compensation
- Saga 的问题
- Transactional Outbox 等相关模式

而且内容和实际微服务设计联系非常紧密。

#### ② AWS：Saga Pattern

[AWS Saga Pattern](https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-data-persistence/saga-pattern.html?utm_source=chatgpt.com)

AWS 的资料比较适合从工程架构角度理解 Saga，尤其是长事务、补偿、微服务协调。

#### ③ AWS：Saga Patterns

[AWS Saga Patterns — Choreography / Orchestration](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/saga-patterns.html?utm_source=chatgpt.com)

这篇重点看：

```txt
Choreography
vs
Orchestration
```

非常值得看。

#### ④ Apache Seata：TCC

如果你想深入 TCC，我推荐直接看 Seata 官方文档：

[Apache Seata TCC 官方文档](https://seata.apache.org/zh-cn/docs/next/user/mode/tcc/?utm_source=chatgpt.com)

里面对：

```txt
Try
Confirm
Cancel
TM
RM
XID
Branch ID
```

都有比较明确的解释。

#### ⑤ Seata 深度剖析 TCC

[Seata：深度剖析 TCC 模式](https://seata.apache.org/zh-cn/blog/seata-tcc/?utm_source=chatgpt.com)

这篇更适合已经理解 TCC 基本概念以后看，里面会涉及 TCC 为什么具有业务侵入性，以及为什么 TCC 能够跨数据库、跨应用资源。

#### ⑥ Transactional Outbox

[Transactional Outbox — microservices.io](https://microservices.io/patterns/data/transactional-outbox?utm_source=chatgpt.com)

这个我强烈建议和 Saga 一起学习。

因为实际项目里经常不是：

```txt
Saga
```

单独存在，而是：

```txt
Saga
 +
Message Queue
 +
Transactional Outbox
 +
Idempotency
 +
Retry
```

共同组成一套最终一致性方案。

#### 16. 最后给你一张脑图

你可以先把整个领域记成：

```txt
                分布式事务
                    │
       ┌────────────┼────────────┐
       ↓            ↓            ↓
      2PC           TCC         Saga
       │            │            │
   Prepare       Try         Local Tx
   Commit        Confirm          ↓
   Rollback      Cancel       Compensation
       │            │            │
       └────────────┴────────────┘
                    ↓
             分布式一致性
                    │
       ┌────────────┼─────────────┐
       ↓            ↓             ↓
    幂等性         重试        消息可靠性
                                  │
                                  ↓
                         Transactional Outbox
                                  │
                                  ↓
                              MQ/Kafka
```

如果你是从 Go 后端/微服务角度学，我建议下一步重点搞懂 `TCC vs Saga vs 2PC` 的执行时序。尤其是把一个 `Order → Inventory → Payment` 的例子画成成功、Try 失败、Confirm 失败、网络超时、重复请求、Coordinator 宕机这几种时序图，你会一下子理解为什么分布式事务比普通数据库事务复杂得多。

## 有没有开源的，使用了分布式事务的go项目，给我地址

chatGPT 答：

有。如果你的目的不是单纯找“分布式事务框架”，而是想找可以直接读源码、跑起来、学习 TCC/Saga 实际工程实现的 Go 开源项目，我比较推荐下面几个。

### 1. DTM —— 最推荐你先看

[dtm-labs/dtm GitHub](https://github.com/dtm-labs/dtm?utm_source=chatgpt.com)

这是一个专门做分布式事务的 Go 项目，支持：

- Saga
- TCC
- XA
- 2-phase message
- Outbox
- Workflow
- HTTP / gRPC
- MySQL / Redis / PostgreSQL 等

官方仓库目前仍然维护，并且明确提供 Go SDK。

它特别适合你现在学习，因为代码和概念对应得非常直接：

```txt
                 DTM
                  │
       ┌──────────┼──────────┐
       ↓          ↓          ↓
     Saga         TCC        XA
       │          │
       ↓          ↓
   Compensation  Try
                Confirm
                Cancel
```

### 配套 Examples

[dtm-labs/dtm-examples GitHub](https://github.com/dtm-labs/dtm-examples?utm_source=chatgpt.com)

这个仓库更适合学习源码。

官方 examples 包含：

```txt
quick-start
saga
tcc
xa
message
outbox
workflow
HTTP
gRPC
```

官方文档明确把 examples 分成协议、事务模式、数据库等维度。

例如最简单的：

```txt
git clone https://github.com/dtm-labs/dtm-examples
cd dtm-examples
go run main.go qs
```

官方说明这个例子就是一个跨账户转账的分布式事务。

### 2. DTM Quick Start —— 非常适合你直接跑

[dtm-labs/quick-start-sample GitHub](https://github.com/dtm-labs/quick-start-sample?utm_source=chatgpt.com)

如果你不想一开始就啃 DTM 整个源码，我建议先从这个开始。

官方给出的例子就是：

```txt
Account A
   │
   │ TransOut
   ↓
DTM
   │
   │ TransIn
   ↓
Account B
```

也就是：

```txt
A -30
B +30
```

然后故意制造失败，看 Saga 怎么补偿。

官方还提供 workflow HTTP / gRPC 示例。

### 3. Seata-Go

如果你想看一个更加标准的分布式事务框架架构，可以看 Apache Seata 的 Go 实现。

[apache/incubator-seata-go GitHub](https://github.com/apache/incubator-seata-go?utm_source=chatgpt.com)

Seata-Go 的架构是：

```txt
              TC
       Transaction Coordinator
                 │
       ┌─────────┼─────────┐
       ↓         ↓         ↓
      RM        RM        RM
       │         │         │
      DB1       DB2       DB3

             ↑
             │
             TM
```

其中：

- TM：Transaction Manager
- TC：Transaction Coordinator
- RM：Resource Manager

Global Transaction：

```txt
Global Transaction
        │
   ┌────┼────┐
   ↓    ↓    ↓
Branch Branch Branch
  Tx     Tx     Tx
```

Seata-Go 官方 README 对这个生命周期有比较完整的说明：TM 创建全局事务，TC 分配 XID，RM 注册 branch transaction，最终由 TC 驱动各 branch commit/rollback。

它最大的优势是：

```txt
可以把 Java Seata 和 Go Seata 放在同一个分布式事务体系里面。
```

官方也明确说明 Seata-Go 与 Java Seata 可以互操作。

### 4. go-saga —— 如果你只想研究 Saga

[itimofeev/go-saga GitHub](https://github.com/itimofeev/go-saga?utm_source=chatgpt.com)

这个项目比 DTM 小很多。

它的定位就是：

```txt
Go implementation of Saga pattern
```

而且实现的是 Choreography-based Saga。

所以如果你的目的是：

```txt
“我想看看 Saga 最核心的代码到底是什么样。”
```

这个项目反而值得读。

相比：

```txt
DTM
 ↓
完整事务框架
 ↓
很多工程细节
```

它更加接近：

```txt
Saga
 ↓
Step 1
 ↓
Step 2
 ↓
Step 3
 ↓
Compensation
```

### 5. 如果你用 go-zero

你之前也在关注 go-zero，这个组合很值得看：

```txt
go-zero
   +
DTM
```

DTM 官方提供了 go-zero 的支持。

[DTM + go-zero 示例](https://github.com/dtm-labs/dtmdriver-clients/tree/main/gozero?utm_source=chatgpt.com)

例如：

```txt
               go-zero
             /         \
            ↓           ↓
      Account Service  Account Service
             \         /
                  ↓
                 DTM
```

可以看到 gRPC 服务如何参与：

```txt
Saga
TCC
2-phase message
```

对于你这种 Go 微服务方向，这个例子比单纯看理论更有价值。

### 6. 我建议你按这个顺序读

如果你的真正目标是学习分布式事务，而不是找一个马上生产使用的框架，我建议：

```txt
                    分布式事务
                         │
                         ↓
                 DTM Quick Start
                         │
                         ↓
                   Saga Example
                         │
              ┌──────────┴──────────┐
              ↓                     ↓
             Saga                   TCC
              ↓                     ↓
        Compensation         Try/Confirm/Cancel
              │                     │
              └──────────┬──────────┘
                         ↓
                  DTM 核心源码
                         │
                         ↓
                 Seata-Go 源码
```

重点看这几个东西：

#### 第一阶段

先跑：

```txt
dtm-examples
```

理解：

```txt
TransOut
TransIn
Compensate
```

#### 第二阶段

重点研究 DTM 的：

```txt
BranchBarrier
```

这个非常重要。

因为真实的 Saga/TCC 并不是：

```txt
if err {
    rollback()
}
```

这么简单。

它必须处理：

```txt
网络超时
重复请求
空补偿
悬挂
重试
幂等
```

DTM 的 Branch Barrier 就是在处理这些分布式事务非常棘手的问题。官方示例中也直接使用 `barrier.Call` 包裹本地数据库事务。

#### 第三阶段

再看：

```txt
TCC
```

你会看到：

```txt
           Global Transaction
                  │
             ┌────┴────┐
             ↓         ↓
            Try       Try
             │         │
             ↓         ↓
           Success   Success
             │         │
             └────┬────┘
                  ↓
              Confirm
```

失败则：

```txt
Try A   → OK
Try B   → OK
Try C   → FAIL

          ↓

Cancel B
          ↓
Cancel A
```

### 7. 最后推荐一个非常实际的学习项目

你可以直接基于 DTM examples 自己做一个：

```txt
distributed-order-demo
```

拆成三个 Go 服务：

```txt
order-service
inventory-service
payment-service
```

然后：

```txt
                    DTM
                     │
          ┌──────────┼──────────┐
          ↓          ↓          ↓
       Order      Inventory   Payment
        MySQL       MySQL       MySQL
```

实现：

```txt
CreateOrder
    ↓
ReserveStock
    ↓
Pay
    ↓
ConfirmOrder
```

然后故意制造：

```txt
① Payment 失败
② Inventory 失败
③ Confirm 超时
④ Cancel 重复调用
⑤ Payment 响应丢失
⑥ 服务在事务中途 Crash
```

再观察 Saga/TCC 怎么处理。

这一个项目基本能把：

**Raft → 分布式锁 → MQ → Saga → TCC → 幂等 → 重试 → Outbox → 最终一致性**

这一整套分布式系统知识串起来。

如果只选一个仓库开始，我建议你直接从 **dtm-examples → dtm 源码 → seata-go** 这个顺序走。
