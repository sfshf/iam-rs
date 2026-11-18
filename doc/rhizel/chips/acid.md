# 问： mysql数据库事务这样的事务解决的是什么核心问题，mysql是如何解决的，postgres是如何解决的？

ChatGPT 答：

数据库事务（Transaction）解决的核心问题是：

当一个业务操作需要修改多个数据时，如何保证这些修改作为一个整体执行，避免数据库进入业务不允许的中间状态，并且在并发访问、程序崩溃或机器故障时，仍然保持数据正确。

MySQL 和 PostgreSQL 都支持 ACID 事务，也都使用 MVCC（多版本并发控制）来处理并发读写，但底层实现有明显区别：

- MySQL（这里主要指 InnoDB）：通过 Undo Log、Redo Log、锁和 MVCC 等机制实现事务。
- PostgreSQL：通过数据行多版本、WAL（预写日志）、事务 ID、可见性判断和锁等机制实现事务。

要真正理解它们，建议先从事务究竟解决什么问题开始。

## 一、事务到底解决什么核心问题？

假设我们有一个银行转账业务：

账户 A 向账户 B 转账 100 元。

数据库中有两条记录：

```
accounts
+----+---------+
| id | balance |
+----+---------+
| A  | 1000    |
| B  | 500     |
+----+---------+
```

转账需要执行两个操作：

```
UPDATE accounts
SET balance = balance - 100
WHERE id = 'A';

UPDATE accounts
SET balance = balance + 100
WHERE id = 'B';
```

最终应该得到：

```
A = 900
B = 600
```

看起来只有两条 SQL，但实际存在很多问题。

### 1. 原子性：不能只执行一半

假设第一条 SQL 成功了，第二条 SQL 执行时数据库突然崩溃。

结果可能变成：

```
A = 900
B = 500
```

100 元凭空消失了。

事务要求这两个操作作为一个整体：

- 全部成功：提交（COMMIT）。
- 任意操作失败：回滚（ROLLBACK）。

这就是 Atomicity（原子性）。

### 2. 一致性：不能破坏业务约束

假设业务规定：

```
所有账户余额之和必须保持不变。
```

转账前：

```
1000 + 500 = 1500
```

转账后：

```
900 + 600 = 1500
```

事务需要确保数据库操作不会破坏数据库约束，以及业务逻辑所依赖的不变量。

注意：数据库事务本身并不知道什么是银行转账，也不会自动理解业务规则。一致性最终需要数据库约束、事务隔离和正确的业务代码共同保证。

### 3. 隔离性：并发执行不能产生错误结果

假设两个事务同时读取账户 A 的余额：

```
账户 A = 1000
```

事务 T1：

```
读取 A = 1000
A = 1000 - 100
写入 A = 900
```

事务 T2：

```
读取 A = 1000
A = 1000 - 200
写入 A = 800
```

如果两个事务不加任何并发控制，最终可能是：

```
A = 800
```

而不是应该得到的：

```
A = 700
```

这就是典型的丢失更新问题（Lost Update）。

事务隔离性需要处理的不只是丢失更新，还包括：

- 脏读（Dirty Read）
- 不可重复读（Non-repeatable Read）
- 幻读（Phantom Read）
- 写偏差（Write Skew）

### 4. 持久性：提交成功后不能因为崩溃而丢失

假设数据库已经返回：

```
COMMIT;
```

应用程序认为转账成功。

但如果数据库只是把数据写入了内存缓存，还没有持久化到磁盘，随后服务器断电，就可能丢失已经提交的事务。

持久性（Durability）要求：在系统故障恢复后，已经成功提交的事务仍然有效。

这通常依赖 WAL、Redo Log、刷盘策略以及数据库恢复机制。

## 二、数据库内部究竟需要解决哪几类问题？

从数据库内核的角度，可以把事务拆成三个相互关联的问题：

问题 1

修改如何成为一个整体？

多条 SQL 执行到一半失败，如何撤销已经完成的修改？

问题 2

多个事务如何并发执行？

多个客户端同时读写相同数据，如何避免相互干扰？

问题 3

数据库崩溃后如何恢复？

如何判断哪些事务应该保留，哪些事务应该撤销？

这三个问题分别涉及：

| 问题  | 主要技术              |
| --- | ----------------- |
| 原子性 | Undo、版本可见性、事务提交状态 |
| 隔离性 | MVCC、行锁、范围锁、谓词锁   |
| 持久性 | WAL、Redo、刷盘、崩溃恢复  |
| 一致性 | 约束检查、事务隔离、业务逻辑    |

这里有一个很重要的认识：

事务不是简单地给多条 SQL 加一个 BEGIN 和 COMMIT。它实际上是数据库内核的一套并发控制、版本管理和故障恢复机制。

## 三、MySQL InnoDB 是如何实现事务的？

先看 MySQL。

需要明确：MySQL 有多种存储引擎，下面讨论的是最常用的 InnoDB。MyISAM 等不具备相同的事务能力。

InnoDB 的事务实现主要依赖四个组件：

- Undo Log：撤销日志。
- Redo Log：重做日志。
- MVCC：多版本并发控制。
- Lock：锁机制。

