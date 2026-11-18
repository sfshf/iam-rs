# saga

## 给我讲解一下saga pattern，哪些应用、哪些场景能用到saga pattern

ChatGPT 答：

可以把 Saga Pattern 理解成：

**把一个跨多个服务的大事务，拆成多个本地事务；每一步成功就继续，后面的步骤失败，就执行前面步骤对应的“补偿操作”。**

它最适合解决的是：**微服务之间各自拥有数据库，但一个业务流程又必须跨越多个服务的问题。**

### 1. 先看一个最典型的例子：电商下单

假设一个订单需要：

```txt
下单
 ↓
扣库存
 ↓
支付
 ↓
创建物流单
```

四个微服务：

```txt
                  Order
                    │
          ┌─────────┼─────────┐
          ↓         ↓         ↓
      Inventory   Payment   Shipping
          │         │         │
         DB        DB        DB
```

每个服务自己的数据库：

```txt
Order DB
Inventory DB
Payment DB
Shipping DB
```

这时候你无法简单地：

```sql
BEGIN;

Order DB
Inventory DB
Payment DB
Shipping DB

COMMIT;
```

因为它们不是同一个本地数据库事务。

Saga 就是解决这种问题的一种模式。

### 2. Saga 的核心思想

假设：

```txt
T1 = 创建订单
T2 = 扣库存
T3 = 支付
T4 = 创建物流单
```

对应补偿操作：

```txt
C1 = 取消订单
C2 = 恢复库存
C3 = 退款
C4 = 取消物流单
```

成功：

```txt
T1 → T2 → T3 → T4
```

全部完成：

```txt
SUCCESS
```

如果：

```txt
T1 → T2 → T3 → T4 ❌
```

就反向补偿：

```txt
T3 → C3
T2 → C2
T1 → C1
```

最终：

```txt
订单：取消
库存：恢复
支付：退款
```

这就是 Saga 最核心的思想。

AWS 对 Saga 的定义也是：Saga 是一系列本地事务；如果后续事务失败，就执行补偿事务来撤销之前已经完成的本地事务。

### 3. 注意：Saga 不是数据库 Rollback

这是理解 Saga 最重要的一点。

普通数据库：

```txt
BEGIN
 ↓
UPDATE A
 ↓
UPDATE B
 ↓
UPDATE C
 ↓
ROLLBACK
```

数据库可以真正回滚。

但是 Saga：

```txt
T1 创建订单
 ↓
T2 扣库存
 ↓
T3 支付
 ↓
T4 失败
```

不能：

```txt
ROLLBACK
```

而是：

```txt
T4 ❌
 ↓
C3 退款
 ↓
C2 恢复库存
 ↓
C1 取消订单
```

所以：

```txt
Saga 的补偿不是数据库意义上的 rollback，而是业务意义上的反向操作。
```

例如：

```txt
扣款 100
```

对应：

```txt
退款 100
```

这两个操作在业务上可以抵消，但它们不是数据库事务意义上的原子回滚。

这也是 Saga 最大的特点和限制之一。Saga 缺少传统 ACID 中的自动回滚和隔离能力，因此补偿和并发控制都需要业务设计。

### 4. Saga 有两种主要实现

这是你学习 Saga 时必须掌握的。

```txt
                 Saga
                  │
          ┌───────┴───────┐
          ↓               ↓
    Choreography      Orchestration
      编排/协同           协调/编排器
```

### 5. Choreography：事件驱动

没有一个中央协调器。

例如：

```txt
Order Service
      │
      │ OrderCreated
      ↓
Inventory Service
      │
      │ StockReserved
      ↓
Payment Service
      │
      │ PaymentSucceeded
      ↓
Shipping Service
```

每个服务：

1. 执行自己的本地事务
2. 发布事件
3. 下一个服务监听事件
4. 继续执行

例如：

```txt
OrderCreated
     ↓
Inventory
     ↓
StockReserved
     ↓
Payment
     ↓
PaymentSucceeded
     ↓
Shipping
```

如果 Payment 失败：

```txt
PaymentFailed
     ↓
Inventory
     ↓
ReleaseStock
     ↓
Order
     ↓
CancelOrder
```

AWS 将这种方式定义为 choreography：各服务通过订阅彼此发布的事件来推进 Saga。

**Choreography 的优点**

最大的优点：

```txt
没有中央协调器，服务之间比较松耦合。
```

