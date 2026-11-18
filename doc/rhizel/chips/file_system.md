# 问：给我详细讲解一下linux系统的文件系统设计，并以文本文件和可执行文件给我讲解一下

ChatGPT答：

Linux 文件系统是 Linux 操作系统中非常核心的一部分。理解它，不仅能帮助你理解文件是如何存储在磁盘上的，还能进一步理解 Linux 的文件读写、权限管理、程序加载、内存映射、动态链接、进程启动等机制。

我会从整体架构 → 磁盘数据组织 → 文件元数据 → 文件读写过程 → 文本文件 → ELF 可执行文件这个顺序讲解。

重点放在两个实际例子上：

1. 一个普通的文本文件，例如 `hello.txt`。
2. 一个 Linux 可执行文件，例如用 C 编译出来的 `hello` 程序。

我们最终要理解一个问题：

当你执行 `cat hello.txt` 或者 `./hello` 时，Linux 内核究竟做了什么？

# 一、Linux 文件系统的整体设计

## 1. Linux 的一切皆文件

Linux 有一个非常重要的设计思想：

Everything is a file（一切皆文件）。

在 Linux 中，文件不仅仅指磁盘上的普通文件，还包括：

| 类型      | 例子                 | 作用                 |
| ------- | ------------------ | ------------------ |
| 普通文件    | `/etc/passwd`      | 存储文本、程序、图片等数据      |
| 目录文件    | `/home/user`       | 保存文件名与 inode 的对应关系 |
| 字符设备文件  | `/dev/null`        | 按字符流方式访问设备         |
| 块设备文件   | `/dev/sda`         | 访问磁盘等块设备           |
| FIFO 管道 | `mkfifo` 创建的文件     | 进程间通信              |
| Socket  | Unix Domain Socket | 进程间通信              |
| 符号链接    | `/bin -> /usr/bin` | 指向另一个文件的路径         |

需要注意：

并不是所有这些对象都真实存储在磁盘上。例如 `/proc` 和 `/sys` 通常是内核提供的虚拟文件系统。

这种设计使得很多操作可以统一使用类似的接口：

```
open()
read()
write()
close()
```

例如：

```
cat hello.txt
```

本质上就是打开文件、读取数据、将数据写入标准输出。

## 2. Linux 文件系统的分层架构

Linux 并不直接让应用程序操作磁盘，而是通过多层抽象来实现。