[MySQL的binlog、redo log和undo log](https://images.openai.com/static-rsc-4/qM8zJwGNRziWfJQ7eiVG7MHeIfPrPEZAjZuiIELXZjFDRQVMbfZzDGxpcGev0rAcqc_5OnIRTpyx9WiTmOvff9elRnDY1rvE_Cd-6UbhcFcleVxSI_jE86QJrXOZisJUvnjoo3EzMlsZGQbHAAreb6j3cMGF30Yria7ChgPzfdg?purpose=inline)

[cnblogs.com](https://www.cnblogs.com/coderw/p/16328691.html?utm_source=chatgpt.com)

### 3.1 Undo Log：解决回滚和历史版本读取

假设数据库中有一条记录：

```
id = 1
balance = 1000
```

事务 T1 执行：

```
BEGIN;

UPDATE accounts
SET balance = 900
WHERE id = 1;
```

InnoDB 修改数据时，会同时生成 Undo Log。

可以把它简单理解成：

```
当前数据：
balance = 900

Undo Log：
旧版本 balance = 1000
```

Undo Log 有两个重要作用。

作用一：事务回滚。

如果 T1 执行：

```
ROLLBACK;
```

InnoDB 就可以根据 Undo Log，将数据恢复到修改之前的状态：

```
balance = 1000
```

作用二：支持 MVCC。

假设 T1 尚未提交，另一个事务 T2 执行：

```
SELECT balance
FROM accounts
WHERE id = 1;
```

T2 不应该看到 T1 尚未提交的修改。

InnoDB 可以利用 Undo Log 找到历史版本，让 T2 读取：

```
balance = 1000
```

这就是 MVCC 的一个核心思想：

不一定要阻塞读操作，而是让读操作读取自己应该看到的数据版本。

InnoDB 的记录还包含事务 ID 和回滚指针等信息，借助 Undo Log 可以沿着版本链找到合适的历史版本。

### 3.2 Redo Log：解决数据库崩溃后的数据恢复

Undo Log 解决了回滚和历史版本读取问题，但还有一个问题：

假设事务已经提交，数据库突然断电，数据页还没有写入磁盘怎么办？

这就需要 Redo Log。

假设事务执行：

```
BEGIN;

UPDATE accounts
SET balance = 900
WHERE id = 1;

COMMIT;
```

InnoDB 不要求每次提交都立即将修改过的所有数据页写入数据库文件。

它采用的是 WAL（Write-Ahead Logging，预写日志）思想：

1. 修改数据页，通常先在 Buffer Pool 中完成。
2. 生成对应的 Redo Log。
3. 在提交时，按照持久性配置要求，将 Redo Log 刷到持久存储。
4. 后续再由后台机制将脏数据页写回数据文件。

关键规则是：

对应的数据页修改必须先有日志保护，日志必须先于数据页落盘。

假设数据库崩溃时：

```
Redo Log：已经持久化
数据文件：还没有写入最新数据
```

重启后，InnoDB 就可以根据 Redo Log 重做尚未落盘的修改。

这就是 Redo 的含义。

官方文档明确说明，Redo Log 用于崩溃恢复，重新应用未能完成写入数据文件的修改。

[image](https://www.google.com/s2/favicons?domain=https://dev.mysql.com\&sz=32)

MySQL Developer Zone+1



### 3.3 Undo Log 和 Redo Log 的区别

这是理解 InnoDB 事务最重要的知识点之一。

| 对比     | Undo Log      | Redo Log       |
| ------ | ------------- | -------------- |
| 核心作用   | 撤销修改          | 重做修改           |
| 解决问题   | 回滚、历史版本       | 崩溃恢复           |
| 记录目的   | 恢复旧版本所需的信息    | 恢复已记录的数据页修改    |
| 主要使用场景 | ROLLBACK、MVCC | Crash Recovery |

可以记成：

```
Undo：我要撤销刚才做的事情。

Redo：我要重新执行已经记录的修改。
```

需要注意，Redo 不等于重新执行原始 SQL，而是重放日志记录的底层修改。

### 3.4 MVCC：解决读写并发问题

InnoDB 默认隔离级别是：

```
REPEATABLE READ
```

其 MVCC 主要依赖：

- 隐藏事务 ID（DB_TRX_ID）
- 回滚指针（DB_ROLL_PTR）
- Undo Log 版本链
- Read View（读视图）

假设：

```
初始 balance = 1000
```

T1：

```
BEGIN;

UPDATE accounts
SET balance = 900
WHERE id = 1;
```

此时 T1 尚未提交。

T2：

```
BEGIN;

SELECT balance FROM accounts WHERE id = 1;
```

T2 通过 Read View 判断 T1 的修改不可见，于是沿着 Undo Log 版本链找到旧版本：

```
T2 读取到：1000
```

之后 T1：

```
COMMIT;
```

那么 T2 接下来读取什么，取决于隔离级别和快照的建立时机。

在默认 REPEATABLE READ 下，同一事务中的普通一致性读通常继续使用第一次一致性读建立的 Read View，因此仍然看到：

```
1000
```

而在 READ COMMITTED 下，每次普通一致性读都会创建新的 Read View，因此下一次读取可以看到：

```
900
```

这就是 MySQL 不同隔离级别在 MVCC 上的一个重要区别。

[image](https://www.google.com/s2/favicons?domain=https://dev.mysql.com\&sz=32)

MySQL Developer Zone+1



### 3.5 Lock：解决写写冲突和范围并发问题

MVCC 并不意味着数据库不需要锁。

例如：

```
BEGIN;

UPDATE accounts
SET balance = balance - 100
WHERE id = 1;
```

另一个事务也想修改同一行：

```
UPDATE accounts
SET balance = balance - 200
WHERE id = 1;
```

这时 InnoDB 会使用排他锁（Exclusive Lock），协调对同一数据的写入。

此外，InnoDB 还有：

- Record Lock：记录锁。
- Gap Lock：间隙锁。
- Next-Key Lock：记录锁与间隙锁的组合。

例如：

```
SELECT *
FROM accounts
WHERE id BETWEEN 10 AND 20
FOR UPDATE;
```

在适用的索引范围和隔离级别下，InnoDB 可以通过 Next-Key Lock 限制其他事务向对应范围插入记录，从而处理幻读相关问题。

[image](https://www.google.com/s2/favicons?domain=https://dev.mysql.com\&sz=32)

MySQL Developer Zone+1



## 四、PostgreSQL 是如何实现事务的？

PostgreSQL 的事务实现思路与 InnoDB 有相似之处，但在底层数据版本管理方面有一个很大的区别：

InnoDB 主要通过 Undo Log 保存旧版本；PostgreSQL 则直接在数据表中保留多个 Tuple（行版本）。

PostgreSQL 也使用 MVCC、WAL 和锁，但其实现机制不同。

### 4.1 MVCC：直接保存多个数据版本

假设 PostgreSQL 有这样一条记录：

```
id = 1
balance = 1000
```

事务 T1 执行：

```
BEGIN;

UPDATE accounts
SET balance = 900
WHERE id = 1;
```

PostgreSQL 的 MVCC 机制不是简单地把原来的 1000 覆盖成 900。

它会创建一个新的 Tuple 版本，逻辑上可以理解为：

```
旧版本：
balance = 1000
xmin = 事务 A
xmax = 事务 T1

新版本：
balance = 900
xmin = 事务 T1
xmax = 无
```

这里的两个字段很重要：

- `xmin`：创建当前 Tuple 的事务 ID。
- `xmax`：删除或使当前 Tuple 版本失效的事务 ID。

注意，`xmax` 不一定代表已经物理删除，它也可能表示该版本被某个事务更新或删除。

现在 T1 还没有提交。

T2 执行：

```
SELECT balance
FROM accounts
WHERE id = 1;
```

PostgreSQL 会根据 T2 的 Snapshot（快照）判断 Tuple 是否可见。

因为 T1 尚未提交，所以：

```
旧版本 balance = 1000：可见

新版本 balance = 900：不可见
```

因此 T2 读取到：

```
1000
```

如果 T1 提交，后续创建的快照就可以看到新版本：

```
900
```

PostgreSQL 的 MVCC 核心就是：

通过事务 ID 和 Snapshot 判断数据版本的可见性，而不是让所有读操作都等待写操作完成。

官方文档对 PostgreSQL 的 MVCC 机制有专门介绍。

[image](https://www.google.com/s2/favicons?domain=https://www.postgresql.org\&sz=32)

PostgreSQL+1



### 4.2 PostgreSQL 如何实现 ROLLBACK？

假设：

```
BEGIN;

UPDATE accounts
SET balance = 900
WHERE id = 1;

ROLLBACK;
```

执行 UPDATE 时，PostgreSQL 已经创建了新 Tuple。

但是 T1 最终回滚了。

PostgreSQL 会将 T1 标记为 aborted（已中止）。

于是：

```
旧版本 balance = 1000
新版本 balance = 900
```

对于其他事务：

```
旧版本：仍然可见

新版本：不可见
```

新 Tuple 并不需要立即从数据文件中物理删除。

后续 VACUUM 会清理不再被任何活动事务需要的旧版本或无效版本，回收空间供后续使用。

这也是 PostgreSQL 和 InnoDB 一个非常重要的区别：

| 对比      | InnoDB                    | PostgreSQL     |
| ------- | ------------------------- | -------------- |
| 更新记录    | 通常修改当前记录，并通过 Undo 保存旧版本信息 | 创建新的 Tuple 版本  |
| 旧版本保存位置 | Undo Log                  | 表数据中的旧 Tuple   |
| 回滚      | 根据 Undo 执行撤销              | 使事务产生的新版本变为不可见 |
| 旧版本清理   | Purge                     | VACUUM         |

PostgreSQL 需要通过 VACUUM 清理不再需要的旧 Tuple，否则频繁 UPDATE、DELETE 会造成 Dead Tuples（死元组）积累。

[image](https://www.google.com/s2/favicons?domain=https://www.postgresql.org\&sz=32)

PostgreSQL+1



### 4.3 WAL：解决崩溃恢复和持久性

PostgreSQL 同样采用 WAL（Write-Ahead Logging）。

例如：

```
BEGIN;

UPDATE accounts
SET balance = 900
WHERE id = 1;

COMMIT;
```

PostgreSQL 会记录数据修改对应的 WAL 日志。

其核心规则是：

数据页写入磁盘之前，描述这些修改的 WAL 必须先写入持久存储。

当事务提交时，PostgreSQL 会写入事务提交记录，并按照同步提交配置要求刷新 WAL。

数据页不必在 COMMIT 时全部落盘。

假设：

```
WAL：已经持久化
数据页：尚未全部写入磁盘
```

数据库崩溃之后，PostgreSQL 可以从最近的 Checkpoint 开始，重放 WAL 中的必要记录，恢复数据文件。

这就是 PostgreSQL 的 Crash Recovery。

另外，PostgreSQL 的 WAL 不仅用于崩溃恢复，还可以用于：

- 主从复制。
- 时间点恢复（PITR）。
- 物理备份恢复。
- 持续归档。

[image](https://www.google.com/s2/favicons?domain=https://www.postgresql.org\&sz=32)

PostgreSQL+2

### 4.4 PostgreSQL 的隔离级别

PostgreSQL 也支持四种标准隔离级别：

```
READ UNCOMMITTED
READ COMMITTED
REPEATABLE READ
SERIALIZABLE
```

但有一点值得注意：

PostgreSQL 的 READ UNCOMMITTED 实际表现与 READ COMMITTED 一样，不会产生脏读。

PostgreSQL 默认隔离级别是：

```
READ COMMITTED
```

它与 MySQL InnoDB 默认的 REPEATABLE READ 不同。

下面看它们的区别：

| 隔离级别             | PostgreSQL 行为                         |
| ---------------- | ------------------------------------- |
| READ UNCOMMITTED | 等同 READ COMMITTED                     |
| READ COMMITTED   | 每条 SQL 使用新的 Snapshot                  |
| REPEATABLE READ  | 整个事务使用一致的 Snapshot                    |
| SERIALIZABLE     | 在 Snapshot Isolation 基础上增加 SSI 并发冲突检测 |

特别是 SERIALIZABLE。

PostgreSQL 使用 SSI（Serializable Snapshot Isolation）机制检测事务之间的读写依赖关系。

假设两个事务：

T1：

```
BEGIN ISOLATION LEVEL SERIALIZABLE;

SELECT SUM(balance) FROM accounts;

UPDATE accounts
SET balance = balance - 100
WHERE id = 1;

COMMIT;
```

T2：

```
BEGIN ISOLATION LEVEL SERIALIZABLE;

SELECT SUM(balance) FROM accounts;

UPDATE accounts
SET balance = balance - 200
WHERE id = 2;

COMMIT;
```

这里两个事务修改的是不同的账户，但它们都依赖于账户总余额。

如果两个事务同时执行，可能产生一种叫作 Write Skew（写偏差）的并发异常。

PostgreSQL 的 SSI 会检测相关的读写依赖关系。如果发现并发执行无法等价于某种串行执行，就会中止其中一个事务，要求应用重试。

这也是 PostgreSQL SERIALIZABLE 与简单的 Snapshot Isolation 不同的地方。

[image](https://www.google.com/s2/favicons?domain=https://www.postgresql.org\&sz=32)

PostgreSQL+1



## 五、MySQL 与 PostgreSQL 的核心实现差异

现在把两个数据库放在一起看。

MySQL InnoDB

SQL / Transaction

MVCC + Lock

Read View、行锁、Gap Lock、Next-Key Lock

Undo Log

回滚、历史版本

Redo Log

持久性、崩溃恢复

Buffer Pool / Data Files

PostgreSQL

SQL / Transaction

MVCC + Lock + SSI

Snapshot、Tuple 可见性、行锁、谓词冲突检测

Tuple Versions

旧版本、新版本

WAL

持久性、崩溃恢复

Shared Buffers / Data Files

VACUUM 负责清理失效版本

核心差异可以总结成：

| 对比维度      | MySQL InnoDB        | PostgreSQL     |
| --------- | ------------------- | -------------- |
| MVCC 版本存储 | Undo Log            | Heap Tuple 多版本 |
| 回滚机制      | Undo                | 事务中止、版本不可见     |
| 崩溃恢复      | Redo Log            | WAL            |
| 读一致性      | Read View           | Snapshot       |
| 默认隔离级别    | REPEATABLE READ     | READ COMMITTED |
| 范围并发控制    | Gap / Next-Key Lock | 谓词锁、SSI 等机制    |
| 历史版本清理    | Purge               | VACUUM         |

## 六、一个容易忽略的问题：MVCC 不等于事务隔离

很多人学习数据库时，容易把 MVCC 和事务隔离混为一谈。

它们其实是两个不同层次的东西：

- MVCC 是一种数据版本管理和并发控制技术。
- 事务隔离级别是数据库向用户提供的并发执行语义。

MVCC 可以让读操作读取历史版本，但它本身并不能解决所有并发异常。

例如 PostgreSQL 的 REPEATABLE READ 允许某些 Serialization Anomaly（序列化异常），所以 PostgreSQL 还需要 SERIALIZABLE + SSI 来提供更强的隔离保证。

MySQL InnoDB 也需要结合锁机制，处理 MVCC 单独无法解决的写写冲突和范围冲突。

可以这样理解：

```
事务隔离
    |
    +---- MVCC：读操作应该看到哪个版本？
    |
    +---- Lock：哪些操作需要互相等待？
    |
    +---- SSI / 范围锁：如何处理更复杂的并发依赖？
```

## 七、数据库事务不能解决什么问题？

最后还有一个非常重要的边界。

数据库事务只能保证数据库自身事务范围内的操作，不能自动保证外部系统操作的原子性。

例如：

```
1. 扣减 MySQL 账户余额
2. 调用 Redis 更新缓存
3. 调用第三方支付接口
4. 发送 Kafka 消息
```

即使把数据库操作放在：

```
BEGIN;

UPDATE accounts ...;

COMMIT;
```

也无法保证 Redis、第三方支付、Kafka 与 MySQL 一起提交或回滚。

因为它们属于不同的资源管理器，MySQL 的本地事务无法直接控制它们。

这就引出了分布式事务：

- 2PC / XA：协调多个资源管理器共同提交。
- TCC：Try、Confirm、Cancel。
- Saga：通过一系列本地事务和补偿操作实现最终一致性。
- Transactional Outbox：通过本地事务保证业务数据和待发送消息一起落库。

它们解决的是比数据库本地事务更大的问题：多个独立系统之间如何协调状态变更。

## 八、建议你建立的整体知识框架

如果从数据库内核和分布式系统的角度继续深入，我建议按这个顺序学习：

1. ACID：理解事务到底要保证什么。
2. WAL：理解为什么先写日志，再写数据页。
3. Undo / Redo：理解回滚与崩溃恢复的区别。
4. MVCC：理解数据版本、快照和可见性判断。
5. Lock：理解写写冲突、死锁、范围锁。
6. Isolation Level：理解脏读、不可重复读、幻读、写偏差。
7. 2PC / Saga / Outbox：理解数据库事务如何扩展到分布式系统。

推荐直接阅读两套官方文档：

- [MySQL InnoDB Transaction Model](https://dev.mysql.com/doc/refman/8.4/en/innodb-transaction-model.html?utm_source=chatgpt.com)
- [PostgreSQL MVCC and Concurrency Control](https://www.postgresql.org/docs/current/mvcc.html?utm_source=chatgpt.com)

我认为最值得进一步深入的是 MVCC。 因为一旦理解了 Undo Log、Tuple Version、Read View、Snapshot 和事务可见性，你就真正理解了 MySQL 和 PostgreSQL 并发控制设计的核心差异。

# 问： 数据库的WAL模式启动时，当一条SQL语句从客户端发来时，数据库是先更新WAL的日志文件，后更新缓存，最后写入数据库文件吧？以Badger数据库为例，给我讲解一下。然后再给我详细讲解一下Mysql的Undo Log、Redo Log等日志的写入时机

ChatGPT答：

你的理解**整体方向是对的，但需要修正一个非常重要的细节：WAL 模式下，通常不是先更新 WAL 文件、再更新缓存、最后更新数据库文件，而是先保证日志记录进入 WAL，再允许相应的数据页在合适的时机写回磁盘。**

这里有几个容易混淆的概念：

1. **WAL（Write-Ahead Logging）**：预写日志，保证数据修改在落盘前有可恢复的日志记录。
2. **Cache / Buffer Pool**：内存中的数据缓存，数据库通常先修改内存中的数据页。
3. **Data File**：真正存放数据库数据的文件，数据页可以延迟写入。
4. **Commit（事务提交）**：向客户端承诺事务成功的时间点，它与数据页真正写入数据库文件的时间点并不相同。

我们先以 Badger 为例理解 WAL，再深入 MySQL 的 Undo Log、Redo Log、Binlog，以及它们之间的写入时机。

# **一、先理解 WAL 的核心思想**

WAL 的全称是 Write-Ahead Logging，直译就是「先写日志」。

它遵循一个核心原则：

> 在一个数据页被写入持久化数据文件之前，必须先确保描述该数据页修改的日志已经持久化。

假设数据库中有一条记录：

```
key = user:1001
value = Alice
```

现在客户端执行：

```
UPDATE users SET name = 'Bob' WHERE id = 1001;
```

假设数据库使用 WAL，整个过程可以简化为：

## **WAL 的基本写入流程**

1. **接收数据修改请求**

   客户端发起修改，数据库生成对应的日志记录。
2. **写入 WAL**

   把修改操作记录到 WAL，必要时执行 fsync，确保日志持久化。
3. **修改内存中的数据**

   更新 Buffer Pool 或其他内存数据结构。
4. **事务提交**

   当满足持久化要求后，向客户端返回成功。
5. **异步写入数据文件**

   后台线程将脏页（Dirty Page）刷入真正的数据文件。

这是用于理解 WAL 的通用简化模型，不代表所有数据库都严格采用这一执行顺序。

这里有一个关键点：

**WAL 持久化和数据文件持久化是两个不同的时间点。**

例如：

| 时间 | WAL 文件 | 内存数据  | 数据文件  |
| -- | ------ | ----- | ----- |
| T1 | 无新日志   | Alice | Alice |
| T2 | 写入修改日志 | Alice | Alice |
| T3 | 日志持久化  | Alice | Alice |
| T4 | 日志已持久化 | Bob   | Alice |
| T5 | 日志已持久化 | Bob   | Bob   |

如果在 T4 时机器突然断电，数据库可以利用 WAL 恢复出 Bob。

这就是 WAL 最核心的价值。

# **二、以 Badger 数据库为例**

这里说的 Badger 是 Go 语言实现的嵌入式 KV 数据库 BadgerDB。

它和 MySQL 有一个很大的区别：

- MySQL 是关系型数据库，主要围绕 Buffer Pool、B+Tree、Redo Log、Undo Log 工作。
- Badger 是 KV 数据库，采用 LSM Tree 思想，主要围绕 MemTable、Value Log、SSTable 工作。

因此，Badger 的 WAL 不能简单理解成 MySQL 的 Redo Log。

## **2.1 Badger 的存储结构**

先看一下 Badger 的核心存储组件：

## **Badger 存储架构（简化）**

**客户端**

Set / Delete / Update

**Write Batch / Transaction**

事务处理与写入组织

**Value Log**

磁盘文件

保存 key/value 数据及日志记录

**MemTable**

内存

保存 key 到 Value Pointer 的映射

**SSTable / LSM Tree**

持久化数据文件

后台 Flush、Compaction 生成和整理有序数据文件

概念结构图，省略了索引、Manifest、Value Log GC 等组件。

其中：

**1. Value Log**

Value Log 是 Badger 非常重要的存储组件。

它保存实际的 key/value 数据以及对应的记录信息。写入时采用追加（Append）的方式，避免频繁随机修改磁盘位置。

**2. MemTable**

MemTable 是内存中的有序数据结构。

它保存 Key 与 Value Pointer 之间的映射关系。Value Pointer 指向 Value Log 中实际数据的位置。

**3. SSTable**

MemTable 达到一定大小后，会被 Flush 成 SSTable。

SSTable 是磁盘上的有序数据文件，属于 LSM Tree 的组成部分。

## **2.2 Badger 写入一条数据时发生了什么？**

假设执行：

```
err := db.Update(func(txn *badger.Txn) error {
    return txn.Set([]byte("name"), []byte("Alice"))
})
```

从逻辑上看，可以分成以下几个阶段：

1. **创建事务并准备数据**

   Badger 创建写事务，组织本次写入的 Key、Value 等信息。
2. **向 Value Log 追加记录**

   将记录编码后追加到 Value Log，获得这条记录在日志中的位置。
3. **更新 MemTable**

   将 Key 和对应的 Value Pointer 写入 MemTable，使得后续查询能够找到新值。
4. **事务提交**

   写事务完成提交，向调用者返回结果。具体持久化保证取决于 Badger 的同步配置及提交路径。
5. **后台 Flush**

   MemTable 满足条件后，后台将其转换成 SSTable。

这里特别需要注意：

**Badger 的 Value Log 和 MemTable 并不是简单的「WAL 文件和普通缓存」关系。**

Value Log 本身就是 Badger 的一种实际数据存储形式。对于较大的 Value，MemTable 中通常只保存 Value Pointer，而不是保存整个 Value。

而且，Badger 的写入过程中涉及事务提交、写入批次和同步策略，不能把每次 `Set` 都理解成一次独立的 `fsync`。

## **2.3 Badger 的数据什么时候真正进入 SSTable？**

假设你连续写入：

```
txn.Set([]byte("user:1"), []byte("Alice"))
txn.Set([]byte("user:2"), []byte("Bob"))
txn.Set([]byte("user:3"), []byte("Charlie"))
```

这些数据不需要每次都直接更新 SSTable。

通常是：

```
             写入请求
                 |
                 v
             Value Log
                 |
                 v
              MemTable
                 |
          MemTable 达到阈值
                 |
                 v
          Flush 成 SSTable
                 |
                 v
          LSM Tree 磁盘层级
```

这样做的好处是：

- Value Log 采用顺序追加，降低随机磁盘写入。
- MemTable 负责吸收大量内存写入。
- SSTable 批量生成，减少频繁更新磁盘数据结构的开销。
- Compaction 在后台整理 SSTable。

因此，Badger 的持久化过程并不是每次事务提交都要等 SSTable 写入完成。

## **2.4 Badger 崩溃后如何恢复？**

假设：

1. Value Log 已经写入。
2. MemTable 已经更新。
3. 还没有 Flush 成 SSTable。
4. 此时机器突然断电。

重新启动时，Badger 可以通过 Value Log 中尚未被持久化到 SSTable 的有效记录恢复索引状态。

这就是它利用日志和 LSM Tree 实现崩溃恢复的基本思路。

注意：日志中已经有记录，不等于任何配置下都能保证该记录在突然断电后一定存在。还要考虑操作系统缓存、磁盘写缓存以及 Badger 的同步配置。

# **三、MySQL InnoDB 的日志体系**

接下来讲 MySQL。

MySQL 的 InnoDB 存储引擎中，主要涉及以下几种日志：

| 日志          | 核心作用               | 主要解决的问题   |
| ----------- | ------------------ | --------- |
| Undo Log    | 保存修改前的数据版本或回滚信息    | 事务回滚、MVCC |
| Redo Log    | 记录数据页修改的重做信息       | 崩溃恢复      |
| Binlog      | 记录数据库逻辑变更事件        | 主从复制、数据恢复 |
| Relay Log   | 从库保存接收到的 Binlog 事件 | 主从复制      |
| Doublewrite | 防止数据页部分写入导致损坏      | 数据页写入完整性  |

我们重点讲前三种。

## **InnoDB 三大核心日志**

**Undo Log**

保存旧版本 / 回滚信息

向过去恢复

**Redo Log**

保存数据页修改的重做信息

向前恢复

**Binlog**

保存数据库层面的变更事件

复制与时间点恢复

这三种日志的设计目标完全不同，不能相互替代。

# **四、Undo Log：事务回滚与 MVCC**

Undo Log 是理解 MySQL 事务机制最重要的一种日志。

它主要有两个用途：

1. **事务回滚**：事务执行失败时，可以撤销已经执行的修改。
2. **MVCC（多版本并发控制）**：其他事务读取数据时，可以通过 Undo Log 找到旧版本。

## **4.1 Undo Log 什么时候写入？**

假设数据库中有一条记录：

```
id = 1001
name = Alice
age = 20
```

客户端开启事务：

```
BEGIN;

UPDATE users
SET name = 'Bob', age = 21
WHERE id = 1001;
```

在修改数据之前，InnoDB 需要生成 Undo Log，记录恢复旧数据所需要的信息。

对于这个 UPDATE，可以简化理解为：

**修改之前的数据**

```
{
  "id": 1001,
  "name": "Alice",
  "age": 20
}
```

**Undo Log**

```
name: Bob -> Alice
age:  21  -> 20
```

示意性表达，实际 Undo Record 的格式与字段组织更复杂。

**修改 Buffer Pool 中的数据页**

```
{
  "id": 1001,
  "name": "Bob",
  "age": 21
}
```

这里要强调：

**Undo Log 是在修改记录的过程中生成的，而不是等到 COMMIT 时才生成。**

否则，如果事务在提交之前发生异常，数据库就无法可靠地撤销已经执行的修改。

## **4.2 Undo Log 保存在哪里？**

Undo Log 并不是简单地保存在一个独立的普通日志文件中。

在 InnoDB 中：

- Undo Log Record 位于 Undo Log Segment。
- Undo Log Segment 位于 Rollback Segment。
- Rollback Segment 位于 Undo Tablespace 或全局临时表空间。

对于普通持久化表的事务，Undo Log 还需要通过 Redo Log 保护其自身的崩溃恢复。

也就是说，Undo Log 本身的修改也会产生 Redo Log。

## **4.3 Undo Log 和 MVCC 有什么关系？**

这是一个很有意思的设计。

假设：

- 事务 A 开始时，数据是 Alice。
- 事务 B 将 Alice 修改成 Bob。
- 事务 A 仍然需要读取修改前的数据。

那么事务 A 就不能直接读取 Bob。

InnoDB 可以利用 Undo Log 找到 Alice 对应的旧版本。

可以把它理解成一个版本链：

**当前记录**

# **Bob**

当前版本

roll pointer

**Undo Log 旧版本**

# **Alice**

历史版本

真实的 InnoDB 版本链通过记录中的事务 ID 和 Roll Pointer 等信息关联。

因此，Undo Log 不仅仅用于回滚，它还是 InnoDB 实现 MVCC 的重要基础。

## **4.4 事务提交后 Undo Log 会立即删除吗？**

不会。

假设事务 B 已经提交了：

```
UPDATE users SET name = 'Bob' WHERE id = 1001;
COMMIT;
```

但事务 A 仍然需要读取 Alice。

如果 InnoDB 立即删除 Undo Log，那么事务 A 就无法再读取正确的历史版本了。

所以 InnoDB 会通过 Purge 机制清理不再被任何活跃事务需要的旧版本。

这也解释了为什么：

**长时间运行的事务可能导致 Undo Log 不断积累。**

MySQL 官方文档也明确说明，已经提交的修改和删除操作，其旧版本并不会立即被物理清除。

[image](https://www.google.com/s2/favicons?domain=https://dev.mysql.com\&sz=32)

MySQL+1



# **五、Redo Log：崩溃恢复**

如果说 Undo Log 负责向过去恢复，那么 Redo Log 就负责向前恢复。

Redo Log 的核心作用是：

> 即使数据库突然崩溃，也能够把已经生成但尚未写入数据文件的修改重新应用。

## **5.1 Redo Log 记录什么？**

假设 users 表的一条记录位于某个 InnoDB 数据页中。

执行：

```
UPDATE users
SET age = 21
WHERE id = 1001;
```

InnoDB 在内存中修改 Buffer Pool 中的数据页，同时生成描述这些页修改的 Redo Log。

这里的 Redo Log 不是简单地记录：

```
UPDATE users SET age = 21 WHERE id = 1001;
```

它主要记录的是能够用于重做底层数据页修改的信息。

可以简单理解为：

```
Page ID: 100
Offset:  128
修改内容: 将对应字段修改为 21
```

这只是帮助理解的示意，并非真实 Redo Log Record 的完整格式。

## **5.2 Redo Log 什么时候写入？**

Redo Log 的写入过程可以分成三个阶段：

1. 生成 Redo Log Record。
2. 写入内存中的 Redo Log Buffer。
3. 从 Redo Log Buffer 写入磁盘 Redo Log Files，并根据配置执行 fsync。

## **Redo Log 写入流程**

**Buffer Pool**

数据页被修改，成为 Dirty Page

**Redo Log Buffer**

内存中的 Redo Log 记录

**Redo Log Files**

磁盘中的持久化日志

**Checkpoint**

推进检查点，允许回收不再需要的旧 Redo Log

这里有两个重要概念：

**Redo Log Buffer**

它是内存中的日志缓冲区。生成的 Redo Log Record 会先进入这里，不代表已经持久化。

**Redo Log Files**

它们是磁盘上的日志文件。Redo Log Buffer 中的数据写入文件，也不一定代表已经真正落盘，因为还可能停留在操作系统缓存中。

## **5.3 Redo Log 和数据文件谁先写？**

这正是 WAL 的核心原则。

假设：

```
Buffer Pool 中的数据：

age = 21

磁盘数据文件中：

age = 20
```

此时 Redo Log 已经持久化：

```
Redo Log：

age 从 20 修改为 21
```

那么即使数据库突然崩溃，重新启动时也能通过 Redo Log 恢复数据页。

因此：

**Redo Log 必须先于对应的数据页持久化。**

但是，Redo Log 不要求等到数据页写入磁盘之后才能提交事务。

这就使得 InnoDB 可以在后台异步刷新脏页，而不需要让每个事务都等待数据文件完成随机写入。

MySQL 官方对 Redo Log 的定义也是：它用于在异常关闭后重新应用尚未完成写入数据文件的修改。

[image](https://www.google.com/s2/favicons?domain=https://dev.mysql.com\&sz=32)

MySQL+1



## **5.4 Redo Log 什么时候真正刷盘？**

MySQL 有一个非常重要的参数：

```
innodb_flush_log_at_trx_commit
```

它决定事务提交时 Redo Log 的刷盘策略。

| 参数 | 写入 Redo Log | fsync  | 崩溃时的风险                     |
| -- | ----------- | ------ | -------------------------- |
| 0  | 通常每秒一次      | 通常每秒一次 | 可能丢失最近约 1 秒事务              |
| 1  | 每次提交        | 每次提交   | 正常配置下满足事务持久性要求             |
| 2  | 每次提交        | 通常每秒一次 | MySQL 进程崩溃通常不丢，但 OS 崩溃可能丢失 |

注意这里的区别：

- `write`：将数据写入文件系统，可能还在 OS Page Cache。
- `fsync`：请求操作系统将文件数据同步到持久化存储。

生产环境通常使用：

```
innodb_flush_log_at_trx_commit = 1
```

这也是 MySQL 官方推荐的严格持久性配置。

[image](https://www.google.com/s2/favicons?domain=https://dev.mysql.com\&sz=32)

MySQL+1



# **六、Binlog：MySQL Server 层的逻辑日志**

Redo Log 和 Undo Log 都属于 InnoDB 存储引擎层面的日志。

Binlog 则属于 MySQL Server 层。

它主要解决：

1. 主从复制。
2. 基于时间点的数据恢复（Point-in-Time Recovery）。
3. 记录数据库发生过的逻辑变更事件。

## **6.1 Binlog 记录什么？**

例如执行：

```
UPDATE users
SET age = 21
WHERE id = 1001;
```

Binlog 根据配置的格式记录：

- STATEMENT：记录 SQL 语句。
- ROW：记录行级变更事件。
- MIXED：根据情况选择。

这里需要注意：

Redo Log 和 Binlog 最大的区别是：

| 对比              | Redo Log      | Binlog         |
| --------------- | ------------- | -------------- |
| 所属层             | InnoDB        | MySQL Server   |
| 主要作用            | 崩溃恢复          | 主从复制、时间点恢复     |
| 记录方式            | 物理/页级重做信息     | 逻辑事件           |
| 文件形式            | 固定容量循环使用的日志体系 | 按文件追加，滚动生成     |
| 是否支持非 InnoDB 引擎 | 否             | 可以记录其他受支持引擎的事件 |

## **6.2 Binlog 什么时候写入？**

这个地方与 Undo Log、Redo Log 不一样。

对于普通 InnoDB 事务，事务执行过程中产生的 Binlog 事件通常先缓存起来。

等到事务提交时，再将整个事务的 Binlog 写入 Binlog 文件。

例如：

```
BEGIN;

UPDATE users SET age = 21 WHERE id = 1001;

UPDATE users SET age = 22 WHERE id = 1002;

COMMIT;
```

前面两条 UPDATE 不会在执行每条 SQL 时就立即将完整事务写入 Binlog。

而是在 COMMIT 阶段，将该事务的 Binlog 事件写入。

MySQL 官方对此有明确说明。

[image](https://www.google.com/s2/favicons?domain=https://dev.mysql.com\&sz=32)

MySQL



这里有一个非常关键的问题：

**Redo Log 和 Binlog 必须保持一致。**

例如：

- Redo Log 记录了事务已经提交。
- Binlog 却没有记录这个事务。

那么主从复制时，从库就可能缺少主库已经提交的数据。

反过来也一样，如果 Binlog 有事务，但是 InnoDB 没有提交，那么从库可能执行了一笔主库实际上没有提交的事务。

MySQL 使用了一个非常重要的机制来解决这个问题：**Two-Phase Commit（两阶段提交，2PC）**。

# **七、两阶段提交（2PC）：Redo Log 和 Binlog 如何保证一致？**

这是 MySQL 事务提交机制中非常核心的一部分。

假设执行：

```
BEGIN;

UPDATE accounts
SET balance = balance - 100
WHERE id = 1;

UPDATE accounts
SET balance = balance + 100
WHERE id = 2;

COMMIT;
```

InnoDB 和 MySQL Server 必须确保：

- InnoDB 提交成功，Binlog 也必须有这笔事务。
- 如果 Binlog 已经持久化，InnoDB 必须能够在崩溃恢复时完成这笔事务。

因此 MySQL 使用两阶段提交。

## **7.1 第一阶段：Prepare**

当客户端执行 COMMIT 后：

1. InnoDB 完成事务修改对应的内部处理。
2. InnoDB 将事务标记为 Prepare 状态。
3. 产生对应的 Prepare Redo Log。
4. 按照提交持久性要求，将 Prepare 阶段需要的 Redo Log 持久化。

此时事务还没有真正完成最终提交。

可以理解为：

```
InnoDB：

事务修改已经准备完成
        |
        v
    Prepare
        |
        v
Redo Log 持久化
```

## **7.2 第二阶段：Commit**

接下来 MySQL Server：

1. 将事务对应的 Binlog 从事务缓存写入 Binlog 文件。
2. 根据 `sync_binlog` 配置同步 Binlog。
3. 通知 InnoDB 完成 Commit。
4. InnoDB 将事务提交状态完成。

简化流程：

```
             COMMIT
                |
                v
        InnoDB Prepare
                |
                v
       Prepare Redo 持久化
                |
                v
        写入 Binlog 文件
                |
                v
         Binlog fsync
                |
                v
         InnoDB Commit
                |
                v
         返回客户端成功
```

这里需要注意，MySQL 8.0 还有 Group Commit 机制，多个事务可以批量完成 Binlog Flush、Sync 和 Commit，并不意味着每个事务都独占执行一次完整的磁盘同步流程。

官方文档也介绍了这种 Binlog 与 InnoDB 两阶段提交机制。

[image](https://www.google.com/s2/favicons?domain=https://dev.mysql.com\&sz=32)

MySQL+1



## **7.3 如果在两阶段提交中间崩溃，会发生什么？**

这是理解 2PC 最好的方式。

假设：

**情况 A：Prepare 阶段完成，Binlog 还没写入就崩溃**

事务不会提交

```
Redo Log：Prepare 已持久化
Binlog：  没有对应事务
```

恢复时，发现事务处于 Prepare 状态，但 Binlog 中没有对应的有效事务提交记录，因此回滚该事务。

**情况 B：Binlog 已经持久化，但 InnoDB 尚未完成 Commit 就崩溃**

事务恢复为提交状态

```
Redo Log：Prepare 已持久化
Binlog：  已持久化
InnoDB：  还处于 Prepare
```

恢复时，通过 Binlog 中的事务 XID 判断事务已经进入有效提交流程，通知 InnoDB 完成提交。

**情况 C：InnoDB 已经完成 Commit**

事务已提交

两份日志都已经完成对应的提交流程，事务正常恢复。

注意，以上假设采用正常的严格持久化配置，例如：

```
innodb_flush_log_at_trx_commit = 1
sync_binlog = 1
```

否则，突然断电时，日志可能丢失，持久性保证也会降低。

# **八、把所有日志放在一起：一条 UPDATE 的完整生命周期**

现在我们把前面的知识串起来。

假设：

```
BEGIN;

UPDATE users
SET name = 'Bob'
WHERE id = 1001;

COMMIT;
```

从执行 UPDATE 到事务提交，可以这样理解。

## **MySQL InnoDB 写事务完整流程**

1. **客户端发送 UPDATE**

   MySQL Server 接收 SQL，完成解析、优化和执行计划生成。
2. **读取数据页**

   InnoDB 从 Buffer Pool 获取数据页。如果不在内存中，则从磁盘读取。
3. **生成 Undo Log**

   保存修改前的数据版本，供事务回滚和 MVCC 使用。
4. **生成 Redo Log**

   记录数据页修改及相关内部修改所需的重做信息，先进入 Redo Log Buffer。
5. **修改 Buffer Pool**

   修改内存中的数据页，形成 Dirty Page。

   实际执行时，Undo、Redo 的生成和页修改通过 InnoDB 内部的 Mini-Transaction 等机制协调，不应把它们理解为三个完全独立的串行操作。
6. **记录 Binlog 事件**

   UPDATE 对应的 Binlog 事件暂存在事务缓存中，尚未完成最终提交。
7. **客户端发送 COMMIT**

   InnoDB 进入 Prepare 阶段，准备提交事务，并持久化所需的 Redo Log。
8. **写入并同步 Binlog**

   将事务 Binlog 写入文件，根据配置执行同步。
9. **InnoDB Commit**

   完成事务提交，释放相关事务资源和锁。
10. **返回客户端成功**

    事务提交完成。
11. **后台刷新 Dirty Page**

    Buffer Pool 中的脏页在合适的时机写入真正的 InnoDB 数据文件。

这里有一个很容易被忽略的地方：

**事务提交时，数据页不一定已经写入磁盘。**

只要对应的 Redo Log 已经满足持久化要求，InnoDB 就可以安全地向客户端返回提交成功。

之后如果数据库突然崩溃，可以通过 Redo Log 重做已经提交的修改，并根据事务状态回滚未提交的修改。

# **九、Undo Log、Redo Log、Binlog 的写入时机对比**

这张表建议重点记忆。

| 对比项      | Undo Log         | Redo Log          | Binlog       |
| -------- | ---------------- | ----------------- | ------------ |
| 所属层      | InnoDB           | InnoDB            | MySQL Server |
| 什么时候生成   | 修改数据过程中          | 修改数据过程中           | 事务执行过程中产生事件  |
| 什么时候写入   | 写入 Undo Segment  | 先进入 Log Buffer    | 先进入事务缓存      |
| 什么时候持久化  | 通过 Redo 保护其持久化修改 | 提交阶段按配置刷盘         | 提交阶段按配置同步    |
| 主要作用     | 回滚、MVCC          | 崩溃恢复              | 复制、时间点恢复     |
| 提交后立即删除吗 | 不会               | 由 Checkpoint 控制回收 | 按日志保留策略清理    |

另外还有两个非常重要的区别：

- Undo Log 主要记录如何撤销修改。
- Redo Log 主要记录如何重新应用修改。

可以用一句话概括：

**Undo Log 负责回滚，Redo Log 负责恢复，Binlog 负责记录数据库变更历史。**

# **十、再深入一点：为什么 MySQL 需要 Redo Log 和 Binlog 两种日志？**

你可能会产生一个疑问：

既然 Redo Log 已经可以恢复数据库了，为什么 MySQL 还需要 Binlog？

原因是两者解决的问题不同。

假设 MySQL 主库发生故障：

- Redo Log 可以帮助主库恢复崩溃前已经提交的数据。
- Binlog 可以让从库重新执行主库的事务。
- Binlog 还可以配合备份进行时间点恢复。

如果只有 Redo Log，那么 MySQL 的主从复制和基于逻辑事件的恢复机制就无法直接依赖它实现。

反过来，如果只有 Binlog，也无法直接替代 InnoDB 的 Redo Log 来完成存储引擎的数据页崩溃恢复。

这就是两种日志同时存在的原因。

# **十一、最后做一个总结：Badger 与 MySQL 的区别**

回到你最初的问题。

Badger 和 MySQL 都使用了日志先行的思想，但是它们的存储架构不同。

| 对比项             | Badger                    | MySQL InnoDB                 |
| --------------- | ------------------------- | ---------------------------- |
| 数据结构            | LSM Tree                  | B+Tree                       |
| 内存数据结构          | MemTable                  | Buffer Pool                  |
| 日志机制            | WAL / Value Log           | Undo Log / Redo Log / Binlog |
| 日志主要用途          | 恢复 MemTable 状态、保存实际 Value | 回滚、崩溃恢复、复制                   |
| 数据文件            | SSTable                   | InnoDB Tablespace            |
| 后台持久化           | Flush、Compaction          | Dirty Page Flush、Checkpoint  |
| 提交时是否必须写入最终数据文件 | 不需要                       | 不需要                          |

特别注意，Badger 的 WAL 和 MySQL 的 Redo Log 不能直接画等号：

- Badger 的 MemTable WAL 记录可以在启动时重放，恢复 MemTable。
- Badger 的 Value Log 还承担了实际 Value 存储的职责。
- MySQL Redo Log 记录的是 InnoDB 数据页修改所需的重做信息。
- MySQL Undo Log 则负责事务回滚与历史版本读取。

**如果你想真正掌握这套机制，我建议按照下面的顺序继续研究：**

1. **LSM Tree 与 B+Tree**：理解 Badger 和 InnoDB 为什么采用不同的数据写入架构。
2. **WAL 与 Checkpoint**：理解为什么日志持久化后，数据页仍然可以延迟写入。
3. **MVCC 与 Undo Log**：理解 MySQL 如何在并发事务中实现一致性读。
4. **Redo Log 与 Crash Recovery**：理解 MySQL 宕机后如何恢复数据。
5. **Binlog 与 2PC**：理解 MySQL 如何保证存储引擎和复制日志的一致性。

官方资料也值得直接阅读：

- [MySQL InnoDB Redo Log](https://dev.mysql.com/doc/refman/8.4/en/innodb-redo-log.html?utm_source=chatgpt.com)
- [MySQL InnoDB Undo Logs](https://dev.mysql.com/doc/refman/8.4/en/innodb-undo-logs.html?utm_source=chatgpt.com)
- [MySQL Binary Log](https://dev.mysql.com/doc/refman/8.4/en/binary-log.html?utm_source=chatgpt.com)
- [Badger MemTable 源码](https://github.com/dgraph-io/badger/blob/main/memtable.go?utm_source=chatgpt.com)
- [Badger DB 写入流程源码](https://github.com/dgraph-io/badger/blob/main/db.go?utm_source=chatgpt.com)

其中，Badger 的 `db.go`、`memtable.go` 和 `value.go` 很值得结合阅读，能够直接看到 WAL 写入、MemTable 更新以及 Value Log 写入的实际代码。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+2
