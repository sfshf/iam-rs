# 问：给我详细讲解一下B树和B+树这两个数据结构、这两个数据结构的差异、这两个数据结构的应用场景差异

ChatGPT 答：

# B 树（B-Tree）与 B+ 树（B+ Tree）深入详解

B 树和 B+ 树都是多路平衡搜索树（Multiway Balanced Search Tree），主要用于解决大规模数据存储和检索问题。

它们最重要的应用领域是：

- 数据库索引，例如 MySQL InnoDB。
- 文件系统，例如 NTFS、Btrfs。
- 磁盘索引和存储引擎。
- 大规模有序数据的查找、插入、删除和范围扫描。

理解这两种数据结构，最重要的是理解一个问题：

为什么数据库不直接使用红黑树，而要使用 B 树或 B+ 树？

下面从设计动机、数据结构、查找、插入、删除、复杂度、磁盘 I/O 和实际数据库应用几个方面展开。

# 一、为什么需要 B 树和 B+ 树？

## 1. 从二叉搜索树开始

假设有以下数据：

```
10, 20, 30, 40, 50, 60, 70
```

使用二叉搜索树（BST）存储：

[Balanced Binary Search Trees. Binary search trees allow binary search… | by Kalpani Ekanayake | Dec, 2025 | Medium](https://images.openai.com/static-rsc-4/TjJ_9IjiaBMGrFNe4q-9DNGuT7lmWkKvuEUrky41jCQY9hLQ95N0W5SJTq6MJ7KCjHh9l38ckPLhU4icc1Ub0vPaR_IP3OMfOyU9daefHJZm9m3rCgaC8i983jEJdkUw39CM-nZX9SHVHa6dqZ6ACQjEQqAaoVKLge9zrbbpq6o?purpose=inline)

[medium.com](https://medium.com/%40ekanayake.emkn/balanced-binary-search-trees-cf0b5bac781a?utm_source=chatgpt.com)

二叉搜索树具有以下特点：

- 每个节点最多有两个子节点。
- 左子树中的 Key 小于父节点。
- 右子树中的 Key 大于父节点。

如果树保持平衡，查找复杂度为：

\\[ O(\log_2 N) \\]

但是普通二叉搜索树存在一个问题：它不保证平衡。

例如依次插入：

```
10, 20, 30, 40, 50
```

可能得到：

```
10
  \
   20
     \
      30
        \
         40
           \
            50
```

此时树退化成链表，查找复杂度变成：

\\[ O(N) \\]

于是出现了 AVL 树、红黑树等自平衡二叉搜索树。

但即使使用红黑树，仍然存在一个问题。

## 2. 为什么红黑树不适合直接作为数据库磁盘索引？

假设数据库有 1 亿条记录。

红黑树的高度大约为：

\\[ h \approx \log_2(10^8) \approx 27 \\]

查找一个数据，可能需要访问约 27 个节点。

如果这些节点都需要从磁盘读取，就可能产生很多次随机磁盘 I/O。

而磁盘随机读取的成本远高于内存访问。

因此，数据库希望使用一种新的数据结构：

- 一个节点能够存储多个 Key。
- 一个节点能够拥有多个子节点。
- 一个节点最好对应一个磁盘页。
- 尽量减少树的高度。
- 尽量减少磁盘 I/O。

这就是多路搜索树的设计思路。

## 3. 多路搜索树

普通二叉搜索树：

```
        [30]
       /    \
     [20]   [40]
```

多路搜索树：

```
             [20 | 40 | 60]
            /      |      \
           /       |       \
        [10]     [30]     [50 | 70]
```

多路搜索树允许一个节点存储多个 Key。

例如节点：

```
[20 | 40 | 60]
```

可以将数据划分为四个区间：

- 小于 20。
- 20 到 40 之间。
- 40 到 60 之间。
- 大于等于 60。

这样，一个节点就能管理更多数据。

树的高度也就可以显著降低。

B 树和 B+ 树都是在这个基础上发展出来的。

# 二、B 树（B-Tree）

## 1. B 树的定义

B 树是一种自平衡的多路搜索树。

与普通二叉搜索树不同，B 树的每个节点可以存储多个 Key，并拥有多个子节点。

B 树具有以下特征：

1. 每个节点可以存储多个 Key。
2. 每个节点中的 Key 按照从小到大的顺序排列。
3. 一个节点有多个子节点。
4. 一个节点有 \\(k\\) 个 Key 时，最多有 \\(k+1\\) 个子节点。
5. 所有叶子节点都位于同一层。
6. 除根节点外，每个节点必须满足最小 Key 数量要求。
7. 节点 Key 数量不能超过最大容量。

第 5 点非常重要，它保证了 B 树始终保持平衡。

## 2. 什么是 B 树的阶（Order）？

B 树有一个重要概念：阶（Order）。

假设一棵 B 树的阶为 \\(m\\)，这里采用一种常见定义：

- 每个节点最多有 \\(m\\) 个子节点。
- 每个节点最多有 \\(m-1\\) 个 Key。
- 除根节点外，每个内部节点至少有 \\(\lceil m/2 \rceil\\) 个子节点。
- 除根节点外，每个节点至少有 \\(\lceil m/2 \rceil-1\\) 个 Key。
- 如果根节点不是叶子节点，至少有两个子节点。
- 所有叶子节点处于同一层。

例如，阶为 4 的 B 树：

| 属性             | 数量 |
| -------------- | -- |
| 最大子节点数         | 4  |
| 最大 Key 数       | 3  |
| 最小子节点数（非根内部节点） | 2  |
| 最小 Key 数（非根节点） | 1  |

注意：不同教材对阶的定义可能不同，阅读资料时需要先确认定义。

## 3. B 树的结构

来看一个阶为 4 的 B 树：

[Trees and TreeLike Structures Trees TreeLike Structures Binary](https://images.openai.com/static-rsc-4/YUjTm0MnZFVF_xDmP1FtSVij7OI2CJeHS3km4RjdHPmDMuGFZCfd_JOSLReGL2irXJ6Tma1pGHZnLplFn-KzG5LRvzPXu_IDfHstnS0BuF9f2DAiVeOWoXrng4AONQWs_WS7pqSWnD5x_gvptOOXNSu6zBj1GqDYk-cWCyVpClQ?purpose=inline)

[slidetodoc.com](https://slidetodoc.com/trees-and-treelike-structures-trees-treelike-structures-binary/?utm_source=chatgpt.com)

示例：

```
                  [20 | 40]
                 /    |    \
                /     |     \
          [5 | 10] [25 | 30] [50 | 60 | 70]
```

根节点有两个 Key：

```
20、40
```

因此，它拥有三个子节点。

三个子节点分别负责：

```
左子树：      Key < 20

中间子树：    20 < Key < 40

右子树：      Key > 40
```

实际实现还需要明确等于分隔 Key 时的归属规则。

B 树的一个重要特点是：

内部节点和叶子节点都可以存储实际数据记录。

也就是说，查找一个 Key 时，有可能在内部节点直接找到数据。

## 4. B 树的查找

假设查找：

```
Key = 30
```

首先访问根节点：

```
[20 | 40]
```

比较：

```
30 > 20
30 < 40
```

因此进入中间子节点：

```
[25 | 30]
```

找到 30。

查找过程：

```
                  [20 | 40]
                       |
                       |
                  [25 | 30]
                       |
                    找到 30
```

节点内部的 Key 可以使用二分查找。

假设每个节点最多有 \\(m-1\\) 个 Key，则节点内部查找复杂度为：

\\[ O(\log m) \\]

树的高度约为：

\\[ O(\log_m N) \\]

因此总体查找复杂度仍然是：

\\[ O(\log N) \\]

不过，在磁盘场景中，更重要的是访问了多少个节点，也就是磁盘 I/O 次数。

## 5. B 树的插入

B 树插入的核心操作是：

节点分裂（Split）。

假设阶为 4，每个节点最多有 3 个 Key。

当前结构：

```
          [20 | 40]
          /   |   \
       [10] [30] [50]
```

现在插入：

```
35
```

找到中间叶子节点：

```
[30]
```

插入后：

```
[30 | 35]
```

没有超过容量。

继续插入：

```
37
38
```

最终：

```
[30 | 35 | 37 | 38]
```

超过最大容量，必须分裂。

分裂前：

```
[30 | 35 | 37 | 38]
```

以 37 为分隔 Key，将节点分裂：

```
       [37]
       /  \
 [30 | 35] [38]
```

然后将 37 提升到父节点。

父节点原来是：

```
[20 | 40]
```

现在变成：

```
[20 | 37 | 40]
```

最终结构：

```
             [20 | 37 | 40]
             /     |    |   \
          [10] [30|35] [38] [50]
```

如果父节点也满了，就继续向上分裂。

如果根节点分裂，树的高度增加一层。

[B Tree Insertion - Sarthaks eConnect | Largest Online Education Community](https://images.openai.com/static-rsc-4/LurGD58laWZK3xdBmVPyjgGQVGVGGqNVMhZg8_qiG7nzIdBix8nICUsQYQsTvinkQYGWAmyeaYsGGrWucJVej1yi0rSKI51NURFGqUyvAnm3xVWxjFZF5p87pitAaQwBuvnSpBgWVCvsK4hOLyqMW4wZ70k-LiijS3iT6TFpsko?purpose=inline)

[sarthaks.com](https://www.sarthaks.com/3617943/b-tree-insertion?utm_source=chatgpt.com)

## 6. B 树的删除

删除比插入复杂。

核心问题是：

删除 Key 后，节点可能不满足最小 Key 数量要求。

假设：

```
           [20 | 40]
           /   |   \
         [10] [30] [50]
```

删除 30：

```
           [20 | 40]
           /   |   \
         [10]  []   [50]
```

中间节点已经不满足最小 Key 数量要求。

通常有两种处理方式。

### 方式一：借 Key

如果相邻兄弟节点有多余的 Key，就可以从兄弟节点借一个 Key。

例如：

```
删除前：

          [20]
          /  \
       [10] [30 | 40]
```

假设删除左侧节点中的 10：

```
          [20]
          /  \
         [] [30 | 40]
```

可以从右侧兄弟节点借 Key：

```
          [30]
          /  \
         []  [40]
```

这里通过父节点的 Key 和兄弟节点的 Key 重新分配，维持 B 树的约束。

### 方式二：合并节点

如果兄弟节点没有多余的 Key，就需要合并。

例如：

```
          [20]
          /  \
       [10] [30]
```

合并后：

```
       [10 | 20 | 30]
```

父节点中的分隔 Key 也会被移入合并后的节点。

如果父节点发生下溢，则可能继续向上合并。

[B tree short | PPTX](https://images.openai.com/static-rsc-4/uaSwQJkrZpx4geQm7hh5oT5z58DaA8cYvwu3yaHBOM5uPH3eqvqtIxJuZO4Wmll2TrHlzovapZZu1Ai5R4kLThGjD7UvQCxheWp_T3wFVWflhtr3wMBM1QkdmeaiHynXlRsOhniBqnxIc_-L6mL_Huxv6I_OT0-_MNEu4AEYJR0?purpose=inline)

[slideshare.net](https://www.slideshare.net/slideshow/b-tree-short/9993005?utm_source=chatgpt.com)

# 三、B+ 树（B+ Tree）

## 1. B+ 树的定义

B+ 树是 B 树的一种变体，也是一种自平衡多路搜索树。

它最重要的特点是：

实际数据记录只存储在叶子节点中，非叶子节点只存储索引 Key 和子节点指针。

另外，B+ 树的叶子节点通常通过指针连接起来，形成有序链表。

这两个特点决定了 B+ 树非常适合数据库索引。

## 2. B+ 树的结构

[How Databases Store Data: B+ Tree Explained Simply for Beginners (With Real-World Examples) - DEV Community](https://images.openai.com/static-rsc-4/KwmxLI_lmRMKJIhF0oV3aSBQgfj83xi4sxhjm74mhrEutsraV7FfTyRehw641v1jLmBQgHQU7otrg-dmc9t261ObWUQakg_DyNnZi5kztXnsuwwKomafG1dKRm0Y1GtHXmZd_yjZ5i0W_uNe2FUwVwDziaJbC10EW7-FijxAStY?purpose=inline)

[dev.to](https://dev.to/creator79/how-databases-store-data-b-tree-explained-simply-for-beginners-with-real-world-examples-2jmf?utm_source=chatgpt.com)

示意结构：

```
                    [20 | 40]
                   /    |    \
                  /     |     \
                 /      |      \
          [10 | 15] [20 | 30] [40 | 50 | 60]
               <--------><-------->
                叶子节点链表
```

注意：

- 非叶子节点：存储 Key 和子节点指针。
- 叶子节点：存储实际数据记录，或者数据记录的指针。
- 所有叶子节点处于同一层。
- 叶子节点之间通过指针连接。
- 内部节点中的 Key 可以在叶子节点再次出现。

最后一点是 B+ 树与 B 树非常重要的区别。

## 3. 为什么 B+ 树要把数据集中放在叶子节点？

这是 B+ 树设计中最重要的思想。

假设一个数据库有 100 万条记录。

假设：

- 磁盘页大小：16KB。
- Key 大小：8 字节。
- 子节点指针：8 字节。
- 完整数据记录：100 字节。

如果内部节点也存储完整数据记录，那么每个索引项的空间成本就会增加。

粗略计算：

B+ 树内部节点：

\\[ F\_{B+}\approx\frac{16KB}{8+8} \\]

大约能存储 1024 组 Key 和指针。

如果 B 树每个 Key 都携带完整记录：

\\[ F_B\approx\frac{16KB}{8+100+8} \\]

大约只能存储 141 组。

这只是一个简化模型，实际页结构还需要考虑页头、槽目录、对齐、分隔 Key 等额外开销。

但是它说明了一个非常重要的问题：

B+ 树内部节点更小，因此能够容纳更多索引项，拥有更大的扇出（Fanout）。

扇出越大，树通常越矮。

树越矮，查找时需要访问的节点通常越少。

## 4. B+ 树的查找

假设查找：

```
Key = 30
```

根节点：

```
[20 | 40]
```

确定进入中间子树。

到达叶子节点：

```
[20 | 30]
```

找到 30 对应的数据记录。

这里有一个非常重要的区别：

B+ 树即使在内部节点中找到了目标 Key，也通常需要继续向下查找到叶子节点。

因为内部节点中的 Key 只是索引信息，不代表实际数据记录。

因此：

| 数据结构 | 查找特点          |
| ---- | ------------- |
| B 树  | 可能在内部节点直接找到数据 |
| B+ 树 | 必须到叶子节点才能找到数据 |

不过，B+ 树通常具有更大的扇出，树高也往往更低。

所以不能简单认为 B 树的点查询一定更快。

## 5. B+ 树的插入

B+ 树插入也需要处理节点溢出和分裂。

与 B 树有一个重要区别：

叶子节点分裂时，分隔 Key 通常会复制到父节点，而原 Key 仍然保留在叶子节点中。

例如，某个叶子节点最多容纳 3 个 Key：

```
[20 | 30 | 40]
```

现在插入 50：

```
[20 | 30 | 40 | 50]
```

发生溢出。

分裂成：

```
[20 | 30] -> [40 | 50]
```

父节点需要增加分隔 Key：

```
40
```

最终：

```
                [40]
               /    \
       [20 | 30]    [40 | 50]
```

注意：

- 40 仍然保留在右侧叶子节点。
- 40 同时出现在父节点中。
- 叶子节点链表需要更新。
- 如果父节点溢出，也需要继续分裂。

[PPT - B+ Trees PowerPoint Presentation, free download - ID:1200778](https://images.openai.com/static-rsc-4/NPJp_Z7roNl8iuBVj1OKuLaeIPLR5ctXlbdf-Bv4a0S_4yhumTq5aLCqgljOLuwv0ENvZkOLDpp0jtOdMRTnrfRqKOvvqY_xG0V-RqqPoub7p5xpm8rhK4k1W9fcyQUcNeSlnBkJkw-kLxWO5yQ8MbMSxabHk9VQvaZltz_vJ7g?purpose=inline)

[slideserve.com](https://www.slideserve.com/Olivia/b-trees?utm_source=chatgpt.com)

## 6. B+ 树的删除

B+ 树删除也需要维护节点的最小容量。

主要有两种处理方式：

- 从兄弟节点借 Key，重新分配数据，并更新父节点分隔 Key。
- 与兄弟节点合并，同时更新父节点索引和叶子链表。

如果内部节点发生下溢，也需要向上进行调整。

B+ 树的删除比 B 树多了一个需要维护的关系：

叶子节点之间的有序链表关系。

# 四、B 树和 B+ 树的核心区别

现在把两种数据结构放在一起对比。

## 1. 数据存储位置

这是两者最本质的区别。

B 树

[Some indexing data structures - DEV Community](https://images.openai.com/static-rsc-4/dA8Yg-fb8YRwxR9sHqTXrR4XDvweFO2-v8DuyZoTBYbr_o2lkPYF-UChqyTYbt9Zpgy5vNHdgK5vi9NmwMRix0gppAQsmq3SgtpbMvw_CzeesxQLTYrsj309r5PDy7wrUVNBWjDIbZgm3xgpq5IqkKfG2fBiKAzikbrwQL5cn2o?purpose=inline)

内部节点和叶子节点都可以存储实际数据。

B+ 树

[B tree | PPTX](https://images.openai.com/static-rsc-4/ex0tvrB1qXlJE5fkMQ_2e245shcwyFdzua_qO41peiwylCUSMNqlrREQ8l9Ubvn1ZHgj_Duf9HpHtN4KFYGSN3wRtAGvJuHhOHSryYPDFQfm-m6eYorasPj21hx9Gr8-m_DxEs1P5VzmSgMLhYXrJ8iGL_3i_KOmFlVoCEhJyW0?purpose=inline)

只有叶子节点存储实际数据，内部节点只负责索引导航。

| 对比项           | B 树                  | B+ 树      |
| ------------- | -------------------- | --------- |
| 内部节点          | Key + 数据（或记录指针）+ 子指针 | Key + 子指针 |
| 叶子节点          | Key + 数据             | Key + 数据  |
| 数据存储位置        | 整棵树                  | 叶子节点      |
| 内部 Key 是否可能重复 | 通常不重复                | 可能在叶子节点重复 |
| 查找是否可能提前结束    | 是                    | 否         |
| 叶子节点是否有序链表    | 不要求                  | 通常有       |
| 范围查询          | 需要后继遍历               | 直接扫描叶子链表  |

需要注意：B 树并不一定必须在内部节点存储完整数据，具体取决于实现。这里比较的是典型的数据组织方式。

## 2. 树的高度

为什么 B+ 树通常比 B 树矮？

因为 B+ 树内部节点只存储索引 Key 和子节点指针。

所以，一个磁盘页能够容纳更多 Key。

假设：

- 数据量为 1 亿条。
- B+ 树扇出为 1000。
- B 树扇出为 100。

粗略估算：

\\[ h\_{B+}\approx\log\_{1000}(10^8)\approx2.67 \\]

\\[ h_B\approx\log\_{100}(10^8)\approx4 \\]

也就是说，B+ 树可能只需要约 3 层，而 B 树可能需要约 4 层。

实际树高还取决于节点填充率、分裂策略、数据记录大小等因素。

因此，B+ 树通常具有更低的树高。

## 3. 单点查询

假设查询：

```
SELECT * FROM users WHERE id = 100;
```

B 树可能出现：

```
根节点 [20 | 50 | 100]
                  |
               找到 100
```

直接在根节点命中。

B+ 树则需要：

```
根节点
   |
中间节点
   |
叶子节点
   |
找到 100
```

因此 B 树具有提前命中的可能性。

但是，数据库的实际性能还需要考虑：

- 节点扇出。
- 树的高度。
- Buffer Pool 缓存命中率。
- 磁盘页读取次数。
- 记录大小。

所以单点查询并不能简单地说 B 树一定优于 B+ 树。

## 4. 范围查询

这是 B+ 树最重要的优势之一。

假设执行：

```
SELECT *
FROM users
WHERE id >= 100
  AND id <= 200;
```

B+ 树的执行过程：

第一步：定位到 Key = 100 对应的叶子节点。

第二步：沿着叶子节点链表向后扫描。

```
[100 | 110 | 120]
          |
          v
[130 | 140 | 150]
          |
          v
[160 | 170 | 180]
          |
          v
[190 | 200 | 210]
```

第三步：遇到 210，超过范围上界，停止扫描。

时间复杂度：

\\[ O(\log N + K) \\]

其中：

- \\(N\\)：总记录数量。
- \\(K\\)：符合范围条件的记录数量。

B 树也可以实现范围查询，但是通常需要使用树的中序遍历或后继查找机制，可能需要反复回溯父节点。

因此，B+ 树更适合范围查询。

## 5. 顺序扫描

例如：

```
SELECT * FROM users ORDER BY id;
```

B+ 树：

```
叶子节点 1 -> 叶子节点 2 -> 叶子节点 3 -> ...
```

从最左侧叶子节点开始，沿着链表顺序访问即可。

B 树：

需要使用中序遍历等方式实现有序访问。

虽然两者都可以实现 \\(O(N)\\) 的有序遍历，但 B+ 树的叶子链表更适合连续扫描，也更容易利用磁盘顺序预读。

# 五、时间复杂度对比

假设：

- \\(N\\)：数据总量。
- \\(K\\)：范围查询返回的数据量。

| 操作   | B 树               | B+ 树              |
| ---- | ----------------- | ----------------- |
| 单点查找 | \\(O(\log N)\\)   | \\(O(\log N)\\)   |
| 插入   | \\(O(\log N)\\)   | \\(O(\log N)\\)   |
| 删除   | \\(O(\log N)\\)   | \\(O(\log N)\\)   |
| 范围查询 | \\(O(\log N+K)\\) | \\(O(\log N+K)\\) |
| 全量遍历 | \\(O(N)\\)        | \\(O(N)\\)        |

这里需要特别强调：

两种数据结构在渐进时间复杂度上基本相同。

真正的差异主要在：

1. 节点扇出。
2. 树的高度。
3. 磁盘 I/O 次数。
4. 缓存效率。
5. 范围扫描的实现成本。

这也是为什么只看大 O 复杂度，无法充分解释数据库为什么偏爱 B+ 树。

# 六、两者的实际应用场景

## 1. B 树的应用场景

B 树适合：

- 有序数据的多路查找。
- 某些磁盘存储索引。
- 某些文件系统元数据管理。
- 数据量大、需要平衡查找的存储系统。

它的特点是数据可以分布在整棵树上，因此点查询有机会提前命中。

但在现代数据库的典型磁盘索引场景中，B+ 树通常更有优势。

## 2. B+ 树的应用场景

B+ 树适合：

- 关系型数据库索引。
- 大规模磁盘索引。
- 范围查询。
- 排序与顺序扫描。
- 文件系统中的有序索引。

尤其是关系型数据库，B+ 树几乎是经典的索引数据结构。

# 七、MySQL InnoDB 为什么选择 B+ 树？

这是理解 B+ 树实际价值最重要的案例。

MySQL InnoDB 使用 B+ 树组织索引。

我们从两个索引来看。

## 1. 聚簇索引（Clustered Index）

例如：

```
CREATE TABLE users (
    id BIGINT PRIMARY KEY,
    name VARCHAR(100),
    age INT,
    email VARCHAR(200)
);
```

InnoDB 会根据主键组织聚簇索引。

示意：

[indexing - Disk IO in MySQL clustered index - Stack Overflow](https://images.openai.com/static-rsc-4/J9FpXBX0BEmwJlv4MvMZbLQRsB3nK-kmQbAG97ORgMy5xdLkMeMy6exKmYVcIACDzsCHznZInHi3g-Y0oeCPj4LaP_I6jj_h_gRb7Gj09K-EgHTQ7SF2jRHArFVmLyIST0CXGKjJa8tCtOf8z596TzMMihZjmWFZXE8y68x0ruo?purpose=inline)

[stackoverflow.com](https://stackoverflow.com/questions/55836641/disk-io-in-mysql-clustered-index?utm_source=chatgpt.com)

内部节点存储：

```
主键 Key + 子节点指针
```

叶子节点存储：

```
完整的行记录
```

因此：

```
SELECT *
FROM users
WHERE id = 100;
```

通过主键索引找到叶子节点，就可以取得完整行记录。

## 2. 二级索引（Secondary Index）

例如：

```
CREATE INDEX idx_age ON users(age);
```

二级索引的叶子节点存储：

```
age + primary_key
```

例如：

```
age    primary_key
20     1001
21     1002
22     1003
23     1004
```

执行：

```
SELECT *
FROM users
WHERE age = 22;
```

查询过程：

1. 通过 `idx_age` 找到 age = 22。
2. 获取主键 id = 1003。
3. 根据主键 id = 1003 查询聚簇索引。
4. 获取完整行记录。

这个过程叫作回表（Bookmark Lookup / Table Lookup）。

[MySQL学习笔记（三）索引-上篇-腾讯云开发者社区-腾讯云](https://images.openai.com/static-rsc-4/PN1m4pHOjdr2n1IP3MDTvi-FZmw9B0a4YooDnRHIafmsDrk8Hq8SIly7mOcNKHg4Z1a7EM07O948Ht1XAjBBJGafgi-ee2hbuFH_EhcnllfNEe0NydHV2VW4h_JJkeoby7oSIMHLWMlF42dxBAMF_VpcvuP4I3MhF-LPcBKIbx0?purpose=inline)

[cloud.tencent.cn](https://cloud.tencent.cn/developer/article/1580225?utm_source=chatgpt.com)

如果查询需要的所有字段都包含在二级索引中，则可能直接通过二级索引完成查询，这叫作覆盖索引（Covering Index），不需要回表。

例如：

```
SELECT age
FROM users
WHERE age = 22;
```

## 3. B+ 树为什么适合 InnoDB？

总结起来：

| InnoDB 的需求 | B+ 树提供的优势    |
| ---------- | ------------ |
| 大量数据存储     | 多路平衡结构       |
| 降低磁盘 I/O   | 内部节点扇出大      |
| 主键点查询      | 高效索引定位       |
| 范围查询       | 叶子节点链表       |
| 排序查询       | 有序叶子节点       |
| 顺序扫描       | 连续访问叶子页      |
| 缓存优化       | 内部节点体积小，适合缓存 |

另外，InnoDB 默认页大小通常是 16KB，一个 B+ 树节点通常对应一个索引页。

这意味着 B+ 树的设计与数据库磁盘页的组织方式非常契合。

# 八、为什么不是所有数据库都使用 B+ 树？

理解这一点，可以进一步理解存储引擎的设计。

不同数据库面临的工作负载不同。

例如：

| 存储引擎               | 主要数据结构      | 设计侧重点           |
| ------------------ | ----------- | --------------- |
| MySQL InnoDB       | B+ 树        | 通用 OLTP、点查和范围查询 |
| PostgreSQL         | B-tree 索引   | 通用索引、范围查询和并发访问  |
| MongoDB WiredTiger | B-tree 家族结构 | 文档数据库索引         |
| RocksDB            | LSM-Tree    | 写入密集型存储         |

特别值得了解的是 RocksDB。

RocksDB 为什么没有选择传统 B+ 树作为核心存储结构？

因为它非常重视写入性能。

传统 B+ 树插入数据时，可能需要：

- 定位目标叶子页。
- 修改磁盘页。
- 发生节点分裂。
- 更新父节点索引。
- 产生随机写入。

LSM-Tree 则采用不同的思路：

```
写入请求
    |
    v
MemTable（内存有序结构）
    |
    v
Immutable MemTable
    |
    v
SST 文件（磁盘有序文件）
    |
    v
Compaction（后台合并）
```

通过先写内存、再批量刷盘，减少随机写入。

因此：

- 读多写少、需要高效范围查询的场景，B+ 树非常合适。
- 写入密集型场景，LSM-Tree 往往更有优势。

当然，实际选择还要考虑读放大、写放大、空间放大、延迟要求和缓存策略。

# 九、总结：应该如何理解两者的本质差异？

我认为最值得记住的是下面这张表：

| 维度   | B 树         | B+ 树     |
| ---- | ----------- | -------- |
| 本质   | 多路平衡搜索树     | B 树的变体   |
| 数据位置 | 内部节点和叶子节点   | 仅叶子节点    |
| 内部节点 | 可以存储数据      | 只存储索引信息  |
| 查找   | 可能提前命中      | 必须到叶子节点  |
| 扇出   | 典型实现中较小     | 通常较大     |
| 树高   | 通常较高        | 通常较低     |
| 范围查询 | 需要后继遍历      | 叶子链表扫描   |
| 顺序遍历 | 中序遍历        | 叶子链表     |
| 典型应用 | 某些存储和文件系统索引 | 关系型数据库索引 |

一句话记忆：

- B 树：数据分布在整棵树上，查找可能提前结束。
- B+ 树：数据集中在叶子节点，内部节点只负责导航，叶子节点通过链表连接。

B+ 树并不是所有情况下都比 B 树快，而是它的设计更符合数据库磁盘索引的工作特点。

# 十、建议进一步深入的三个知识点

如果你希望从数据结构进一步理解数据库底层，我建议按这个顺序学习：

第一阶段

B+ 树的页结构与扇出计算

研究 16KB InnoDB Page 内部究竟存储了什么，如何根据 Key 大小、指针大小、页头和记录大小计算扇出，以及为什么不同索引的树高可能不同。

第二阶段

InnoDB 索引与查询执行

深入理解聚簇索引、二级索引、回表、覆盖索引、最左前缀原则，以及联合索引在 B+ 树中的实际排列方式。

第三阶段

B+ 树与 LSM-Tree 的底层比较

理解随机写、顺序写、读放大、写放大、Compaction，以及为什么 RocksDB 选择 LSM-Tree。

我尤其建议下一步研究 InnoDB 的 16KB B+ 树索引页内部结构，以及联合索引 `(a,b,c)` 在 B+ 树中的真实排列方式。这能把 B+ 树理论直接联系到日常 SQL 查询优化。