[Course Section — obyo | obyo](https://images.openai.com/static-rsc-4/5lnioaJOOe8EWgYEVrtK_WsICzgjN2kwcSvrwby1s1wlICBz_SnrRZ3Eav1H8_nrqmnfQhjdmb8epWRw-hLbFxnfj9ygwSHeU1DIIkt0rX9gwWncXp16rJOSuoWGkEb6lFObd81_GixT7cK5qaChV2b8NCK82eloHNaCVjZ5aZg?purpose=inline)

[obyo.ai](https://obyo.ai/courses/linux-under-the-hood-how-a-modern-operating-system-actually-works/the-proc-and-sys-filesystems-the-kernels-control-panel?utm_source=chatgpt.com)

可以把它理解成下面这个结构：

```
          应用程序
              |
       open / read / write
              |
       系统调用接口
              |
       VFS（虚拟文件系统）
              |
    +---------+----------+
    |         |          |
   ext4      XFS        Btrfs
    |         |          |
    +---------+----------+
              |
        块设备层
              |
        设备驱动程序
              |
         磁盘 / SSD
```

这里有一个非常重要的概念：VFS（Virtual File System，虚拟文件系统）。

VFS 是 Linux 内核提供的一层统一抽象。

假设你的机器同时存在：

- `/` 使用 ext4。
- `/data` 使用 XFS。
- `/mnt/usb` 使用 FAT32。

对于应用程序来说，它们都可以使用：

```
open("/data/test.txt", O_RDONLY);
```

应用程序不需要关心底层究竟使用 ext4、XFS 还是 FAT32。

VFS 会把请求分发给对应的具体文件系统实现。

## 3. Linux 常见文件系统

| 文件系统  | 特点                          |
| ----- | --------------------------- |
| ext4  | Linux 最常见的通用文件系统之一          |
| XFS   | 擅长处理大文件和高并发 I/O             |
| Btrfs | 支持快照、写时复制、校验等               |
| ZFS   | 提供高级存储管理和数据完整性功能，通常通过额外模块使用 |
| FAT32 | 兼容性好，但不支持 Linux 原生权限和日志机制   |
| NTFS  | Windows 常用文件系统，Linux 也支持    |

下面主要以 ext4 为例，因为它比较适合用来理解 Linux 文件系统的核心设计。

# 二、Linux 文件系统在磁盘上是如何组织的？

这是理解文件系统最重要的部分。

首先需要区分三个概念：

- 文件名（Filename）
- inode
- 文件数据（Data）

很多初学者会认为：

```
hello.txt
```

就是磁盘上真实存储数据的那个对象。

实际上不是。

Linux 文件系统通常采用这样的组织方式：

```
目录项（文件名） ---> inode ---> 文件数据
```

我们分别来看。

## 1. inode：文件的核心元数据

inode（Index Node）是 Linux 文件系统中描述文件的核心数据结构。

一个 inode 通常记录：

| 属性    | 说明                |
| ----- | ----------------- |
| 文件类型  | 普通文件、目录、符号链接等     |
| 文件权限  | rwx 权限            |
| 文件所有者 | UID               |
| 文件所属组 | GID               |
| 文件大小  | 文件占用的逻辑字节数        |
| 时间信息  | atime、mtime、ctime |
| 硬链接数量 | 有多少目录项指向这个 inode  |
| 数据块位置 | 文件数据在磁盘上的位置       |

注意：inode 通常不保存文件名。

例如：

```
ls -i hello.txt
```

可能得到：

```
123456 hello.txt
```

其中 `123456` 就是这个文件的 inode 编号。

可以通过：

```
stat hello.txt
```

查看更加详细的元数据信息。

## 2. 文件名到底存在哪里？

文件名保存在目录中。

例如：

```
/home/user/
    |
    +---- hello.txt
    |
    +---- main.c
    |
    +---- app
```

从概念上看，目录保存着类似下面的映射：

```
hello.txt  ---> inode 1001
main.c     ---> inode 1002
app        ---> inode 1003
```

目录本身也是一种特殊的文件，它有自己的 inode，其数据内容包含目录项信息。

因此：

文件名属于目录项，而文件属性属于 inode。

这也是为什么 Linux 可以支持硬链接。

## 3. 硬链接和软链接

假设我们创建一个文本文件：

```
echo "Hello Linux" > hello.txt
```

然后创建一个硬链接：

```
ln hello.txt hello_hard.txt
```

再创建一个软链接：

```
ln -s hello.txt hello_soft.txt
```

它们的关系大致如下：

[Inode in the Linux file system - SoByte](https://images.openai.com/static-rsc-4/RUNzYyA6XyIhjjEswd5kmHNbMYmwzzU2votAu5_s0rtTbktYZaTAW1WfkX2y6N-1SgkIym8ZBX_l5QyxXjw9wOaAx_Eyn_n9LxrBL_FoU8O5fWBceRC__cqgbGtn62l9gfLfw7tbLNL5Ef7njd04Frl2mbRXUGk1zsijCjqzUNw?purpose=inline)

[sobyte.net](https://www.sobyte.net/post/2022-08/linux-inode/?utm_source=chatgpt.com)

```
hello.txt --------+
                  |
                  +----> inode 1001 ----> 文件数据
                  |
hello_hard.txt ---+

hello_soft.txt --------> 自己的 inode ----> "hello.txt"
```

硬链接：

- 两个文件名指向同一个 inode。
- 修改其中一个文件的内容，另一个也会看到变化。
- 删除其中一个文件名，不会导致文件数据立即消失。
- 通常不能跨文件系统创建。
- 不能为目录创建普通硬链接。

软链接：

- 自己拥有独立的 inode。
- 文件数据保存的是目标路径。
- 类似 Windows 的快捷方式，但行为上存在差异。
- 可以跨文件系统。
- 目标文件删除后，软链接可能变成悬空链接。

可以通过以下命令观察 inode：

```
ls -li hello.txt hello_hard.txt hello_soft.txt
```

你会发现前两个 inode 相同，而软链接的 inode 不同。

# 三、文件数据在磁盘上如何存储？

前面说 inode 保存了文件数据的位置。

那么它究竟如何定位数据呢？

## 1. 磁盘块（Block）

文件系统通常不会逐字节管理磁盘，而是以块为单位管理空间。

例如 ext4 常见的块大小是 4096 字节（4KB）。

假设有一个文件：

```
hello.txt
```

文件内容为：

```
Hello Linux!
```

总共只有 12 个字节。

虽然文件只有 12 字节，但文件系统会按照自己的分配策略管理它所占用的空间。

在简单情况下，它可能占用一个 4KB 数据块。

```
磁盘数据块：

+--------------------------------+
| Block 100                      |
|                                |
| Hello Linux!                   |
|                                |
| 剩余空间                       |
|                                |
+--------------------------------+
```

这里要区分两个概念：

- 文件逻辑大小：12 字节。
- 实际分配空间：可能是 4096 字节。

但现代文件系统支持延迟分配、内联数据和稀疏文件等机制，因此不能简单认为每个小文件一定占用一个完整的磁盘块。

## 2. inode 如何找到文件数据？

以 ext4 为例，文件的数据块映射主要使用 extent 机制。

Extent 可以理解为一段连续的磁盘块。

例如：

```
文件逻辑块：

0    1    2    3    4    5
|----|----|----|----|----|----|

磁盘物理块：

100  101  102  200  201  202
|----|----|----|----|----|----|
```

可以用两个 extent 表示：

```
Extent 1:
逻辑块 0 ~ 2
物理块 100 ~ 102

Extent 2:
逻辑块 3 ~ 5
物理块 200 ~ 202
```

这样就不需要为每一个逻辑块单独保存一个物理地址。

这也是 ext4 在处理大文件时非常重要的优化。

## 3. ext4 的磁盘整体结构

一个 ext4 文件系统通常包含：

- Superblock
- Block Group Descriptor
- Block Bitmap
- Inode Bitmap
- Inode Table
- Data Blocks

简化示意：

[📁 Hard Disks and File Systems](https://images.openai.com/static-rsc-4/nGjTHxCj8ZDA4Hjk1A5UAKZiB3UHGRllUvsI8HJvI8MOFOu_w3IvEL14kTASLUpmqzUEjuDI12dRtNPjuTbm63D53D4l6wOnaG1d5RS-GYsEUZcAGN_RAJuzqsxBsq4agtyzZfIOvX9qx3yJm1Tk96RgTwOtakYawYSVc4F18AM?purpose=inline)

[johntai.net](https://johntai.net/posts/chfi-notes/module-03/?utm_source=chatgpt.com)

各部分的作用：

| 组件                     | 作用             |
| ---------------------- | -------------- |
| Superblock             | 保存整个文件系统的核心信息  |
| Block Group Descriptor | 描述各个块组的信息      |
| Block Bitmap           | 记录数据块的分配情况     |
| Inode Bitmap           | 记录 inode 的分配情况 |
| Inode Table            | 保存 inode 元数据   |
| Data Blocks            | 保存文件实际数据       |

ext4 会把磁盘划分为多个 Block Group。

这样做的目的是让 inode 和对应的数据块尽可能靠近，减少磁盘寻址开销，并改善局部性。

另外，ext4 还有日志机制（Journaling），用于降低系统崩溃或断电时文件系统元数据不一致的风险。

注意：日志并不等于所有文件内容都必然先写入日志。ext4 有不同的数据日志模式，默认通常是 `data=ordered`。

# 四、Linux 内核如何管理一个打开的文件？

前面讲的是磁盘上的组织方式。现在我们从内核角度来看一个文件。

这里有四个非常重要的数据结构：

1. `inode`
2. `dentry`
3. `file`
4. `address_space`

它们是理解 Linux 文件系统源码时经常遇到的核心概念。

## 1. inode：描述文件本身

内核中的 `struct inode` 表示一个文件对象。

它包含文件类型、权限、大小、所有者等信息，以及文件系统相关的操作接口。

简化理解：

```
struct inode {
    umode_t i_mode;       // 文件类型和权限
    kuid_t i_uid;         // 文件所有者
    kgid_t i_gid;         // 文件所属组
    loff_t i_size;        // 文件大小
    unsigned long i_ino;  // inode 编号

    // 其他文件系统相关信息
};
```

这只是示意，实际 Linux 内核结构复杂得多。

## 2. dentry：描述文件名与 inode 的关系

内核使用 `struct dentry` 管理目录项。

例如：

```
/home/user/hello.txt
```

内核解析这个路径时，会逐级查找：

```
/
 |
 +-- home
      |
      +-- user
            |
            +-- hello.txt
```

每一个目录项都可以通过 dentry 缓存起来。

这样下一次访问相同路径时，就不一定需要重新从磁盘读取目录信息。

dentry 的核心作用是：

建立路径名与 inode 之间的联系，并缓存路径查找结果。

注意，dentry 和 inode 不是同一个东西：

- 一个 inode 可以对应多个 dentry，例如硬链接。
- dentry 主要负责路径和目录项管理。
- inode 负责描述文件对象。

## 3. file：描述一次打开的文件

当你执行：

```
int fd = open("hello.txt", O_RDONLY);
```

内核成功打开文件后，会创建一个 `struct file` 对象。

它表示一次打开的文件实例。

简化理解：

```
struct file {
    struct path f_path;
    struct inode *f_inode;
    const struct file_operations *f_op;

    loff_t f_pos;   // 当前文件读写位置

    // 其他信息
};
```

这里最值得注意的是 `f_pos`。

假设文件内容为：

```
Hello Linux!
```

当你执行：

```
read(fd, buf, 5);
```

读取 5 个字节之后，文件偏移量会从 0 变成 5。

再次执行：

```
read(fd, buf, 5);
```

就会从第 5 个字节继续读取。

## 4. 文件描述符（File Descriptor）

应用程序拿到的 `fd`，其实是一个整数。

例如：

```
int fd = open("hello.txt", O_RDONLY);
```

可能返回：

```
fd = 3
```

这个 3 并不是 inode 编号，也不是磁盘块编号。

它是当前进程文件描述符表中的索引。

关系如下：

[Module 4: Operating system kernel tracing](https://images.openai.com/static-rsc-4/yzSzEh0iiEDM7VTuzcq1tlC9OfT2jZmqpEVMeVwXrT58pT279HPSGUeFJXIzul-Nqv-I0oRIKmyKyV6Z8pBgKNM0v_y7oGa3lGiFNclk_lVcnMJs6S6zEJgJxWb35jMbXtWNiQFUa7bC0haadRtbAy54NRWH-Ux4VGL4xIaHBMY?purpose=inline)

[myaut.github.io](https://myaut.github.io/dtrace-stap-book/kernel/fs.html?utm_source=chatgpt.com)

```
进程 A
  |
  +-- 文件描述符表
         |
         +-- 0 ---> struct file（标准输入）
         |
         +-- 1 ---> struct file（标准输出）
         |
         +-- 2 ---> struct file（标准错误）
         |
         +-- 3 ---> struct file
                       |
                       +-- inode
                       |
                       +-- 文件偏移量
                       |
                       +-- 文件操作接口
```

Linux 默认约定：

| 文件描述符 | 含义                |
| ----- | ----------------- |
| 0     | 标准输入 stdin        |
| 1     | 标准输出 stdout       |
| 2     | 标准错误 stderr       |
| 3 及以上 | 通常是程序自己打开的文件或其他资源 |

需要注意：

`dup()`、`fork()` 等操作可以让多个文件描述符引用同一个 `struct file`，此时它们可能共享文件偏移量。

而独立调用两次 `open()`，通常会产生两个独立的 `struct file` 对象，即使它们指向同一个 inode。

# 五、以文本文件为例：Linux 如何读写一个文件？

现在我们使用一个具体例子。

创建文件：

```
echo "Hello Linux Filesystem" > hello.txt
```

查看内容：

```
cat hello.txt
```

我们分别分析写入和读取的过程。

## 1. 写入文本文件

假设执行：

```
echo "Hello Linux Filesystem" > hello.txt
```

大致会经历以下过程：

[Essential Linux Page Cache theory | Viacheslav Biriukov](https://images.openai.com/static-rsc-4/0AMmziRIySGzKYusBjdvqGO6bayqyo8wSCTX_kuYuV4M4qQT-PRKrdkdQrUYjUrUR8O5O94xmNNuVF-LMEqQa0rmLGFHV5Xucq5wPL81Unaxw6lcz115T15UqP7hBF0mfwpIOWMLebU2KfsmsDpsH6JNDZ5UZFrbdSUl7UGvIQ0?purpose=inline)

[biriukov.dev](https://biriukov.dev/docs/page-cache/2-essential-page-cache-theory/?utm_source=chatgpt.com)

```
echo 命令
    |
    v
shell 创建重定向
    |
    v
open("hello.txt", O_WRONLY | O_CREAT | O_TRUNC)
    |
    v
获取文件描述符
    |
    v
write(fd, data, length)
    |
    v
系统调用进入内核
    |
    v
VFS
    |
    v
具体文件系统（例如 ext4）
    |
    v
Page Cache
    |
    v
磁盘
```

这里有一个重要概念：Page Cache（页缓存）。

现代 Linux 文件系统的普通 buffered I/O 通常会经过页缓存。

当执行：

```
write(fd, "Hello Linux", 11);
```

内核通常不会立即把数据直接写入 SSD。

而是先将数据写入页缓存，相关页面被标记为 Dirty（脏页）。

随后内核通过回写机制将数据写入磁盘。

所以：

```
write() 返回成功
```

并不一定意味着数据已经物理落盘。

如果你希望更明确地要求数据持久化，可以使用：

```
fsync(fd);
```

`fsync()` 会请求内核同步文件数据及相关元数据，但要获得完整的断电持久性保证，还需要考虑文件创建、重命名时目录项的同步，以及硬件是否正确执行 flush 等问题。

## 2. 读取文本文件

执行：

```
cat hello.txt
```

大致过程：

```
cat
 |
 v
open("hello.txt", O_RDONLY)
 |
 v
VFS 路径解析
 |
 v
找到 dentry
 |
 v
找到 inode
 |
 v
创建 struct file
 |
 v
read(fd, buffer, size)
 |
 v
检查 Page Cache
 |
 +---- 命中 ----> 直接从内存读取
 |
 +---- 未命中 --> 从磁盘读取数据
 |
 v
返回用户空间
 |
 v
write(1, buffer, size)
 |
 v
终端显示内容
```

这里有一个很重要的优化：

Linux 会缓存最近访问过的文件数据。

假设第一次执行：

```
cat hello.txt
```

需要从 SSD 读取数据。

第二次执行：

```
cat hello.txt
```

如果数据仍然在 Page Cache 中，就可能直接从内存读取，而不需要再次访问磁盘。

这也是为什么重复读取同一个文件时，通常会快很多。

## 3. 文本文件本身有什么特殊之处？

实际上，Linux 文件系统并不知道什么是文本文件。

对于文件系统来说：

```
hello.txt
```

就是一串字节。

例如：

```
Hello Linux
```

对应的 ASCII / UTF-8 字节为：

```
48 65 6c 6c 6f 20 4c 69 6e 75 78
```

文件系统只负责保存这些字节，不关心它们代表：

- 英文字符。
- 中文字符。
- JSON。
- XML。
- YAML。
- 图片数据。
- 视频数据。

因此：

```
file hello.txt
```

可能会根据文件内容判断它是 ASCII 文本，但这只是用户空间工具进行的内容识别，并不是 ext4 对文本文件的特殊处理。

同样，文件扩展名 `.txt` 也不是 Linux 内核判断文件类型的依据。

例如：

```
mv hello.txt hello.bin
```

并不会改变文件本身的数据，也不会改变它的 inode（同一文件系统内重命名时）。

# 六、以可执行文件为例：Linux 是如何存储程序的？

现在进入一个更加有意思的部分。

我们编写一个简单的 C 程序：

```
#include <stdio.h>

int main() {
    printf("Hello Linux!\n");
    return 0;
}
```

保存为：

```
hello.c
```

使用 GCC 编译：

```
gcc hello.c -o hello
```

得到一个可执行文件：

```
hello
```

那么问题来了：

Linux 是如何知道 `hello` 是一个可以执行的程序，而不是一个普通的文本文件？

答案涉及两个概念：

1. 文件权限。
2. ELF 文件格式。

## 1. 文件权限：决定是否允许执行

首先查看文件权限：

```
ls -l hello
```

可能得到：

```
-rwxr-xr-x 1 user user 15960 Nov 10 10:00 hello
```

拆解一下：

```
-rwxr-xr-x
|
+-- 文件类型：- 表示普通文件
|
+-- rwx：文件所有者具有读、写、执行权限
|
+-- r-x：所属组具有读、执行权限
|
+-- r-x：其他用户具有读、执行权限
```

这里的 `x` 就表示执行权限。

例如：

```
chmod 644 hello
```

此时：

```
-rw-r--r--
```

即使 `hello` 的内容是一个完全合法的 ELF 程序，普通用户也无法通过：

```
./hello
```

正常执行它。

恢复执行权限：

```
chmod 755 hello
```

不过要注意：

执行权限不代表文件一定是可执行程序。

例如：

```
echo "Hello" > test.txt
chmod +x test.txt
./test.txt
```

这个文件虽然具有执行权限，但内核还需要找到能够处理它的可执行格式。普通文本文件通常不能直接作为 ELF 程序执行。

# 七、ELF：Linux 可执行文件的核心格式

ELF 全称：

Executable and Linkable Format（可执行与可链接格式）。

Linux 上常见的 ELF 文件包括：

- 可执行程序。
- 目标文件（`.o`）。
- 共享库（`.so`）。
- 内核模块（`.ko`）。

注意，它们虽然都是 ELF，但用途不同。

例如：

```
file hello
```

可能得到：

```
hello: ELF 64-bit LSB pie executable,
x86-64, dynamically linked,
interpreter /lib64/ld-linux-x86-64.so.2
```

这告诉我们：

- ELF：文件格式。
- 64-bit：64 位程序。
- LSB：小端字节序。
- PIE：位置无关可执行文件。
- x86-64：CPU 架构。
- dynamically linked：动态链接程序。
- interpreter：动态链接器的位置。

## 1. ELF 文件整体结构

一个 ELF 文件大致由以下部分组成：

[Introduction to ELF | COMS 4995 Advanced Systems Programming](https://images.openai.com/static-rsc-4/eeCISSE1PSIInF5--XYWPrfaq2l354aPx10oBZLvWa0Dm5Wgd1dI8LbDWsX1z_ng-VaRHdvRrtL_oKKVKyOnLtJ2ABl6Fqjv27GSKYrjH4ZHAixutzeBkB4NCpIDu6ZMATOksQ5gqWX0ltyOqPlVjiwEhxzDubQQWxEqgStHqnw?purpose=inline)

[cs4157.github.io](https://cs4157.github.io/www/2024-1/lect/15-elf-intro.html?utm_source=chatgpt.com)

```
+--------------------------------+
| ELF Header                     |
+--------------------------------+
| Program Header Table           |
+--------------------------------+
| .interp                        |
+--------------------------------+
| .text                          |
+--------------------------------+
| .rodata                        |
+--------------------------------+
| .data                          |
+--------------------------------+
| .bss                           |
+--------------------------------+
| Other Sections                 |
+--------------------------------+
| Section Header Table            |
+--------------------------------+
```

需要特别注意：

上面是帮助理解的简化结构，并不代表所有 ELF 文件都按照这个顺序排列，也不代表每个 ELF 都包含这些 section。

我们逐个理解。

## 2. ELF Header：文件头

ELF Header 是整个 ELF 文件的入口。

它描述：

- 文件是不是 ELF。
- 32 位还是 64 位。
- 使用什么字节序。
- 目标 CPU 架构。
- ELF 文件类型。
- 程序入口地址。
- Program Header Table 的位置。
- Section Header Table 的位置。

ELF 文件开头有一个非常重要的魔数：

```
7f 45 4c 46
```

对应 ASCII：

```
. E L F
```

所以 Linux 可以通过 ELF Header 判断文件是否符合 ELF 格式。

你可以使用：

```
readelf -h hello
```

查看 ELF Header。

## 3. .text：程序代码段

`.text` 通常保存程序的机器指令。

例如：

```
int main() {
    return 0;
}
```

经过 GCC 编译之后，会变成 CPU 可以执行的机器码。

概念上：

```
.text:

55 48 89 e5
b8 00 00 00 00
5d
c3
```

这些字节就是机器指令。

CPU 最终执行的不是 C 语言源代码，而是这些机器指令。

通常 `.text` section 会被映射到具有：

```
READ + EXECUTE
```

权限的内存区域。

也就是常见的：

```
r-x
```

通常不允许同时写入，以便配合内存保护机制。

## 4. .rodata：只读数据段

例如：

```
printf("Hello Linux!\n");
```

这里的字符串常量通常会被放在 `.rodata` 中。

概念上：

```
.rodata:

Hello Linux!\0
```

它一般会被映射为只读内存。

这样程序就可以读取字符串，但不能随意修改它。

## 5. .data：已初始化的全局变量和静态变量

例如：

```
int global_num = 100;

int main() {
    return 0;
}
```

`global_num` 是一个已初始化的全局变量。

通常会放在 `.data` 中。

其初始值 100 会存储在 ELF 文件里。

程序启动时，内核会将相关内容映射到进程的虚拟地址空间。

## 6. .bss：未初始化的全局变量和静态变量

例如：

```
int global_num;

int main() {
    return 0;
}
```

这里的 `global_num` 是未显式初始化的全局变量。

通常放在 `.bss` 中。

它有一个重要特点：

`.bss` 通常只记录需要分配的内存大小，不需要在 ELF 文件中为这些零值逐个保存数据。

程序加载时，对应的内存会被初始化为 0。

因此：

```
int a = 100;
int b;
```

在典型的 ELF 布局中：

```
a ---> .data
b ---> .bss
```

这也是为什么大量未初始化的全局变量不会让可执行文件体积按同样大小增长。

## 7. Program Header 和 Section Header 有什么区别？

这是 ELF 最容易让人混淆的地方。

ELF 中有两个重要的组织视角：

- Section：主要服务于编译、链接等环节。
- Segment：主要服务于程序加载和运行。

举个例子：

```
ELF Sections:

.text
.rodata
.data
.bss
```

加载程序时，内核并不需要逐个 section 去映射。

它主要读取 Program Header Table。

例如：

```
LOAD Segment 1
    .text
    .rodata

LOAD Segment 2
    .data
    .bss
```

实际一个 Segment 可以包含多个 Section。

Program Header 会描述：

- Segment 在 ELF 文件中的偏移。
- Segment 加载到进程虚拟地址空间的位置。
- Segment 在文件中的大小。
- Segment 在内存中的大小。
- Segment 的读写执行权限。

例如：

```
LOAD:
    Offset = 0x1000
    VirtAddr = 0x401000
    FileSiz = 0x500
    MemSiz  = 0x500
    Flags   = R E
```

这意味着：

文件中从 `0x1000` 开始的内容，会映射到程序虚拟地址空间中相应的位置，具有读取和执行权限。

另一个 Segment 可能是：

```
LOAD:
    Flags = R W
```

表示可读可写。

通过这种方式，Linux 能够把程序代码、只读数据和可写数据映射到不同的内存区域。

可以使用以下命令查看：

```
readelf -l hello
```

建议你重点观察 `LOAD` 类型的 Program Header。

# 八、Linux 如何执行一个 ELF 可执行文件？

这是前面所有知识的结合。

当你执行：

```
./hello
```

Shell 并不是直接读取文件内容，然后把机器码交给 CPU 执行。

它会通过 `execve()` 系统调用，请求 Linux 内核加载并执行这个程序。

## 1. execve 系统调用

Linux 中最核心的程序执行接口是：

```
#include <unistd.h>

int execve(
    const char *pathname,
    char *const argv[],
    char *const envp[]
);
```

例如：

```
char *argv[] = {
    "./hello",
    NULL
};

char *envp[] = {
    NULL
};

execve("./hello", argv, envp);
```

`execve()` 的三个参数：

| 参数       | 说明      |
| -------- | ------- |
| pathname | 可执行文件路径 |
| argv     | 命令行参数   |
| envp     | 环境变量    |

例如：

```
./hello arg1 arg2
```

对应的参数大致是：

```
argv[0] = "./hello"
argv[1] = "arg1"
argv[2] = "arg2"
```

注意：`execve()` 成功后不会返回到原来的程序代码，而是用新程序替换当前进程的用户空间执行映像。

它不会创建一个新的进程 PID。

如果想创建一个新进程再执行程序，通常会组合使用：

```
fork();
execve();
```

## 2. 内核执行 ELF 文件的过程

当调用：

```
execve("./hello", argv, envp);
```

大致经历以下过程：

[Part 5: Assemblers, Linkers & Loaders | Computer Architecture & OS Mastery - Wasil Zafar](https://images.openai.com/static-rsc-4/W1jDCP56h0Y-B3zpdrYV-BUXWSeBybnCqnCWRAtM1ixqn0bdlPlahO6BVrQ-PQiM5USO3BTOjHd3oU0OKIrgZOdbygvp-Jsxm4abifszu4ZdpOKqalnWCslhEcAfDRvAyK54k5TtvEsV-bxKIEX5XYLHRqqaJ4W5wznMg1qK-Pg?purpose=inline)

[wasilzafar.com](https://www.wasilzafar.com/pages/series/computer-architecture/comp-arch-linkers-loaders.html?utm_source=chatgpt.com)

```
应用程序
    |
    v
execve("./hello", argv, envp)
    |
    v
Linux 内核
    |
    v
检查文件访问权限
    |
    v
打开 ELF 文件
    |
    v
识别 ELF Header
    |
    v
解析 Program Header
    |
    v
建立新的进程虚拟地址空间
    |
    v
映射 LOAD Segments
    |
    v
准备栈、argv、envp
    |
    v
确定程序入口地址
    |
    v
跳转到程序入口
```

我们重点解释几个步骤。

### 第一步：检查文件

内核会检查：

- 文件是否存在。
- 当前用户是否具有执行权限。
- 文件格式是否受支持。
- ELF 架构是否与当前系统兼容。

例如：

```
chmod 644 hello
./hello
```

通常会得到：

```
Permission denied
```

如果你在 x86-64 Linux 上执行一个不受当前系统支持的 ELF 架构文件，也可能得到：

```
Exec format error
```

### 第二步：解析 ELF Header

内核识别 ELF 魔数：

```
7f 45 4c 46
```

然后读取 ELF Header，找到 Program Header Table。

接下来根据 Program Header 中的 `PT_LOAD` 条目，确定程序需要映射哪些内容。

### 第三步：建立虚拟地址空间

每个进程都有自己的虚拟地址空间。

例如：

[CSAPP Class Notes(3) - Wentao's Blog](https://images.openai.com/static-rsc-4/ZBzOVxhl5Sxf2u5YLn1WoGWIsd2ci25MlVNkNyLks3n6slW4kc-n-xtfjBjRm5jl-x7WWR6u8O1mWc_1HZijX7Jp4XS7-OGoax579MatSE8nq8k1JBBNbGHWn4PzGt1p8jN2BQjoKAcfvY4uRyQT38QJ5-xtbO1_MV-99Bg1NTU?purpose=inline)

[wentao.site](https://wentao.site/csapp_class_notes_3/?utm_source=chatgpt.com)

一个典型的进程虚拟地址空间可以抽象为：

```
高地址
+---------------------------+
|                           |
|          Stack            |
|          栈               |
|                           |
+---------------------------+
|                           |
|      mmap 映射区域         |
|                           |
|  共享库、文件映射等         |
|                           |
+---------------------------+
|                           |
|          Heap             |
|          堆               |
|                           |
+---------------------------+
|                           |
|          .bss             |
|                           |
+---------------------------+
|          .data            |
|                           |
+---------------------------+
|         .rodata           |
|                           |
+---------------------------+
|          .text            |
|                           |
+---------------------------+
低地址
```

这只是常见的概念布局，不是所有 Linux 进程都严格按照这个地址顺序排列。

另外，现代 Linux 通常启用 ASLR（地址空间布局随机化），不同进程的地址布局可能不同。

### 第四步：映射 ELF Segment

这是理解 Linux 程序加载的关键。

内核通常不会简单地把整个 ELF 文件复制到内存。

它会使用内存映射机制，将 ELF 文件中的 LOAD Segment 映射到进程的虚拟地址空间。

底层涉及：

```
mmap()
```

但这里需要注意：ELF 加载是内核内部的加载流程，并不是简单地在用户空间调用一次 `mmap()` 就完成了。

例如：

```
ELF 文件：

+----------------------+
| ELF Header           |
+----------------------+
| Program Header       |
+----------------------+
| .text                |
+----------------------+
| .rodata              |
+----------------------+
| .data                |
+----------------------+
```

内存映射之后：

```
进程虚拟地址空间：

+----------------------+
|                      |
| .text                |  r-x
|                      |
+----------------------+
| .rodata              |  r--
+----------------------+
| .data                |  rw-
+----------------------+
| .bss                 |  rw-
+----------------------+
```

这里又涉及一个非常重要的概念：

虚拟内存和物理内存并不是一一对应的。

例如，程序执行时，内核可以先建立文件映射，但不立即把所有数据加载到物理内存。

当 CPU 真正访问某个虚拟地址时，如果对应页面还没有建立物理映射，就可能触发 Page Fault（缺页异常）。

内核再根据映射关系从 Page Cache 获取数据，必要时从磁盘读取。

因此，一个大型可执行文件即使有几百 MB，也不意味着启动时必须把几百 MB 全部读入物理内存。

这就是 Demand Paging（按需分页）的思想。

### 第五步：准备程序栈

内核还需要准备程序启动所需的信息。

例如：

```
argc
argv
envp
auxiliary vector
```

其中 Auxiliary Vector（辅助向量）会向程序提供一些启动信息，例如：

- 程序入口相关信息。
- 系统页面大小。
- 动态链接器相关信息。
- 随机化数据。

这些信息会放在新进程的初始栈区域附近。

### 第六步：跳转到程序入口

ELF Header 中存在一个非常重要的字段：

```
e_entry
```

它表示程序入口地址。

但这里要注意一个细节：

对于动态链接程序，内核通常首先将控制权交给 ELF 指定的动态链接器，而不一定直接进入我们编写的 `main()`。

# 九、动态链接器：程序为什么还需要加载共享库？

我们刚才编写的程序：

```
#include <stdio.h>

int main() {
    printf("Hello Linux!\n");
    return 0;
}
```

虽然我们只写了一行 `printf()`，但是它背后涉及 C 标准库。

那么 `printf()` 的代码究竟在哪里？

通常在共享库中，例如：

```
/lib/x86_64-linux-gnu/libc.so.6
```

可以通过以下命令查看程序依赖：

```
ldd hello
```

可能输出：

```
linux-vdso.so.1
libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6
/lib64/ld-linux-x86-64.so.2
```

其中：

| 文件                   | 作用                 |
| -------------------- | ------------------ |
| linux-vdso.so.1      | 内核提供给用户空间的特殊虚拟共享对象 |
| libc.so.6            | GNU C 标准库          |
| ld-linux-x86-64.so.2 | 动态链接器              |

## 1. 动态链接的执行过程

如果 ELF 文件是动态链接的，它通常包含一个特殊的 Program Header：

```
PT_INTERP
```

它描述动态链接器的路径。

例如：

```
/lib64/ld-linux-x86-64.so.2
```

启动过程可以理解为：

```
执行 ./hello
      |
      v
Linux 内核加载 hello
      |
      v
发现 PT_INTERP
      |
      v
加载动态链接器
      |
      v
动态链接器加载 libc.so.6 等依赖库
      |
      v
完成符号解析和必要的重定位
      |
      v
进入程序初始化流程
      |
      v
调用 main()
      |
      v
printf("Hello Linux!")
```

所以，实际上：

```
main()
```

并不是程序最开始执行的地方。

在进入 `main()` 之前，通常还有：

- ELF 加载。
- 动态链接。
- C 运行时初始化。
- 全局构造函数等初始化工作。

## 2. 动态链接和静态链接的区别

我们可以使用 GCC 编译两种程序：

动态链接：

```
gcc hello.c -o hello
```

静态链接：

```
gcc -static hello.c -o hello_static
```

两者区别：

| 对比    | 动态链接          | 静态链接                          |
| ----- | ------------- | ----------------------------- |
| 共享库   | 运行时加载         | 通常编译时链接进程序                    |
| 文件大小  | 通常较小          | 通常较大                          |
| 启动依赖  | 需要相应动态链接器和共享库 | 不需要这些动态库                      |
| 更新共享库 | 可能影响程序        | 通常不受共享库更新影响                   |
| 内存共享  | 多个进程可共享库的代码页  | 也可能通过文件映射等方式共享，但不具有动态共享库的更新机制 |

这里的静态链接并不代表程序完全不依赖操作系统或内核，只是它不需要在运行时通过动态链接器加载这些普通共享库。

# 十、文本文件和可执行文件究竟有什么区别？

现在把前面学到的知识放在一起。

假设我们有两个文件：

```
hello.txt
hello
```

它们都属于 Linux 文件系统中的普通文件。

但它们的内容和使用方式不同。

## 1. 从文件系统角度比较

| 对比项目       | hello.txt      | hello        |
| ---------- | -------------- | ------------ |
| 文件类型       | 普通文件           | 普通文件         |
| inode      | 有              | 有            |
| 数据块        | 有（可能使用内联数据等优化） | 有            |
| 文件名        | 目录项保存          | 目录项保存        |
| 权限         | 通常不需要执行权限      | 通常需要执行权限     |
| 文件格式       | 没有统一要求         | 通常是 ELF      |
| 内容         | 字符编码后的字节       | 机器指令及相关数据    |
| 文件系统是否理解内容 | 不理解            | 不负责解析 ELF 语义 |

这里有一个非常重要的结论：

在文件系统层面，文本文件和可执行文件没有本质区别，它们都是普通文件。

ext4 不会因为文件里面是 C 语言机器码，就给它使用另一套 inode 或磁盘数据结构。

文件系统负责存储数据，内核的可执行文件加载机制负责解释 ELF 格式。

## 2. 从内核使用方式比较

假设执行：

```
cat hello.txt
```

内核主要做：

```
open()
    |
    v
read()
    |
    v
读取文件数据
    |
    v
write(1, ...)
```

而执行：

```
./hello
```

内核主要做：

```
execve()
    |
    v
检查权限和文件格式
    |
    v
解析 ELF
    |
    v
映射程序 Segment
    |
    v
建立进程虚拟地址空间
    |
    v
准备运行环境
    |
    v
执行程序
```

这是两者最大的区别。

## 3. 一个特别有意思的实验

我们可以用同一个文件，观察 Linux 对它的不同处理。

首先创建文本文件：

```
echo "Hello Linux" > test
```

查看：

```
file test
```

输出类似：

```
test: ASCII text
```

然后赋予执行权限：

```
chmod +x test
```

尝试执行：

```
./test
```

通常会得到：

```
./test: line 1: Hello: command not found
```

这里发生了什么？

这次其实是 Shell 对无法直接执行的文件进行了脚本回退处理，尝试把它当作 Shell 脚本解释。

如果使用 C 程序的 `execve()` 直接执行一个不支持的普通文本文件，通常会得到 `ENOEXEC`，而不会自动把它当作 Shell 脚本执行。

这说明：

- 文件系统只知道这是普通文件。
- 文件具有执行权限。
- 内核尝试识别可执行文件格式。
- Shell 可以在特定情况下对普通文本进行解释。

再创建一个真正的 ELF 文件：

```
gcc hello.c -o hello
chmod +x hello
```

执行：

```
./hello
```

此时内核可以识别 ELF 格式，并进入程序加载流程。

# 十一、如何通过 Linux 命令实际观察这些机制？

前面讲了很多原理。下面给你一组实验命令，可以直接在 Linux 环境中运行。

建议使用 Ubuntu 或 Debian。

## 1. 观察文件的 inode 和元数据

创建一个文本文件：

```
echo "Hello Linux" > hello.txt
```

查看 inode：

```
ls -i hello.txt
```

查看文件详细属性：

```
stat hello.txt
```

重点关注：

```
File:
Size:
Inode:
Links:
Access:
Modify:
Change:
```

它们分别表示文件名称、逻辑大小、inode 编号、硬链接数量、权限，以及相关时间信息。

## 2. 观察文件的实际磁盘占用

```
du -h hello.txt
```

查看文件逻辑大小：

```
ls -l hello.txt
```

查看文件系统块大小：

```
stat -f hello.txt
```

你会发现文件逻辑大小和实际占用空间不一定相同。

## 3. 观察 ELF 文件结构

编译：

```
gcc hello.c -o hello
```

查看文件类型：

```
file hello
```

查看 ELF Header：

```
readelf -h hello
```

查看 Program Header：

```
readelf -l hello
```

查看 Section Header：

```
readelf -S hello
```

查看符号表：

```
readelf -s hello
```

查看反汇编：

```
objdump -d hello
```

这几个命令非常值得掌握。

## 4. 观察程序执行过程

使用：

```
strace ./hello
```

`strace` 可以跟踪程序使用的系统调用。

你可能会看到类似：

```
execve("./hello", ...)
openat(...)
mmap(...)
mprotect(...)
write(1, "Hello Linux!\n", 13)
exit_group(0)
```

注意，具体系统调用及顺序会因 Linux 版本、动态链接方式、运行环境等因素而不同。

其中：

- `execve()`：请求执行程序。
- `openat()`：打开文件。
- `mmap()`：建立内存映射。
- `mprotect()`：调整内存区域权限。
- `write()`：向标准输出写入内容。
- `exit_group()`：结束线程组。

通过 `strace`，你能够直观地观察一个程序从启动到结束的系统调用行为。

## 5. 观察进程的内存映射

先执行：

```
./hello
```

如果程序执行太快，可以使用一个持续运行的程序：

```
#include <stdio.h>
#include <unistd.h>

int main() {
    printf("PID: %d\n", getpid());
    fflush(stdout);

    sleep(1000);

    return 0;
}
```

编译并运行：

```
gcc test.c -o test
./test
```

另一个终端执行：

```
cat /proc/<PID>/maps
```

你会看到类似：

```
00400000-00401000 r--p ... /path/test
00401000-00402000 r-xp ... /path/test
00402000-00403000 r--p ... /path/test
00403000-00404000 rw-p ... /path/test

7fxxxxxxxxxx-7fxxxxxxxxxx r-xp ... libc.so.6
7fxxxxxxxxxx-7fxxxxxxxxxx rw-p ... libc.so.6
```

这里能看到：

- ELF 文件的不同内存映射区域。
- 代码段的执行权限。
- 数据段的读写权限。
- 动态链接库的映射区域。

这一步可以把前面讲的 ELF Segment、虚拟内存、动态链接器联系起来。

# 十二、最后建立一张完整的知识地图

把 Linux 文件系统相关的知识串起来，可以得到下面这张图：

## Linux 文件系统知识体系

应用程序

cat、gcc、./hello、编辑器

系统调用

openat、read、write、execve、mmap

VFS（虚拟文件系统）

统一文件访问接口

路径管理

dentry / inode

文件访问

file / fd / f_op

缓存管理

Page Cache

内存映射

mmap / Page Fault

具体文件系统

ext4 / XFS / Btrfs

磁盘数据组织

Superblock / Inode Table / Data Blocks / Extents

块设备和物理存储

Block Layer / Driver / SSD / HDD

还有一条与文件系统存储相关、但属于另一条内核路径的知识线：

```
可执行文件
    |
    v
ELF Header
    |
    v
Program Header
    |
    v
execve()
    |
    v
ELF Loader
    |
    v
动态链接器
    |
    v
虚拟地址空间
    |
    v
CPU 执行机器指令
```

## 十三、我建议你接下来重点学习的内容

如果你的目标是深入理解 Linux 内核，而不仅仅是会使用 Linux 文件系统，我建议按照下面的顺序继续：

| 学习顺序 | 知识点               | 重要程度 |
| ---- | ----------------- | ---- |
| 1    | inode、dentry、file | 非常高  |
| 2    | VFS 的设计与实现        | 非常高  |
| 3    | Page Cache 和文件读写  | 非常高  |
| 4    | mmap 与虚拟内存        | 非常高  |
| 5    | ELF 文件格式          | 非常高  |
| 6    | execve 的内核实现      | 非常高  |
| 7    | ext4 的磁盘布局和日志机制   | 高    |
| 8    | 文件系统崩溃恢复与一致性      | 高    |

尤其推荐你深入研究 `inode`、`dentry`、`file` 和 `Page Cache`。 这几个概念是理解 Linux 文件系统源码的基础。

如果继续往 Linux 内核源码层面学习，可以从 `fs/open.c`、`fs/read_write.c`、`fs/namei.c` 和 `fs/binfmt_elf.c` 这几个文件入手，分别研究文件打开、读写、路径解析和 ELF 加载的实际实现。