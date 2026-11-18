# 问：LSM Tree 与 Copy-on-Write B+ Tree，这两个数据结构给我详解讲解一下，并给我资源地址

ChatGPT答：

LSM Tree 和 Copy-on-Write B+ Tree 是两种非常重要的数据库存储结构。它们解决的是同一个问题：如何将大量 Key-Value 数据高效地组织起来，并在磁盘上实现查询、插入、更新和删除。

但两者的设计哲学几乎相反：

- LSM Tree：先写入，再逐步整理。 尽量把写操作转换成顺序写，后台再合并数据。
- Copy-on-Write B+ Tree：修改时复制路径，构造新版本。 通过不覆盖旧页面来维护数据版本和事务快照。

可以先记住这个直观区别：

[初识Pebble - debuger](https://images.openai.com/static-rsc-4/JRXOa-L_ckfswL6Q6ba3fWYOmDHrZFrztaoz8py8iTfWaGkQ89Kq_rozLHsqaAum416u3HcAWXoSrUcN7lSUP7Dc8FWUt5Fakvo5uoww4wGd6bGnX3UnU2N2IJcD2bRfz__u7_WdBG5KpvWT73ZcUTCVolXvnTEmalK42OUVpGw?purpose=inline)

LSM Tree

内存写入缓冲 + 磁盘有序文件 + 后台 Compaction

[LMDB freelist · erigontech/erigon Wiki · GitHub](https://images.openai.com/static-rsc-4/HWltzDjg0RpIAbhvmMXG8pT88z6fNIRHPnysV4uIcSKxTtKx-cb1EAsU6N58d9Y5ExD4VYFid2LTW9RUGTfCtAfRIC9VrZhAOUDpMQy7MNiRpSPQ_NmgM3mJjDnRziCEhMtR-kw-WjHubTN6pGCgq41Q3BfT7i2I24mLuwtZn2A?purpose=inline)

Copy-on-Write B+ Tree

磁盘树页面 + 路径复制 + 新旧 Root 版本

为了理解它们，我们从磁盘 I/O、数据组织、读写流程、并发事务和崩溃恢复几个层次展开。

# 一、LSM Tree：面向写入优化的数据结构

## 1. 为什么需要 LSM Tree？

先看传统 B+ Tree 在磁盘上的一个问题。

假设数据库有 1000 万条记录，所有记录按照 Key 排序，组织成一棵 B+ Tree。

现在随机插入一个 Key：

```
Put("user:92831", "Alice")
```

数据库需要：

1. 从 Root 开始查找目标 Key 所在的 Leaf Page。
2. 将对应的磁盘 Page 读入内存（如果缓存没有命中）。
3. 找到插入位置。
4. 修改 Leaf Page。
5. 如果 Page 满了，可能需要分裂节点。
6. 将修改后的 Page 写回磁盘。

问题在于：如果连续插入的 Key 都是随机的，那么这些操作可能涉及大量不同的磁盘页面。

这会产生大量随机 I/O。

对于 HDD，随机 I/O 尤其昂贵；对于 SSD，虽然没有机械寻道，但随机写入、写放大、擦写管理和 I/O 并发仍然存在成本。

LSM Tree 的核心思想是：

不要每次插入数据时，都立即修改磁盘上对应位置的索引结构，而是先把写入缓冲在内存中，积累到一定程度后，再批量写入磁盘。

它把大量随机的小写入，转换为批量的顺序写入。

## 2. LSM Tree 的整体结构

一个典型的 LSM Tree 可以分为以下几个部分：

[Delving Into Storage Structures: LSM-Tree vs. B+ Tree | by yang guang | Medium](https://images.openai.com/static-rsc-4/zzFjGljKRTAjEYuaiVowx--gG9pqKL8kL5lEC7-QLtxZoth0P1ubsGMruqfNfZUlEJFIZVup0tSF7pwiaDNzeKQm4VwLbF6oBRNXWlMey1zxhuqYrcfVhgJmpjSdAy06wCYWHDe-Ao1atV9dXEt_S7fE6RYsJML-RuaDkCoQqJw?purpose=inline)

[medium.com](https://medium.com/%40qq549631282/delving-into-storage-structures-lsm-tree-vs-b-tree-54f6a4e5111d?utm_source=chatgpt.com)

| 组件                 | 存储位置 | 作用               |
| ------------------ | ---- | ---------------- |
| WAL                | 磁盘   | 保证内存中的写入可以恢复     |
| MemTable           | 内存   | 接收新写入，通常是有序结构    |
| Immutable MemTable | 内存   | 等待刷盘的只读 MemTable |
| SSTable            | 磁盘   | 持久化的有序数据文件       |
| Compaction         | 后台任务 | 合并 SSTable，整理数据  |

其中最核心的是 MemTable 和 SSTable。

### 2.1 MemTable：内存中的有序数据结构

MemTable 是 LSM Tree 接收写入的第一站。

例如连续写入：

```
Put("k3", "v3")
Put("k1", "v1")
Put("k5", "v5")
Put("k2", "v2")
Put("k4", "v4")
```

MemTable 内部可能使用 Skip List（跳表）、平衡树等有序数据结构。

逻辑上，它最终可以呈现为：

```
MemTable

k1 -> v1
k2 -> v2
k3 -> v3
k4 -> v4
k5 -> v5
```

这里有一个关键点：

虽然写入顺序是乱序的，但 MemTable 可以始终按照 Key 排序。

RocksDB 默认就使用 Skip List 作为 MemTable 的实现之一。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



### 2.2 WAL：为什么还需要日志？

如果数据先写到内存的 MemTable，那么数据库崩溃后，这些数据不就丢失了吗？

所以还需要 WAL。

典型写入流程：

```
客户端
  |
  | Put(k, v)
  v
WAL：追加写入日志
  |
  v
MemTable：插入数据
  |
  v
返回写入成功
```

WAL 是 Append-Only Log，也就是追加写日志。

例如：

```
WAL.log

PUT k3 v3
PUT k1 v1
PUT k5 v5
PUT k2 v2
```

如果数据库崩溃，重新启动时可以重新读取 WAL，将数据恢复到 MemTable。

注意：具体何时向客户端确认写入成功，取决于数据库的同步策略。如果要求强持久性，必须保证相应 WAL 已经持久化，而不能只写入操作系统缓存。

### 2.3 Immutable MemTable：为什么需要不可变的 MemTable？

假设 MemTable 已经达到 64 MB 的容量限制。

这时候需要将它写入磁盘。

但如果刷盘过程中继续停止接受写入，就会影响吞吐量。

因此，通常采用这样的机制：

```
Active MemTable
       |
       | 达到容量阈值
       v
Immutable MemTable
       |
       | 后台线程 Flush
       v
     SSTable

与此同时：

新的 Active MemTable
       |
       | 继续接受写入
       v
```

也就是说：

- 旧 MemTable 转换成只读状态。
- 新建一个 MemTable 接收新的写入。
- 后台线程负责将旧 MemTable 转换成 SSTable。

这种设计可以让前台写入和后台刷盘并行进行。

### 2.4 SSTable：磁盘上的有序数据文件

SSTable 全称是 Sorted String Table。

它是一个按照 Key 排序的不可变文件。

例如：

```
SSTable-001

k1 -> v1
k2 -> v2
k3 -> v3
k4 -> v4
k5 -> v5
```

SSTable 一旦生成，通常不会再直接修改其中的数据。

如果需要修改：

```
Put("k3", "new_value")
```

不会直接找到 SSTable 中的 k3 并覆盖原来的 v3。

而是向 MemTable 插入一个新版本：

```
MemTable

k3 -> new_value
```

于是数据库可能同时存在：

```
MemTable:
k3 -> new_value

SSTable:
k3 -> v3
```

这时候，数据库必须知道哪个版本更新。

通常可以通过内部序列号、版本信息和数据所在层级来确定。

这也是为什么 LSM Tree 需要 Compaction。

## 3. Compaction：LSM Tree 最核心的后台整理机制

Compaction 可以说是 LSM Tree 最重要、也最复杂的部分。

假设经过多次写入和刷盘，磁盘上已经存在：

```
L0:

SST-001:
k1 -> v1
k3 -> v3
k5 -> v5

SST-002:
k2 -> v2
k3 -> v3_new
k6 -> v6
```

注意：

- L0 中不同 SSTable 的 Key 范围可能重叠。
- 同一个 Key 可能存在于多个 SSTable。
- 不同 SSTable 中的同一个 Key 可能对应不同版本。

随着 SSTable 越来越多，读取一个 Key 就可能需要检查多个文件。

因此需要 Compaction。

假设将上述两个 SSTable 合并：

```
输入：

SST-001:
k1 -> v1
k3 -> v3
k5 -> v5

SST-002:
k2 -> v2
k3 -> v3_new
k6 -> v6
```

合并时：

1. 对两个 SSTable 做有序归并。
2. 比较相同 Key 的版本。
3. 保留最新有效版本。
4. 删除已经过期的旧版本。
5. 生成新的 SSTable。

最终：

```
SST-003:

k1 -> v1
k2 -> v2
k3 -> v3_new
k5 -> v5
k6 -> v6
```

这与归并排序（Merge Sort）的核心过程非常相似。

[The power of LSM Tree](https://images.openai.com/static-rsc-4/d06ePZo9tv6qDhJPtEN5HuKRxEAWRKLIpwaKMZGbXpTePq6zBmO4NUFpGdwXx2JGXqQ47y2pjKBHYhKQtcF17SSmSvzSXtEosBpugExwdqpbIXkrX2HpITyQn1j9zdpqBGEM8FAuzTRUkpp3YaFvFGmKKt2HXVIaoKeojHM1I6k?purpose=inline)

[muhamedabdalla.hashnode.dev](https://muhamedabdalla.hashnode.dev/the-power-of-lsm-tree?utm_source=chatgpt.com)

### 3.1 Leveled Compaction

常见的 LSM Tree 组织方式之一是分层 Compaction。

```
L0:  多个可能相互重叠的 SSTable

L1:  多个 Key 范围互不重叠的 SSTable

L2:  多个 Key 范围互不重叠的 SSTable

L3:  多个 Key 范围互不重叠的 SSTable
```

举个例子：

```
L1:

[ k1 -------- k100 ]
[ k101 ------ k200 ]
[ k201 ------ k300 ]
```

同一层的 SSTable 通常不会有重叠的 Key 范围。

而 L0 不同文件可以有重叠。

当 L0 达到触发条件，数据库会选择 L0 的部分文件，以及 L1 中与它们 Key 范围重叠的文件进行合并。

然后将结果写入 L1。

当 L1 超过容量限制时，再向 L2 合并。

### 3.2 为什么 Compaction 会影响性能？

Compaction 本身需要消耗磁盘 I/O 和 CPU。

例如：

```
前台写入：

Put(k, v)
Put(k, v)
Put(k, v)

后台 Compaction：

读取旧 SSTable
读取新 SSTable
合并数据
写出新 SSTable
```

如果前台写入速度超过后台 Compaction 的处理能力：

- L0 文件不断增加。
- 查询需要检查更多 SSTable。
- 磁盘空间占用增加。
- 数据库可能触发 Write Stall（写入限流甚至暂停）。

因此，LSM Tree 并不是无条件地写入性能更好。

它把一部分写入成本推迟到了后台 Compaction。

### 3.3 Tombstone：删除为什么不是直接删除？

LSM Tree 中还有一个很重要的概念：Tombstone（墓碑）。

假设磁盘上有：

```
SST-001:

k1 -> v1
k2 -> v2
k3 -> v3
```

现在执行：

```
Delete(k2)
```

数据库不会直接修改 SST-001，将 k2 物理删除。

而是在新的 MemTable 中写入一个删除标记：

```
MemTable:

k2 -> TOMBSTONE
```

查询 k2 时，发现最新版本是 Tombstone，就返回不存在。

之后 Compaction 才会真正清理：

```
SST-002:

k1 -> v1
k3 -> v3
```

为什么要这样做？

因为旧版本可能仍然被其他快照读取，或者存在于尚未合并的 SSTable 中。直接删除旧文件里的数据会破坏版本可见性。

Tombstone 本质上是一个带有版本信息的删除记录。

它必须保留到数据库确认更老版本不会再被读取、也不会在其他文件中重新出现之后，才能安全清理。

## 4. LSM Tree 的读取过程

LSM Tree 写入比较直观，但读取相对复杂。

假设：

```
MemTable:
k3 -> v3_new

L0:
SST-001: k1, k3, k5
SST-002: k2, k3, k6

L1:
SST-003: k1, k2, k3, k4
```

现在执行：

```
Get(k3)
```

一般按照从新到旧的顺序检查：

```
1. Active MemTable
2. Immutable MemTable
3. L0 SSTable
4. L1 SSTable
5. L2 SSTable
...
```

找到第一个符合当前读取快照可见性要求的版本，就可以返回。

为了避免每次查询都扫描所有文件，LSM Tree 通常还会使用：

Bloom Filter（布隆过滤器）

Bloom Filter 可以快速判断：

```
某个 SSTable：

k3 可能存在
k9 一定不存在
```

如果确定不存在，就不必读取该 SSTable 的数据块。

此外还会使用：

- SSTable Index：定位 Key 所在的数据块。
- Block Cache：缓存 SSTable 数据块。
- Bloom Filter：减少无效磁盘查询。
- Block Cache Index：减少重复索引读取。

RocksDB 的官方文档详细介绍了 MemTable、SSTable、Bloom Filter 和 Compaction。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



## 5. LSM Tree 的优缺点

| 优点                 | 缺点                      |
| ------------------ | ----------------------- |
| 写入吞吐量高             | 读取可能需要检查多个文件            |
| 顺序写入较多             | Compaction 消耗 CPU 和 I/O |
| SSTable 不可变，易于并行处理 | 存在写放大                   |
| 适合高频更新和追加写入        | 存在空间放大                  |
| 适合大规模 KV 存储        | Compaction 可能造成写入延迟抖动   |

这里需要认识三个重要指标：

- Write Amplification：逻辑写入 1 MB，实际磁盘写入了多少 MB。
- Read Amplification：读取一个 Key，需要检查多少数据结构或磁盘数据块。
- Space Amplification：为了存储 1 GB 有效数据，实际占用了多少磁盘空间。

LSM Tree 的调优，本质上是在这三个指标之间做权衡。

# 二、Copy-on-Write B+ Tree：面向读性能和版本管理的设计

## 1. 先理解 B+ Tree

在讲 Copy-on-Write 之前，先理解 B+ Tree。

B+ Tree 是一种多路平衡查找树，专门适合磁盘存储。

它和普通二叉搜索树最大的区别是：

一个节点可以存储很多 Key，而不是只有一个 Key。

假设一个节点可以存储三个 Key：

```
                  [20 | 40 | 60]
                 /     |     |     \
                /      |     |      \
               v       v     v       v
           [5,10]  [25,30] [45,50] [70,80]
```

这里的内部节点保存的是路由 Key，用来确定下一步应该访问哪个子节点。

B+ Tree 的一个重要特点：

- 内部节点主要负责索引和路由。
- 叶子节点保存实际 Key-Value。
- 叶子节点通常按 Key 顺序组织，并通过指针连接，方便范围扫描。

[B-Trees vs LSM Trees: Comparison and Trade-Offs](https://images.openai.com/static-rsc-4/00ei4_eiB19-Wd_kiN40Xv3D8ADv1FDJOApFCRnwhbjeLaZ9sCDk-K2w0tD9PQT4GZcactYN8hOUt-K6Q_SHKHQ84RZEXD8yfo6Gly-ueC9OEB6vqdbkVWkney4acAyY8TT9liALbDYjwX5gOqah0QmsYNoEwiGAEMbO9cGZ4vM?purpose=inline)

[blog.bytebytego.com](https://blog.bytebytego.com/p/b-trees-vs-lsm-trees-comparison-and?utm_source=chatgpt.com)

### 为什么 B+ Tree 适合磁盘？

假设一个磁盘 Page 大小为 4 KB。

如果一个节点能够存储几百个 Key，那么即使有数百万条记录，B+ Tree 也可能只需要几层。

一次查询：

```
Get(k)
```

通常只需要从 Root 沿着树向下访问少量 Page。

而数据库还会将经常访问的 Root 和内部节点缓存在内存中。

因此，B+ Tree 在点查询和范围查询方面具有很好的性能。

## 2. 普通 B+ Tree 的更新有什么问题？

假设当前 B+ Tree：

```
              Root-A
             /      \
          Node-B   Node-C
          /   \     /   \
        ...   ...  ...  ...
```

现在执行：

```
Put(k, new_value)
```

假设 k 位于 Node-B 的某个叶子节点。

普通的原地更新方式可能是：

1. 找到 Root-A。
2. 找到 Node-B。
3. 找到对应 Leaf Page。
4. 修改 Leaf Page。
5. 将修改后的 Page 写回磁盘。

如果中间出现节点分裂，还需要更新父节点甚至祖先节点。

这里存在一个问题：

如果数据库正在执行一个事务，其他读事务可能仍然需要读取更新之前的树。

如果直接覆盖原来的页面，旧读事务就可能看到被修改后的数据。

Copy-on-Write（写时复制）就是为了解决这个问题。

## 3. Copy-on-Write 的核心思想

Copy-on-Write 的基本原则：

不修改已经存在的页面，而是创建新的页面，构造一棵新的树，最后切换 Root。

假设原来的树是：

```
                Root-A
               /      \
            Node-B   Node-C
            /   \     /   \
           ...  ...  ...   ...
```

现在要修改 Node-B 下的某个叶子节点。

Copy-on-Write 不直接修改 Node-B 和 Leaf Page。

而是：

1. 创建一个新的 Leaf Page。
2. 创建一个新的 Node-B'，指向新的 Leaf Page，同时复用未修改的子节点。
3. 创建一个新的 Root-B，指向 Node-B' 和原来的 Node-C。
4. 提交事务时，将数据库的 Root 指针切换到 Root-B。

最终：

[Persistent Structures: Key to 2000x Speedup in Neon's WAL Indexing - Neon](https://images.openai.com/static-rsc-4/IlPat8i_9t99XblPQQW1neVxLS4DPQFjpjFZqGScO6__FHq53opS0_4_GlT8q__pz-Zj3FMxyL8yoEnaqZt8OlTGIY6cGEf9g_vZOhRECoQQ6sX1uLzWF1ri2pB3E84dYQNre8Ua3njh2vUVZvms3DrbSWDvqTY44IxY9hwR4Zk?purpose=inline)

[neon.com](https://neon.com/blog/persistent-structures-in-neons-wal-indexing?utm_source=chatgpt.com)

```
旧版本：

                Root-A
               /      \
            Node-B   Node-C
            /   \     /   \
           ...  ...  ...   ...

新版本：

                Root-B
               /      \
           Node-B'   Node-C
            /   \     /   \
           ...  ...  ...   ...
```

注意：

- Root-A 没有被修改。
- Node-B 没有被修改。
- Node-C 被新旧树共同引用。
- 只有修改路径上的页面需要复制。

这就是 Path Copying（路径复制）。

## 4. 为什么只需要复制一条路径？

假设 B+ Tree 的高度为 4：

```
Level 0: Root
Level 1: Internal Node
Level 2: Internal Node
Level 3: Leaf Node
```

如果要修改一个 Key：

```
Root
  |
  v
Node-A
  |
  v
Node-B
  |
  v
Leaf-C
```

只有这些页面需要复制：

```
Root'
  |
  v
Node-A'
  |
  v
Node-B'
  |
  v
Leaf-C'
```

其他没有被修改的子树可以直接复用原来的页面。

所以一次修改的页面数量通常与树的高度相关：

\\[ O(\log_B N) \\]

其中：

- \\(N\\) 是 Key 数量。
- \\(B\\) 是 B+ Tree 的分支因子。

当然，实际写入的字节数量还取决于页面大小、节点分裂、元数据更新等因素。

## 5. Copy-on-Write 如何实现事务？

这是 Copy-on-Write B+ Tree 最有价值的地方。

假设数据库当前有一个 Root：

```
Root-A
```

事务 T1 开始：

```
BEGIN;
```

T1 获取当前 Root：

```
T1 -> Root-A
```

此时另一个读事务 T2 也开始：

```
T2 -> Root-A
```

然后 T1 执行：

```
UPDATE accounts
SET balance = 900
WHERE id = 1;
```

T1 创建新的页面：

```
Root-A：旧版本

Root-B：新版本
```

此时：

```
T1 -> Root-B

T2 -> Root-A
```

这意味着：

- T1 可以看到自己修改后的数据。
- T2 仍然看到事务开始时的旧数据。
- T1 的修改不会破坏 T2 的读取。

当 T1 提交时：

```
数据库当前 Root = Root-B
```

而 T2 仍然持有 Root-A。

等 T2 结束后，Root-A 及其不再被其他事务引用的页面才可以回收。

这就是 Copy-on-Write 与 MVCC 结合的典型方式。

## 6. Copy-on-Write 如何实现回滚？

假设：

```
初始 Root = Root-A
```

事务 T1 修改数据：

```
Root-A：旧版本
Root-B：新版本
```

如果 T1 执行：

```
ROLLBACK;
```

其实不需要像 Undo Log 那样逐条恢复修改。

只需要：

```
丢弃 Root-B 这个未提交版本
继续使用 Root-A
```

当然，实际实现还需要处理事务期间分配的新页面、页面分配器状态以及并发读事务。

但从整体思想上讲：

回滚就是放弃新版本，保留旧版本。

## 7. Copy-on-Write 如何保证崩溃恢复？

这里是 Copy-on-Write 最巧妙的地方之一。

假设当前数据库的有效 Root 是：

```
Root-A
```

事务 T1 需要提交新的 Root-B。

一种典型的安全提交流程是：

```
1. 创建新数据页面。
2. 将新页面写入磁盘。
3. 确保新页面持久化。
4. 更新数据库元数据，让 Root-B 成为新的有效 Root。
5. 确保元数据更新持久化。
6. 向客户端返回提交成功。
```

如果在第 3 步之前崩溃：

```
Root-A 仍然有效
```

如果新页面已经写入，但 Root 切换之前崩溃：

```
Root-A 仍然有效
Root-B 是未提交的孤立页面
```

如果 Root 切换已经可靠持久化：

```
Root-B 成为有效版本
```

数据库重新启动时，只需要根据可靠的元数据判断应该使用哪个 Root。

这类技术常被称为 Shadow Paging（影子分页）或 Copy-on-Write。

注意：Copy-on-Write 是一种页面更新策略，不代表所有实现都不需要 WAL。具体的提交顺序、元数据保护、校验和、双元数据页等设计，取决于数据库实现。

## 8. Copy-on-Write B+ Tree 的页面回收

这里还有一个非常重要的问题。

假设：

```
T1 -> Root-A
T2 -> Root-A
```

T1 修改数据，创建了 Root-B 并提交：

```
数据库当前 Root = Root-B

T2 -> Root-A
```

那么 Root-A 已经不是最新版本，能不能立即删除？

不能。

因为 T2 还在使用 Root-A，它引用的页面仍然必须有效。

因此，Copy-on-Write B+ Tree 通常需要页面回收机制：

```
页面状态：

1. 被当前 Root 引用
   -> 不能回收

2. 被旧事务的 Root 引用
   -> 不能回收

3. 不再被任何活跃事务引用
   -> 可以回收
```

以 redb 为例，它维护活动读事务，并延迟复用旧事务释放的页面，直到不会再有读事务引用这些页面。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



这也解释了为什么长时间不结束的读事务可能造成数据库文件持续增长：旧页面不能及时回收。

## 9. Copy-on-Write B+ Tree 的优缺点

| 优点                 | 缺点               |
| ------------------ | ---------------- |
| 读取旧版本和新版本互不干扰      | 写入需要复制路径上的页面     |
| 非常适合 MVCC          | 写入可能产生页面写放大      |
| 回滚可以通过放弃新 Root 实现  | 页面分配和回收机制复杂      |
| 不需要原地覆盖旧页面         | 长事务可能延迟页面回收      |
| B+ Tree 范围扫描效率好    | 写入密集场景可能不如 LSM   |
| 可通过新 Root 切换实现原子提交 | 节点分裂、元数据提交需要仔细设计 |

这里有个容易误解的地方：

Copy-on-Write 并不意味着每次修改都复制整棵树。

它通常只复制从 Root 到被修改叶子节点的路径，以及必要的分裂、合并节点。

# 三、LSM Tree 和 Copy-on-Write B+ Tree 的核心区别

现在把两种数据结构放在一起看。

## 1. 写入流程对比

假设执行：

```
Put("user:1001", "Alice")
```

LSM Tree：

LSM Tree 写入

Put(Key, Value)

WAL 追加写入

MemTable 插入

返回写入成功（取决于持久化策略）

后台：Flush → SSTable → Compaction

Copy-on-Write B+ Tree：

Copy-on-Write B+ Tree 写入

Put(Key, Value)

从 Root 查找目标 Leaf

复制修改路径上的页面

修改新页面，复用未修改的子树

持久化新页面，提交新 Root

返回写入成功

两种方式最大的差异：

- LSM Tree 将写入集中到内存，然后批量刷盘。
- Copy-on-Write B+ Tree 将修改路径复制出来，直接构造新版本的树。

## 2. 读取流程对比

| 读取场景     | LSM Tree                  | Copy-on-Write B+ Tree |
| -------- | ------------------------- | --------------------- |
| 点查询      | 可能检查多个 MemTable / SSTable | 从 Root 沿树查找           |
| 范围查询     | 需要归并多个有序数据源               | 沿叶子节点顺序扫描             |
| 不存在的 Key | Bloom Filter 可以减少文件访问     | 沿 B+ Tree 搜索路径判断      |
| 快照读取     | 依赖版本、序列号和 Snapshot        | 保留旧 Root 和旧页面         |
| 缓存       | Block Cache、索引缓存          | 页面缓存                  |

LSM Tree 的读取不一定慢。Bloom Filter、Block Cache、层级设计和 Compaction 都能显著改善读取性能。

但如果有大量随机点查询，且 SSTable 数量较多，那么 LSM Tree 的读放大可能比较明显。

Copy-on-Write B+ Tree 通常只需要访问一条树路径，因此点查询比较直接。

## 3. 写放大对比

这是两种数据结构最值得关注的性能区别。

假设要更新一个 100 字节的 Value。

LSM Tree：

```
写入 WAL
写入 MemTable
Flush 到 SSTable
后续多轮 Compaction
```

同一条逻辑记录可能在不同阶段被多次写入磁盘。

Copy-on-Write B+ Tree：

```
复制 Root
复制内部节点
复制目标 Leaf
写入新页面
更新 Root 元数据
```

假设 B+ Tree 高度为 4，页面大小为 4 KB，那么一次修改可能需要写入数个新页面，而不是只写入 100 字节。

因此，不能简单地认为 Copy-on-Write 的写放大一定更小。

实际成本取决于：

- B+ Tree 高度。
- 页面大小。
- 每次修改是否集中在相同的 Key 范围。
- 节点分裂频率。
- 页面缓存命中率。
- LSM Tree 的 Compaction 策略。
- Key 和 Value 的大小。

## 4. 两者更适合什么场景？

| 场景         | 更值得考虑                       |
| ---------- | --------------------------- |
| 高频持续写入     | LSM Tree                    |
| 大量随机更新     | 需要实测，通常优先评估 LSM             |
| 高频点查询      | Copy-on-Write B+ Tree       |
| 大量范围查询     | Copy-on-Write B+ Tree 通常更直接 |
| 多读事务、长时间快照 | 两者都支持，但机制不同                 |
| 大量追加日志     | LSM Tree                    |
| 嵌入式数据库     | 两者都适合                       |
| 需要低写入延迟    | 取决于刷盘和 Compaction 策略        |
| 大 Value    | 需要考虑 Value Separation       |

这里还有一个重要的补充：LSM Tree 和 Copy-on-Write B+ Tree 并不是只能二选一。实际存储引擎可以组合多种技术，例如 LSM Tree + Value Log，或者 B+ Tree + WAL。

# 四、Rust 生态中有哪些对应的实现？

结合你之前问的 Badger 替代品，可以直接看下面几个项目。

[Fjall - Database of Databases](https://images.openai.com/static-rsc-4/e57l1iNxyEAOZtsBiB2Q0eLaC61RcyMIdRR16fzuwxjKmltZc2RFp89dFA3oSfgLgRzD4_TKqQRS6pDbwaWplRHEKE7q6jvr8ip_WzOj3Tszu6bHtX7PSDCvcHqrapFQBX-kXkDZP9ZOe9QXj78i2e_qX0I1dCtpiGPyN_0o3Jg?purpose=inline)

Fjall

LSM Tree

- Rust 原生实现。
- Keyspace、SSTable、Compaction。
- 适合研究 LSM Tree 的实现细节。

[GitHub](https://github.com/fjall-rs/fjall?utm_source=chatgpt.com) · [API 文档](https://docs.rs/fjall/latest/fjall/?utm_source=chatgpt.com)

[redb — Rust database // Lib.rs](https://images.openai.com/static-rsc-4/8qa3TxnDLx_ARp7sxhPgWdj7ELNXMr8g9-kJ7JWYzdO3M4438Hd7DYfosbwH0hgJu7L7INAmY5DzZlY146YDhFQ5TjGZZ20p44fXUoxLqOTqE0oklMTo3McRPodUoo-GDKFICdLU-MSN61NKQ6Rg0JdOXxIMvJ3rsL7ZdgOQny4?purpose=inline)

redb

Copy-on-Write B+ Tree

- Rust 原生实现。
- ACID、MVCC。
- 多读事务、单写事务。
- 非常适合研究页面复制、Root 切换和页面回收。

[GitHub](https://github.com/cberner/redb?utm_source=chatgpt.com) · [设计文档](https://github.com/cberner/redb/blob/master/docs/design.md?utm_source=chatgpt.com)

[GitHub - lmdbjava/lmdbjava: Lightning Memory Database (LMDB) for Java: a low latency, transactional, sorted, embedded, key-value store](https://images.openai.com/static-rsc-4/BlU5LIomhNykybOc7jQ4QyBiK0_A6RDb7e7t0_STtFRG-83w0m6kICGCjCu4BWic4uQHkf9LHbq3DK0Njxe6Ppwrb9rjSSOILqID1vynLeN7kIrhNDL51Ch3QOwJGu8sdSrRE6txpKXE-3kY-0-GLrdzWd0la9MYg5MfeBhDZSg?purpose=inline)

LMDB

Copy-on-Write B+ Tree

- C 实现，不是 Rust 原生。
- 使用 Memory Mapping。
- MVCC、读写事务。
- 是研究 Copy-on-Write B+ Tree 的经典实现。

[GitHub](https://github.com/LMDB/lmdb?utm_source=chatgpt.com) · [项目网站](https://www.symas.com/lmdb?utm_source=chatgpt.com)

## 五、推荐阅读资源

下面按照学习顺序整理，尽量优先选择论文和项目官方资料。

### LSM Tree

| 资源                                                                                                                        | 重点                              |
| ------------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| [The Log-Structured Merge-Tree（1996 原始论文）](https://dsf.berkeley.edu/cs286/papers/lsm-acta1996.pdf?utm_source=chatgpt.com) | LSM Tree 的原始设计思想                |
| [RocksDB Overview](https://github.com/facebook/rocksdb/wiki/RocksDB-Overview?utm_source=chatgpt.com)                      | MemTable、SSTable、WAL、Compaction |
| [RocksDB Compaction](https://github.com/facebook/rocksdb/wiki/Compaction?utm_source=chatgpt.com)                          | Leveled、Universal Compaction    |
| [RocksDB Bloom Filter](https://github.com/facebook/rocksdb/wiki/RocksDB-Bloom-Filter?utm_source=chatgpt.com)              | Bloom Filter 的作用与实现             |
| [LSM Tree 课程讲义](https://www.cs.iit.edu/~glavic/cs595/2021-fall/lectures/lsm_trees.html?utm_source=chatgpt.com)            | 适合建立整体概念                        |

### Copy-on-Write B+ Tree

| 资源                                                                                                        | 重点                          |
| --------------------------------------------------------------------------------------------------------- | --------------------------- |
| [redb Design Document](https://github.com/cberner/redb/blob/master/docs/design.md?utm_source=chatgpt.com) | 页面结构、Copy-on-Write、MVCC     |
| [redb API 文档](https://docs.rs/redb/latest/redb/?utm_source=chatgpt.com)                                   | 事务接口和实际使用                   |
| [LMDB 源码](https://github.com/LMDB/lmdb?utm_source=chatgpt.com)                                            | 经典 Copy-on-Write B+ Tree 实现 |
| [LMDB 官方资料](https://www.symas.com/lmdb?utm_source=chatgpt.com)                                            | LMDB 的事务与存储设计               |

## 六、如果你想自己用 Rust 实现一个 KV 数据库，应该怎么学？

我建议不要一开始就直接实现完整的数据库，而是分别实现两个最小版本。

### 第一阶段：实现一个简化版 LSM Tree

按照下面的顺序：

1. 实现 MemTable：使用 `BTreeMap` 或 Skip List。
2. 实现 WAL：支持追加日志和启动恢复。
3. 实现 SSTable：将有序 Key-Value 写入文件。
4. 实现 Flush：将 MemTable 转换成 SSTable。
5. 实现 Get：同时查询 MemTable 和 SSTable。
6. 实现 Merge Iterator：合并多个有序数据源。
7. 实现 Compaction：合并 SSTable。
8. 实现 Tombstone：支持删除。
9. 实现 Bloom Filter：减少无效磁盘读取。
10. 实现 Snapshot：支持读取历史版本。

### 第二阶段：实现一个简化版 Copy-on-Write B+ Tree

建议按照下面的顺序：

1. 实现固定大小的 Page，例如 4 KB。
2. 实现 B+ Tree 的 Leaf Page 和 Internal Page。
3. 实现节点查找、插入、删除。
4. 实现 Page Allocator。
5. 实现 Path Copying。
6. 实现 Root Version。
7. 实现读事务和写事务。
8. 实现旧页面延迟回收。
9. 实现崩溃恢复和原子提交。

### 第三阶段：做性能对比

建议设计以下基准测试：

| 测试             | 关注指标       |
| -------------- | ---------- |
| 随机写入 1000 万条记录 | 写入吞吐量      |
| 随机读取 1000 万条记录 | 点查询延迟      |
| 连续范围扫描         | 扫描吞吐量      |
| 重复更新相同 Key     | 写放大        |
| 大量删除           | 空间回收效率     |
| 长时间读事务 + 高频写入  | 版本保留成本     |
| 数据库异常退出后重启     | 恢复时间和数据正确性 |

最后给你一个我的学习建议：

如果目的是理解现代 KV 数据库的底层原理，我认为最值得对比阅读的是：

- `Fjall`：看 LSM Tree 的 Rust 实现。
- `redb`：看 Copy-on-Write B+ Tree 的 Rust 实现。
- `Badger`：看 LSM Tree 与 Value Log 分离的设计。

尤其建议你重点研究 `redb` 的 `docs/design.md` 和 Fjall 的底层 `lsm-tree`。这两个项目的设计文档和源码能够让你把理论与实际数据库内核联系起来。

真正理解这两种结构的关键，是理解它们如何处理三个问题：数据如何落盘、事务如何隔离、旧数据如何回收。 这三个问题贯穿了几乎所有现代嵌入式 KV 数据库。