例如：

```txt
Order
  ↓ event
Inventory
  ↓ event
Payment
  ↓ event
Shipping
```

Order 不需要知道 Payment 怎么实现。

**但是它有一个非常明显的问题**

服务一多：

```txt
A → B
B → C
C → D
D → A
A → E
B → F
E → G
```

最终你很难回答：

```txt
“这个订单到底经历了什么？”
```

所以：

```txt
3 个服务
```

可能还挺舒服。

但是：

```txt
20 个服务
```

Choreography 很容易变成事件依赖网络。

AWS 也明确指出，参与者增多以后，Choreography 的依赖关系、重试、补偿和可观测性都会变得更加复杂。

### 6. Orchestration：中央协调器

另一种方式是：

```txt
              Saga Orchestrator
                     │
        ┌────────────┼────────────┐
        ↓            ↓            ↓
      Order       Inventory     Payment
```

Coordinator 负责整个流程：

```txt
1. CreateOrder
       ↓
2. ReserveStock
       ↓
3. Pay
       ↓
4. CreateShipping
```

如果支付失败：

```txt
Payment ❌
   ↓
ReleaseStock
   ↓
CancelOrder
```

也就是：

```txt
              Saga
               │
     ┌─────────┼─────────┐
     ↓         ↓         ↓
   Order    Inventory  Payment
```

中央协调器知道：

```txt
当前执行到哪一步
下一步是什么
失败以后怎么办
如何重试
如何补偿
```

AWS 把这种模式定义为由中央 orchestrator 管理整个事务生命周期。它更适合参与者较多、流程复杂的 Saga。

### 7. 两种模式怎么选择？

简单记：

|            | Choreography | Orchestration   |
| ---------- | ------------ | --------------- |
| 中央协调器 | ❌           | ✅              |
| 通信方式   | Event        | Command/Request |
| 服务耦合   | 较松         | 较集中          |
| 流程可见性 | 较差         | 较好            |
| 服务少     | 很合适       | 也可以          |
| 服务多     | 容易复杂     | 通常更容易管理  |
| 重试/超时  | 分散         | 集中            |
| 调试       | 较困难       | 相对容易        |
| 典型       | Event-driven | Workflow        |

不过不要简单理解为：

```txt
Choreography = 好
Orchestration = 不好
```

它们是两种不同的协调方式。AWS 的资料也强调，两者分别适合不同的服务协作场景。

### 8. 哪些业务非常适合 Saga？

这是最重要的部分。

#### 场景一：电商订单

这是 Saga 的经典场景。

```txt
创建订单
 ↓
锁库存
 ↓
扣优惠券
 ↓
支付
 ↓
创建物流
```

例如：

```txt
CreateOrder       ✅
ReserveStock      ✅
UseCoupon         ✅
Payment           ❌
```

补偿：

```txt
ReleaseCoupon
ReleaseStock
CancelOrder
```

这就是非常典型的 Saga。

### 9. 场景二：酒店/机票/旅游预订

比如旅游套餐：

```txt
预订机票
 ↓
预订酒店
 ↓
预订租车
 ↓
扣款
```

假设：

```txt
机票     ✅
酒店     ✅
租车     ❌
```

就可以：

```txt
取消酒店
取消机票
退款
```

这种业务特别适合 Saga，因为它天然就是：

```txt
多个业务步骤 + 每一步都有对应的取消/补偿操作。
```

### 10. 场景三：支付流程

例如：

```txt
CreateOrder
    ↓
ReserveBalance
    ↓
Payment
    ↓
MerchantSettlement
```

如果：

```txt
Payment
   ↓
成功
   ↓
Settlement
   ↓
失败
```

可以：

```txt
RefundPayment
ReleaseBalance
CancelOrder
```

不过这里需要注意：

**金融核心账务不能简单依赖 Saga 来实现强一致扣账。**

例如银行核心账户余额这种场景，通常需要更严格的事务和账务模型。

Saga 更适合外围业务流程，例如：

```txt
订单
支付订单状态
优惠券
库存
物流
通知
```

而不是简单用：

```txt
Saga
 ↓
银行核心账本
```

### 11. 场景四：物流

例如：

```txt
创建订单
 ↓
分配仓库
 ↓
生成拣货任务
 ↓
生成配送单
 ↓
通知用户
```

如果：

```txt
分配仓库
 ↓
失败
```

就可以：

