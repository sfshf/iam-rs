# [spiceDB](https://github.com/authzed/spicedb): authzrization Database

- [zanzibar](https://authzed.com/zanzibar)

# mysql

- [Mysql 字符编码](https://mathiasbynens.be/notes/mysql-utf8mb4)
- [连接配置参数](https://github.com/go-sql-driver/mysql#parameters)
- [Mysql Read Write Splitting](https://blogs.oracle.com/mysql/post/mysql-82-transparent-readwrite-splitting)


# 问：有没有rust编写的新的关系型数据库？

ChatGPT 答：

有，而且近几年 Rust 生态中出现了不少值得关注的关系型数据库项目。不过需要区分两类：

1. 真正的关系型数据库（RDBMS）：支持 SQL、表、行、列、事务、索引、约束等。
2. 嵌入式 SQL 数据库或数据库内核：可以在应用程序内部运行，不一定提供独立的数据库服务端。

如果你希望寻找类似 PostgreSQL、MySQL 的 Rust 原生实现，重点可以关注 GlareDB、RisingWave、ReadySet 等项目；如果你对数据库内核和存储引擎感兴趣，还有一些更值得研究的项目。

## 一、值得关注的 Rust 关系型数据库

### 1. Turso —— 最值得关注的项目

[GitHub AI项目展示](https://images.openai.com/static-rsc-4/TYoUuYM5BGIjp2aXkPCHUrRTCMh9LCrCvchEGbPo43Mn6J53Hzqn9f6-YDk0HLgGPvdeXAtsh6zdrPthy3HHN2CZTb0oZVdEb4ik99GPLzUyOS5lEKl9tBuMfTr_GfKiJdqTy790TFxOuiXtIJ_uH9kMdq4DWBBA9Ourb14Ejx8?purpose=inline)

GitHub： [tursodatabase/turso](https://github.com/tursodatabase/turso?utm_source=chatgpt.com)

活跃开发

纯 Rust 实现 · 嵌入式 SQL 数据库

Turso 是从零开始用 Rust 重写的 SQLite 兼容数据库，不是 SQLite 的 Rust 封装。

主要特点：

- 兼容 SQLite SQL 方言和文件格式。
- 支持 MVCC，并通过 `BEGIN CONCURRENT` 改善并发写入能力。
- 支持异步 I/O。
- 支持 Linux、macOS、Windows 和 WebAssembly。
- 正在探索 PostgreSQL 协议和方言兼容。

它有一个比较有意思的架构设计：将 SQL 编译成字节码，再交给自己的虚拟机执行。未来可以通过不同的 SQL 前端复用同一个数据库执行内核。

需要注意：截至目前仍未达到 1.0，PostgreSQL 兼容等功能也有实验性标记。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



适合研究： SQL Parser、虚拟机执行引擎、MVCC、WAL、存储引擎。

### 2. NusaDB —— 比较接近传统 RDBMS

GitHub： [nusadb/nusadb](https://github.com/nusadb/nusadb?utm_source=chatgpt.com)

Pre-1.0

纯 Rust · 单机关系型数据库

这个项目很值得数据库内核爱好者关注。

它并不是简单的 KV 数据库，而是从头实现了关系型数据库的多个核心组件：

| 模块     | 实现                              |
| ------ | ------------------------------- |
| 存储引擎   | B-link / B+ Tree                |
| 事务     | MVCC                            |
| 隔离级别   | 四种标准隔离级别                        |
| 并发控制   | Serializable Snapshot Isolation |
| 持久化    | WAL + Crash Recovery            |
| 查询优化   | Cost-based Optimizer            |
| 执行引擎   | Vectorized Executor             |
| SQL 解析 | sqlparser-rs                    |

它支持事务、Savepoint、Rollback、Hash Join、Sort Spill 等能力。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



我认为它尤其适合研究传统关系型数据库的内部实现。

### 3. RisingWave —— 分布式流式关系型数据库

[RisingWave Logo PNG Vector (SVG) Free Download](https://images.openai.com/static-rsc-4/anYYgs5YRK9Kur9yoJa3DZRH5Y7kPtsrJVYz63KrTV4k8QMw95BvBR76CROlrleZHtB9C5fRtaFY1h6pulB-H87kwHCddQjmAcmUf3vGuKJSM7YV6ya-m4whSG6f3SJZjpG3o_NyECAXvl01NCja9SpICPwFPko9oOImWi-xHiI?purpose=inline)

GitHub： [risingwavelabs/risingwave](https://github.com/risingwavelabs/risingwave?utm_source=chatgpt.com)

成熟度较高

纯 Rust · 分布式 · Streaming SQL

RisingWave 和传统 PostgreSQL、MySQL 有明显区别。

它的核心目标是处理持续不断进入系统的数据流。

例如：

```
CREATE MATERIALIZED VIEW order_stats AS
SELECT
    user_id,
    COUNT(*) AS order_count,
    SUM(amount) AS total_amount
FROM orders
GROUP BY user_id;
```

当新的订单数据进入时，它会增量更新物化视图，而不是每次重新计算整个 SQL 查询。

主要技术特点：

- PostgreSQL Wire Protocol 兼容。
- 分布式计算架构。
- 流式 SQL 执行。
- 增量计算。
- MVCC。
- 基于 LSM Tree 的 Hummock 存储引擎。
- 支持 Kafka、CDC 等数据源。

它更适合实时数据处理、流式计算和实时分析，而不是直接替代传统 OLTP 数据库。

[image](https://www.google.com/s2/favicons?domain=https://risingwave.com\&sz=32)

RisingWave+2



### 4. Databend —— 分析型数据库

[Databend Cloud | 分析·搜索·AI 一体化数仓](https://images.openai.com/static-rsc-4/XgzCWWahnPB5PwRtIqZI-ojVnx-f9ZZIIA-PTc_OS56Y4e9ZJlb2B9XGNGAq01JhPtcj-hY9JAXRpM3sh72VtzaK5WLNEpRRhEpxYNaKf4sQJz9JceshYVv9LSCreZ0ePX2069qXsqEv-_14-FMywKBl8STrZHVDecqa8EXxd9w?purpose=inline)

GitHub： [databendlabs/databend](https://github.com/databendlabs/databend?utm_source=chatgpt.com)

纯 Rust · OLAP · 云原生数据仓库

Databend 是 Rust 实现的云原生数据仓库，主要面向大规模分析查询。

特点：

- SQL 查询引擎。
- 列式分析处理。
- 对象存储作为主要存储基础。
- 向量搜索、全文检索。
- 面向 BI 和数据分析。

它不是传统的 OLTP 数据库，不能简单地拿来替代 MySQL。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



## 二、还有一些值得研究的项目

| 项目       | 类型       | 主要用途          |
| -------- | -------- | ------------- |
| ToyDB    | 分布式 SQL  | 学习数据库内核和 Raft |
| OmenDB   | 单机关系型    | 新型 RDBMS 架构探索 |
| KiteSQL  | 嵌入式 SQL  | Rust 原生数据 API |
| RagnorDB | 分布式 OLTP | MVCC、Raft、事务  |

项目地址：

- [ToyDB](https://github.com/erikgrinaker/toydb?utm_source=chatgpt.com)：教育性质很强，适合学习分布式数据库。

- [OmenDB](https://github.com/omendb/omendb?utm_source=chatgpt.com)：目前是 Developer Preview，尚处于早期阶段。

  [image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

  GitHub

- [KiteSQL](https://github.com/KipData/KiteSQL?utm_source=chatgpt.com)：嵌入式关系型数据库。

  [image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

  GitHub

- [RagnorDB](https://github.com/Aditya-1304/ragnordb?utm_source=chatgpt.com)：尝试实现分布式事务型 SQL 数据库。

  [image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

  GitHub



## 三、如果从数据库内核学习的角度，我会这样选择

结合数据库内部实现的学习价值，我会给出下面的推荐：

| 研究方向       | 推荐项目       | 重点研究内容              |
| ---------- | ---------- | ------------------- |
| 嵌入式 SQL 引擎 | Turso      | SQL 虚拟机、MVCC        |
| 传统 RDBMS   | NusaDB     | B+ Tree、WAL、事务、查询优化 |
| 分布式数据库     | ToyDB      | Raft、分布式事务          |
| 流式数据库      | RisingWave | 增量计算、LSM、分布式执行      |
| OLAP 数据库   | Databend   | 列存、向量化执行、对象存储       |

我个人最建议你先研究 Turso 和 NusaDB。

原因是这两个项目能够帮助你理解两个不同的数据库设计方向：

- Turso：如何从零构建一个现代化、嵌入式、SQLite 兼容的 SQL 数据库。
- NusaDB：如何从零构建一个传统意义上的关系型数据库，包括事务、索引、WAL 和查询执行。

## 四、我想确认一下你寻找这类数据库的目的

1\. 你更关注哪一类关系型数据库？

OLTP：类似 PostgreSQL / MySQL，处理业务事务

OLAP：类似 ClickHouse / Databend，处理分析查询

分布式 SQL：类似 CockroachDB / TiDB

嵌入式 SQL：类似 SQLite

2\. 你主要想做什么？

研究源码，学习数据库内核

寻找可以在生产环境使用的数据库

参考架构，自己用 Rust 实现一个数据库

提交

这两个维度会显著影响推荐结果，尤其是 OLTP 和 OLAP 的存储引擎、事务模型及查询执行器设计差异很大。