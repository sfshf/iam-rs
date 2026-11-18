# 问：有没有rust实现的类似badger的kv数据库？

ChatGPT答：

有，而且 Rust 生态里已经有几个值得关注的嵌入式 KV 数据库。

如果你说的 Badger 是 Go 语言的 BadgerDB（Dgraph 开发），那么我最推荐你先研究 Fjall 和 SurrealKV。

它们与 Badger 的设计方向比较接近：都是嵌入式 KV 存储引擎，采用 LSM Tree 思路，而不是简单地把数据放在内存 HashMap 中，再定期写入文件。

不过需要区分：你想找的是纯 Rust 实现，还是只要 Rust 能调用即可。下面主要介绍纯 Rust 实现。

## 1. 最值得关注的项目

### A. Fjall

最推荐 · LSM Tree

- GitHub：[fjall-rs/fjall](https://github.com/fjall-rs/fjall?utm_source=chatgpt.com)
- 文档：[Fjall API](https://docs.rs/fjall/latest/fjall/?utm_source=chatgpt.com)
- Crates.io：[fjall](https://crates.io/crates/fjall?utm_source=chatgpt.com)

Fjall 的设计与 RocksDB、Badger 比较接近：

- LSM Tree 存储结构。
- 支持 Keyspace，类似 RocksDB Column Family。
- 支持范围查询、前缀查询、正反向迭代。
- 支持 LZ4 压缩。
- 支持跨 Keyspace 原子写入。
- 可选 Serializable Transactions。
- 可选 Key-Value Separation，适合大 Value 场景。
- 自动后台维护和 Compaction。

它还有一个值得注意的设计：底层 LSM Tree 被拆成独立的 `lsm-tree` crate。

如果你希望研究 Rust 实现的 LSM Tree、WAL、MemTable、SSTable、Compaction 等机制，Fjall 是很好的入口。

[image](https://www.google.com/s2/favicons?domain=https://docs.rs\&sz=32)

Rust+1



### B. SurrealKV

值得关注 · 原生异步

\- GitHub：[surrealdb/surrealkv](https://github.com/surrealdb/surrealkv?utm_source=chatgpt.com)

这是一个比较有意思的项目。

它的定位是：

- 嵌入式 KV 数据库。
- LSM Tree 架构。
- ACID 事务。
- 多版本数据管理。
- Async-native、Non-blocking。
- 面向高吞吐服务端、嵌入式系统等场景。

与 Fjall 相比，它特别值得关注的是异步设计。

如果你希望在 Tokio 异步服务中直接使用 KV 存储，而不希望大量同步磁盘操作阻塞异步执行器，可以研究它。

不过需要注意：项目自身公布的性能测试不能直接作为客观排名，尤其需要结合自己的数据规模、读写比例、持久化设置和硬件进行测试。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub



### C. redb

纯 Rust · B+ Tree

- GitHub：[cberner/redb](https://github.com/cberner/redb?utm_source=chatgpt.com)
- 文档：[redb API](https://docs.rs/redb/latest/redb/?utm_source=chatgpt.com)

redb 与 Badger 的底层设计不同。

它采用 Copy-on-Write B+ Tree，而不是 LSM Tree。

主要特点：

- 纯 Rust 实现。
- ACID 事务。
- MVCC。
- 零拷贝读取。
- 单文件存储。
- 读事务不会阻塞写事务。

如果你需要一个可靠的嵌入式 KV 数据库，且读写模式更适合 B+ Tree，那么 redb 值得考虑。

但如果你想研究 Badger 的 LSM Tree 和 Value Log 机制，redb 就不是最接近的选择。

[image](https://www.google.com/s2/favicons?domain=https://docs.rs\&sz=32)

Docs.rs



### D. sled

- GitHub：[spacejam/sled](https://github.com/spacejam/sled?utm_source=chatgpt.com)
- 文档：[sled API](https://docs.rs/sled/latest/sled/?utm_source=chatgpt.com)

sled 是 Rust 生态中比较早期的嵌入式数据库项目。

主要特性：

- 日志结构存储。
- 多 Key 事务。
- 多 Tree 事务。
- CAS。
- 零拷贝读取。
- 前缀订阅和 Watch。
- 压缩。

它的 API 比较简洁，也有一些值得学习的并发设计。

不过，如果是新项目，我会优先考察 Fjall 和 SurrealKV，而不是直接选择 sled。

[image](https://www.google.com/s2/favicons?domain=https://sled.rs\&sz=32)

sled-rs.github.io+1



## 2. 它们与 Badger 的架构对比

这里需要先明确 Badger 的一个重要特点：

Badger 不仅仅是一个 LSM Tree 数据库，它采用了 WiscKey 思路，将 Key 和 Value 分离：

- LSM Tree 主要存储 Key 和 Value 的位置索引。
- Value Log 负责存储实际 Value。
- 这样可以减少 LSM Tree Compaction 时反复搬运大 Value 的成本。

它还支持 MVCC、事务和快照。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



| 项目        | 存储结构                  | 事务              | 异步特性         | 纯 Rust |
| --------- | --------------------- | --------------- | ------------ | ------ |
| Badger    | LSM + Value Log       | ACID、SSI        | Go API       | 否      |
| Fjall     | LSM                   | 可选 Serializable | 以同步 API 为主   | 是      |
| SurrealKV | LSM                   | ACID            | Async-native | 是      |
| redb      | Copy-on-Write B+ Tree | ACID            | 以同步 API 为主   | 是      |
| sled      | 日志结构存储                | 多 Key 事务        | 以同步 API 为主   | 是      |

## 3. 如果你想找的是 Badger 的 Rust 替代品，我会这样选择

Fjall

首选

想要 LSM Tree、Keyspace、Compaction、事务，且希望自己掌控异步调度。

SurrealKV

想要原生异步、版本化 KV 和事务能力，优先研究它。

redb

想要简单、可靠的嵌入式 ACID KV 数据库，不要求 LSM Tree。

## 4. 我想进一步确认你的使用场景

这会影响具体的推荐，因为 Badger 的 Value Log 分离机制，在大 Value、高写入量场景下很有价值。

① 主要的数据访问模式？

高频写入（类似日志、事件存储）

高频读取（类似缓存、元数据查询）

读写混合

需要大量范围查询、前缀查询

② 希望采用什么异步模型？

Tokio async/await

同步 API 即可

都可以，性能优先

③ Value 的大小大概是多少？

小于 1 KB

1 KB ～ 64 KB

64 KB ～ 1 MB

大于 1 MB

④ 是否要求 ACID 多 Key 事务？

需要，支持原子提交和回滚

不需要，单 Key 原子性即可

提交使用场景

根据这些条件，可以进一步判断你应该选择 Fjall、SurrealKV，还是寻找一个更接近 Badger 的 WiscKey / Value Log 架构实现。