```txt
取消订单
```

如果：

```txt
生成配送单
 ↓
失败
```

可以：

```txt
取消拣货任务
释放仓库资源
```

### 12. 场景五：SaaS 开通资源

这个其实非常适合微服务。

用户购买：

```txt
Pro Plan
```

系统需要：

```txt
创建租户
 ↓
创建数据库
 ↓
创建对象存储 Bucket
 ↓
创建 Redis Namespace
 ↓
创建默认用户
 ↓
创建权限
 ↓
开通套餐
```

假设：

```txt
创建租户          ✅
创建数据库        ✅
创建 Bucket       ✅
创建 Redis        ❌
```

Saga 可以：

```txt
删除 Bucket
删除数据库
删除租户
```

然后：

```txt
Provision Failed
```

这种场景特别适合 Saga，因为它属于：

```txt
跨多个基础设施/服务的长流程。
```

### 13. 场景六：订单履约 / 长事务

Saga 最适合的其实不是几毫秒的数据库操作，而是：

```txt
持续时间比较长、跨多个服务、无法锁住所有资源的业务流程。
```

比如：

```txt
下单
 ↓
审核
 ↓
风控
 ↓
支付
 ↓
仓库
 ↓
物流
 ↓
签收
```

可能持续：

```txt
几秒
几分钟
甚至更长
```

你显然不能：

```txt
BEGIN
 ↓
锁住数据库资源
 ↓
等待物流
 ↓
等待用户
 ↓
COMMIT
```

这显然不现实。

Saga 可以：

```txt
Step 1 local commit
        ↓
Step 2 local commit
        ↓
Step 3 local commit
        ↓
...
```

AWS 也把长生命周期事务、避免一个服务长时间阻塞其他服务列为 Saga 的适用场景。

### 14. 哪些场景不适合 Saga？

这个同样重要。

#### 第一类：必须强一致

例如：

```txt
银行核心账务
证券成交
核心余额系统
```

如果你的要求是：

```txt
任何时刻都必须满足：

A.balance + B.balance = Constant
```

那么 Saga 的最终一致性语义可能不适合直接承担这个核心约束。

#### 第二类：没有合理补偿操作

例如：

```txt
发送短信
```

你可以：

```txt
SendSMS()
```

但是失败以后：

```txt
CompensateSMS()
```

并没有真正意义上的：

```txt
UndoSMS()
```

所以 Saga 并不是：

```txt
“所有分布式操作都能加一个 rollback 就行”。
```

必须问：

```txt
这个业务操作有没有合理的补偿语义？
```

### 15. 一个很有意思的例子：发货

假设：

```txt
CreateShipment()
```

成功之后：

```txt
快递公司已经取走包裹
```

这时候后面发生：

```txt
Payment problem
```

你能不能：

```txt
Rollback Shipment
```

？

技术上可以设计：

```txt
CancelShipment()
```

但是业务上可能已经产生现实世界的副作用。

这说明：

```txt
Saga 的补偿是业务补偿，不是时间机器。
```

这是学习 Saga 时非常重要的思维。

### 16. Saga 最大的问题：隔离性

假设：

```txt
用户 A
购买最后一件商品
```

Saga：

```txt
T1 ReserveStock
```

但是还没有最终完成。

此时：

```txt
用户 B
```

也可能看到这个库存状态。

Saga 本身没有传统数据库事务的：

```txt
Isolation
```

所以可能出现：

```txt
A Saga
   ↓
库存预留

B Saga
   ↓
读取库存

A Saga
   ↓
失败
   ↓
释放库存
```

于是 B 可能在中间状态下做出了决策。

**这也是 Saga 的重要限制：它缺少 ACID 的传统隔离能力，需要额外使用业务级并发控制/语义锁等手段。**

### 17. 所以实际 Saga 系统通常还需要这些东西

一个真正能用的 Saga，通常不是：

```txt
Saga
```

单独存在，而是：

```txt
                   Saga
                    │
      ┌─────────────┼──────────────┐
      ↓             ↓              ↓
   Idempotency    Retry          Timeout
      │             │              │
      └─────────────┼──────────────┘
                    ↓
               Message Queue
                    │
                    ↓
           Transactional Outbox
                    │
                    ↓
             Observability
```

尤其是：

#### 幂等

```txt
Pay(orderID)
```

重复执行两次：

```txt
第一次 → 扣款
第二次 → 不应该再次扣款
```

#### Retry

网络错误：

```txt
Payment
   ↓
timeout
   ↓
retry
```

#### Timeout

```txt
Inventory
   ↓
10s
   ↓
timeout
   ↓
Compensation
```

#### Outbox

解决：

```txt
DB commit
+
Publish Event
```

之间的双写问题。

AWS 也特别指出，Saga choreography 中需要考虑数据库更新和消息发布之间的 dual-write 问题，Transactional Outbox 是常见解决方式。

### 18. 如果用 Go 实现，一个非常典型的结构

例如：

```txt
Order Service
Inventory Service
Payment Service
```

使用 Orchestration：

```txt
                Saga Coordinator
                       │
              ┌────────┼────────┐
              ↓        ↓        ↓
            Order   Inventory  Payment
```

Go 里面可以把一个 Saga 描述成：

```go
type Step struct {
    Execute   func(ctx context.Context) error
    Compensate func(ctx context.Context) error
}
```

然后：

```go
steps := []Step{
    {
        Execute:   createOrder,
        Compensate: cancelOrder,
    },
    {
        Execute:   reserveStock,
        Compensate: releaseStock,
    },
    {
        Execute:   pay,
        Compensate: refund,
    },
}
```

正常：

```txt
Execute 1
 ↓
Execute 2
 ↓
Execute 3
 ↓
SUCCESS
```

失败：

```txt
Execute 1 ✅
 ↓
Execute 2 ✅
 ↓
Execute 3 ❌
 ↓
Compensate 2
 ↓
Compensate 1
 ↓
FAILED
```

**当然，生产实现远远不止这几行代码，还必须考虑状态持久化、重试、幂等、超时、宕机恢复、消息投递、并发 Saga 等问题。**

### 19. 你可以把 Saga 和 TCC 这样区分

你刚刚问过 TCC，所以这两个放一起比较特别有帮助：

```txt
TCC：

Try
 ↓
预留资源
 ↓
Confirm
   或
Cancel
```

而 Saga：

```txt
Local Tx 1
 ↓
Local Tx 2
 ↓
Local Tx 3
 ↓
失败
 ↓
Compensation
```

例如库存：

#### TCC

```txt
Try:
预留库存 1

Confirm:
真正扣库存

Cancel:
释放库存
```

#### Saga

```txt
扣库存
 ↓
支付失败
 ↓
恢复库存
```

所以：

```txt
TCC 更强调“预留 → 确认/取消”；Saga 更强调“本地事务链 → 补偿”。
```

### 20. 最后给你一个判断口诀

以后看到一个业务流程，可以先问：

```txt
① 是否跨多个服务？
② 每个服务是否有自己的 DB？
③ 是否无法使用一个本地 ACID 事务？
④ 是否允许最终一致？
⑤ 每个操作是否存在合理的补偿动作？
⑥ 事务是否可能持续较长时间？
```

如果基本都是：

```txt
YES
YES
YES
YES
YES
YES
```

那么 Saga 非常值得考虑。

比如：

```txt
电商订单
    ✓

酒店预订
    ✓

机票预订
    ✓

物流履约
    ✓

SaaS 资源开通
    ✓

跨服务订单流程
    ✓

银行核心账本
    ✗/谨慎

强一致余额扣减
    ✗/谨慎

简单单库 CRUD
    ✗
```

### 推荐你接下来这样学

你现在已经连续问了 **Raft → 分布式锁 → 分布式事务 → TCC/Saga**，其实已经可以把它们串起来了：

```txt
              分布式系统
                   │
      ┌────────────┼────────────┐
      ↓            ↓            ↓
     Raft       Distributed   Transaction
                  Lock            │
                                  ↓
                         ┌────────┴────────┐
                         ↓                 ↓
                        TCC              Saga
                                           │
                              ┌────────────┴───────────┐
                              ↓                        ↓
                       Choreography             Orchestration
                              │                        │
                              └──────────┬─────────────┘
                                         ↓
                                MQ + Outbox + Retry
                                         ↓
                                    最终一致性
```

如果你想真正把 Saga 学透，下一步最值得做的是一个 Go + MySQL + Redis/RabbitMQ 的 Order / Inventory / Payment 三服务 Saga Demo，然后分别模拟 支付失败、服务宕机、消息重复、消息丢失、超时、补偿失败。这比单纯读概念更容易真正理解 Saga。
