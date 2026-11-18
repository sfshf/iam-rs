# 问：给我分别详细讲一下TCP、UDP、Quic协议的设计

ChatGPT答：

TCP、UDP 和 QUIC 是现代网络通信中非常重要的三种传输层协议（QUIC 严格来说运行在 UDP 之上，但它实现了完整的可靠传输能力）。

理解这三个协议，不能只停留在它们的区别上，更重要的是理解：它们分别解决了什么问题、采用了什么设计思想、内部如何工作，以及为什么会出现这样的设计。

我会从网络协议设计的角度展开，尽量结合实际网络通信、数据结构、状态机和工程实现来讲。

# 一、先建立整体认识

先看它们在网络协议栈中的位置。

## TCP 传统协议栈

应用层

HTTP / HTTPS / SMTP / SSH

传输层：TCP

可靠传输、流量控制、拥塞控制、连接管理

网络层：IP

寻址、路由、数据包转发

数据链路层 / 物理层

## UDP 传统协议栈

应用层

DNS / RTP / QUIC / 游戏通信

传输层：UDP

端口复用、数据报传输、校验和

网络层：IP

数据链路层 / 物理层

## QUIC 协议栈

应用层

HTTP/3 等应用协议

QUIC

可靠传输、多路复用、拥塞控制、TLS 1.3、安全连接迁移

UDP

提供数据报传输能力

IP

这里有一个非常重要的认知：

- TCP：在操作系统内核中实现可靠的字节流传输。
- UDP：只提供尽可能简单的数据报传输，不负责可靠性。
- QUIC：在 UDP 之上重新设计了一套可靠、安全、多路复用的传输机制。

QUIC 并不是简单地给 UDP 增加一个可靠性开关，而是重新设计了连接管理、数据传输、丢包恢复、拥塞控制和加密协商等机制。

# 二、TCP 协议的设计

## 2.1 TCP 的设计目标

TCP（Transmission Control Protocol，传输控制协议）的核心目标是：

在不可靠的 IP 网络之上，为应用程序提供可靠、有序、无重复的字节流。

为什么需要 TCP？

因为 IP 网络本身并不保证：

1. 数据包一定能到达目的地。
2. 数据包一定按照发送顺序到达。
3. 数据包不会重复。
4. 数据包不会损坏。
5. 数据包不会因为网络拥堵而丢失。

假设应用程序需要发送：

```
Hello World
```

IP 层可能发生：

```
发送顺序：
Packet A -> Packet B -> Packet C

实际到达：
Packet A -> Packet C -> Packet B
```

也可能发生：

```
发送顺序：
Packet A -> Packet B -> Packet C

实际到达：
Packet A -> Packet C
```

TCP 要解决的就是这些问题。

## 2.2 TCP 的核心设计：字节流

TCP 最重要的设计是：它提供的是字节流，而不是消息流。

例如应用程序连续执行：

```
conn.Write([]byte("Hello"))
conn.Write([]byte("World"))
```

接收端可能出现以下情况：

```
buf := make([]byte, 1024)

n, _ := conn.Read(buf)
fmt.Println(string(buf[:n]))
```

可能得到：

```
HelloWorld
```

也可能得到：

```
Hello
```

下一次读取：

```
World
```

甚至可能得到：

```
Hel
```

下一次：

```
loWor
```

再下一次：

```
ld
```

TCP 不会保留应用程序每次 `Write()` 的边界。

这是 TCP 和 UDP 一个非常重要的区别。

| 特征   | TCP      | UDP |
| ---- | -------- | --- |
| 数据模型 | 字节流      | 数据报 |
| 消息边界 | 不保留      | 保留  |
| 可靠性  | 保证可靠传输语义 | 不保证 |
| 顺序   | 按序交付     | 不保证 |
| 重复数据 | TCP 层去重  | 不保证 |

所以使用 TCP 开发应用层协议时，通常需要自己设计消息边界。

常见方法：

- 固定长度。
- 分隔符。
- 长度前缀。
- TLV 编码。

例如长度前缀协议：

```
+----------------+----------------------+
| Length: 4 bytes| Payload: Length bytes|
+----------------+----------------------+
```

## 2.3 TCP 报文头设计

TCP 通过 TCP Header 管理传输过程。

[How Many Bits are Reserved for the TCP Flags? - GeeksforGeeks](https://images.openai.com/static-rsc-4/LShozYgrfq79eG__Wkdl5iQUpujqUDh1QwvRwxSr9_3WqDOiBSa2Qrm8iL5hajdGPl9p_mCHQY_qgalVg0qM19flRpMKfdfHPJwuoFNvuW1h6MDh7VlI_7mKNJtoqWgHYSJVE0MzEVufIjey82j4RNHTpH99LpysCxsuGmbZ8kQ?purpose=inline)

[geeksforgeeks.org](https://www.geeksforgeeks.org/computer-networks/how-many-bits-are-reserved-for-the-tcp-flags/?utm_source=chatgpt.com)

TCP Header 最小长度是 20 字节。

主要字段如下：

| 字段                    | 长度     | 作用      |
| --------------------- | ------ | ------- |
| Source Port           | 16 bit | 源端口     |
| Destination Port      | 16 bit | 目标端口    |
| Sequence Number       | 32 bit | 序列号     |
| Acknowledgment Number | 32 bit | 确认号     |
| Data Offset           | 4 bit  | TCP 头长度 |
| Flags                 | 9 bit  | 控制标志    |
| Window                | 16 bit | 接收窗口    |
| Checksum              | 16 bit | 校验和     |
| Urgent Pointer        | 16 bit | 紧急指针    |
| Options               | 可变     | 扩展选项    |

其中最重要的是：

- Sequence Number
- Acknowledgment Number
- Window
- Flags

### Sequence Number：序列号

TCP 使用序列号标记字节流中的每一个字节。

假设发送端发送：

```
Hello
```

假设第一个字节的序列号为 1000：

```
H -> 1000
e -> 1001
l -> 1002
l -> 1003
o -> 1004
```

如果接收端成功收到全部数据，它会返回：

```
ACK = 1005
```

这代表：

接收端期望下一个收到的字节序列号是 1005。

注意，TCP 的 ACK 采用累计确认机制。

如果发送端发送：

```
Packet A: Seq=1000, Len=100
Packet B: Seq=1100, Len=100
Packet C: Seq=1200, Len=100
```

接收端如果收到 A 和 C，但是没有收到 B，那么它仍然只能确认：

```
ACK = 1100
```

因为从 1100 开始的字节还没有连续到达。

这也是 TCP 后续需要 SACK 等机制优化丢包恢复效率的原因。

## 2.4 TCP 如何建立连接：三次握手

TCP 使用三次握手（Three-Way Handshake）建立连接。

[Understanding the OSI Model: Layer 4 to Layer 7 | Arsh Sharma](https://images.openai.com/static-rsc-4/-hRw9xIOigFryEatPkjJ9Kw-8gOihp1vkJfd2kaeuG3dKSmt8E0szonvAm3XDqtUx_o1fClFssn4dHjQDIjuFAqlRNi75S40To2V448ZkcsFeyl6nFf-PZh8mriR_pZ_Bf_Y8BgX4mJIDonXiLblBg5O_zw5AYJnKEX7u9S5Gnc?purpose=inline)

[arshsharma.com](https://arshsharma.com/posts/2024-11-28-osi-layer-four-seven/?utm_source=chatgpt.com)

假设：

```
Client: 192.168.1.10:50000
Server: 192.168.1.20:8080
```

整个过程：

Client

主动连接方

Server

SYN

Seq = x

LISTEN

SYN-SENT

SYN + ACK

Seq = y, ACK = x + 1

ACK

ACK = y + 1

ESTABLISHED

第三次握手完成后，客户端和服务端都进入 ESTABLISHED 状态。

具体来说：

第一次握手：客户端发送 SYN

```
SYN = 1
Seq = x
```

客户端告诉服务端：

- 我想建立 TCP 连接。
- 我的初始序列号是 x。
- 我支持的 TCP 参数可能包含 MSS、Window Scale、SACK Permitted 等。

客户端进入：

```
SYN-SENT
```

第二次握手：服务端发送 SYN + ACK

```
SYN = 1
ACK = 1
Seq = y
Ack = x + 1
```

服务端告诉客户端：

- 我收到了你的 SYN。
- 我的初始序列号是 y。
- 我也希望建立连接。

服务端进入：

```
SYN-RECEIVED
```

第三次握手：客户端发送 ACK

```
ACK = 1
Seq = x + 1
Ack = y + 1
```

服务端收到这个 ACK 后，双方进入：

```
ESTABLISHED
```

### 为什么必须三次握手？

主要是为了让通信双方确认彼此的发送和接收能力，同时同步初始序列号。

例如：

- 第一次握手：客户端确认自己能够发送，服务端能够接收。
- 第二次握手：服务端确认自己能够发送，客户端能够接收。
- 第三次握手：服务端确认客户端确实收到了自己的 SYN。

此外，三次握手还可以避免历史延迟 SYN 报文造成错误连接建立。

## 2.5 TCP 如何保证可靠性？

TCP 的可靠性并不是依赖某一个机制，而是多个机制共同实现的。

### 机制一：序列号

每个 TCP 字节都有对应的序列号。

作用：

- 判断数据顺序。
- 识别重复数据。
- 识别丢失的数据。
- 支持接收端重新组装数据。

### 机制二：ACK 确认

接收端收到数据后，通过 ACK 告诉发送端已经连续收到哪些字节。

例如：

```
发送端                  接收端

Seq=1000, Len=100  --->
                    <--- ACK=1100

Seq=1100, Len=100  --->
                    <--- ACK=1200
```

### 机制三：超时重传

发送端发送数据后，如果在一定时间内没有收到确认，就会重传数据。

例如：

```
Client                 Server

DATA ----------------->
                       X (丢失)

等待 ACK

超时

DATA ----------------->
                       ACK -------->
```

这里涉及 RTO（Retransmission Timeout）。

RTO 并不是固定值，而是根据 RTT（Round Trip Time）动态估算。

TCP 会使用平滑 RTT 和 RTT 偏差来计算重传超时时间，并且对连续超时进行退避。

### 机制四：快速重传

TCP 还可以通过重复 ACK 判断数据可能丢失。

经典的快速重传机制是：

收到 3 个重复 ACK 后，提前重传缺失的数据，而不是等待 RTO 超时。

例如：

```
发送：
A -> B -> C -> D

接收：
A -> B -> D

返回：
ACK(B之后的位置)
ACK(B之后的位置)
ACK(B之后的位置)
```

发送端发现重复 ACK 达到阈值，就可以提前重传 C。

### 机制五：SACK

SACK（Selective Acknowledgment）允许接收端告诉发送端：

哪些不连续的数据块已经收到。

例如：

```
发送：
A B C D E

接收：
A B   D E
```

接收端除了累计 ACK 外，还可以通过 SACK 告诉发送端：

```
D 和 E 已经收到。
C 还没有收到。
```

这样发送端就可以只重传 C，而不必重复发送已经成功到达的数据。

## 2.6 TCP 流量控制

可靠传输不代表接收端一定有能力无限接收数据。

假设：

```
发送端速度：100 MB/s
接收端处理速度：10 MB/s
```

如果不限制发送端，接收端缓冲区最终就会被填满。

因此 TCP 设计了流量控制（Flow Control）。

核心机制：滑动窗口（Sliding Window）。

接收端通过 TCP Header 中的 Window 字段告诉发送端：

```
我目前还能接收多少数据。
```

例如：

```
Receive Window = 64 KB
```

表示接收端当前通告的接收窗口为 64 KB。

发送端就需要控制未确认数据量，不能超过接收窗口限制。

这里还有一个非常重要的概念：

接收窗口 rwnd 和拥塞窗口 cwnd 不是一回事。

- rwnd：接收端的接收能力。
- cwnd：发送端根据网络拥塞情况估算的网络承载能力。

TCP 实际允许发送的数据量受两者共同限制：

\\[ SendWindow = \min(rwnd,cwnd) \\]

这就引出了下一个机制。

## 2.7 TCP 拥塞控制

流量控制解决的是接收端处理能力问题。

拥塞控制解决的是网络整体承载能力问题。

例如：

```
Client -> Router A -> Router B -> Server
```

假设 Router B 的转发能力只有 10 MB/s，而客户端以 100 MB/s 的速度发送数据。

Router B 的队列可能越来越长，最终发生丢包。

TCP 需要动态调整发送速率。

经典 TCP 拥塞控制主要包含以下几个阶段：

1. 慢启动（Slow Start）。
2. 拥塞避免（Congestion Avoidance）。
3. 快速重传（Fast Retransmit）。
4. 快速恢复（Fast Recovery）。

[\[컴퓨터 네트워크\] TCP 혼잡 제어(Congestion Control)](https://images.openai.com/static-rsc-4/PXyL4oL5oa4T-U5j68IQQWVKDWKtXUba-wYbxk6ps3B4IVVs0-lEAbL9Cq7g1PCH97vQE967lID7ggX96dgDAnIie-vu0chIXrc3W-OGS-swsug49ihylaG2REerfevyDNGcxuyIGzabXQ-WYK9EX61g18AgqXALseAaSObaBw8?purpose=inline)

[jjunsu.tistory.com](https://jjunsu.tistory.com/358?utm_source=chatgpt.com)

### 慢启动

刚开始建立连接时，TCP 并不知道网络能够承载多少数据。

因此先从较小的拥塞窗口开始。

随着 ACK 不断到达，cwnd 通常呈指数增长（以 RTT 为单位观察）。

### 拥塞避免

当 cwnd 达到慢启动阈值 ssthresh 后，进入拥塞避免阶段。

此时 cwnd 通常以每 RTT 大约一个 MSS 的速度线性增长。

### 发生拥塞

传统 Reno 算法中，检测到丢包后会降低拥塞窗口。

经典 TCP 的拥塞控制并不是唯一实现。

例如：

- Reno
- CUBIC
- BBR

Linux 上常见的默认拥塞控制算法长期以来是 CUBIC，具体取决于系统配置。

## 2.8 TCP 如何关闭连接：四次挥手

TCP 是全双工通信。

客户端和服务端两个方向的数据传输可以独立关闭。

因此关闭连接通常需要四次挥手。

[TCP TIME_WAIT 过多怎么处理-腾讯云开发者社区-腾讯云](https://images.openai.com/static-rsc-4/e7YF1EvHWaccXWMwO-3E4dR3Rvt1chlz9hlsT9QQLzjkNkTmerV3d6Q2BpGp8pwmLmjQuREqM9moZwSD2VK-NusHGO7yEnAM-yNQEgPNGSx3taquRYwtPXQcaahD0SKDyYT6MHFKa6ZjqTX46_XFval6JfXfZAm1M-KOrgKW3e8?purpose=inline)

[cloud.tencent.com](https://cloud.tencent.com/developer/article/2387102?utm_source=chatgpt.com)

例如客户端主动关闭连接：

```
Client                     Server

FIN --------------------->

       <------------------ ACK

       <------------------ FIN

ACK --------------------->
```

对应过程：

1. 客户端发送 FIN，表示客户端不再发送数据。
2. 服务端返回 ACK。
3. 服务端处理完剩余数据后发送 FIN。
4. 客户端返回 ACK。

这里涉及两个非常重要的状态：

FIN_WAIT_2

客户端已经收到对方对自己 FIN 的 ACK，但还没有收到对方的 FIN。

TIME_WAIT

主动关闭连接的一方通常进入 TIME_WAIT。

TIME_WAIT 的主要作用：

- 确保最后一个 ACK 有机会被对方收到。
- 让旧连接中的延迟报文在网络中自然消失，避免干扰后续连接。

常见情况下，TIME_WAIT 持续 2MSL。

# 三、UDP 协议的设计

## 3.1 UDP 的设计目标

UDP（User Datagram Protocol，用户数据报协议）的设计思想和 TCP 完全不同。

TCP 希望提供可靠的字节流。

UDP 则希望提供一种简单、低开销、无连接的数据报传输方式。

UDP 的设计目标可以概括为：

尽可能少地干预应用程序的数据传输过程。

UDP 不负责：

- 建立连接。
- 保证数据到达。
- 保证数据顺序。
- 自动重传。
- 流量控制。
- 拥塞控制。

这些机制如果需要，可以由应用层自行实现。

## 3.2 UDP 报文结构

[TCP vs UDP: When to Use What, and How TCP Relates to HTTP - DEV Community](https://images.openai.com/static-rsc-4/qnZtrL-qKAS24a9X2_pEeykb9drRNf5mnVATZ9vVTzbljp6UwJ9er-eyLpkZtcDgAW4LG5HdcCMkZTwtzOQKdr0VuTjQmHCPCN-ufxKTM0BD5efqgegrha29qQED-83N6RmOI99GVbSI0Xgl4rk6AZALCK286jPWy9DTgKIXDaE?purpose=inline)

[dev.to](https://dev.to/ritam369/tcp-vs-udp-when-to-use-what-and-how-tcp-relates-to-http-4144?utm_source=chatgpt.com)

UDP Header 只有 8 字节。

| 字段               | 长度     | 作用         |
| ---------------- | ------ | ---------- |
| Source Port      | 16 bit | 源端口        |
| Destination Port | 16 bit | 目标端口       |
| Length           | 16 bit | UDP 数据报总长度 |
| Checksum         | 16 bit | 校验和        |

UDP 的设计非常简单。

没有：

- 序列号。
- 确认号。
- 滑动窗口。
- 拥塞窗口。
- 连接状态。

这意味着 UDP 的协议处理开销相对较小。

但要注意，UDP 并不是完全没有校验机制。UDP Checksum 可以检测传输中的数据损坏，IPv4 下校验和可以为零表示不使用，而 IPv6 中通常必须使用。

## 3.3 UDP 的核心设计：数据报

UDP 和 TCP 最大的区别之一，就是 UDP 保留消息边界。

例如：

发送端：

```
conn.Write([]byte("Hello"))
conn.Write([]byte("World"))
```

接收端：

```
buf := make([]byte, 1024)

n, _ := conn.Read(buf)
fmt.Println(string(buf[:n]))
```

通常会分别收到：

```
Hello
```

和：

```
World
```

每次 UDP Write 对应一个 UDP 数据报，而不是像 TCP 一样形成连续字节流。

当然，UDP 数据报也可能丢失、乱序、重复。

另外，如果接收缓冲区太小，数据报可能被截断。

## 3.4 UDP 为什么不需要三次握手？

因为 UDP 本身没有连接建立过程。

例如：

```
conn, err := net.DialUDP(
    "udp",
    nil,
    remoteAddr,
)
```

这里的 `DialUDP` 不代表进行了网络层面的连接握手。

它主要是在本地创建一个 UDP socket，并设置默认远端地址等信息。

实际上，UDP 可以直接发送数据：

```
conn.Write([]byte("Hello"))
```

接收端也可以直接接收数据：

```
n, addr, err := conn.ReadFromUDP(buf)
```

因此 UDP 非常适合短小、频繁、对延迟敏感的通信场景。

## 3.5 UDP 为什么适合实时通信？

考虑一个视频会议场景。

假设你正在发送实时语音数据：

```
Packet 1：10:00:00.000
Packet 2：10:00:00.020
Packet 3：10:00:00.040
Packet 4：10:00:00.060
```

如果 Packet 2 丢失了。

使用 TCP：

```
Packet 1 -> Packet 2(丢失) -> Packet 3 -> Packet 4
```

由于 TCP 保证有序交付，Packet 3 和 Packet 4 即使已经到达，也需要等待 Packet 2 补齐后才能交给应用程序。

这就是 TCP 的队头阻塞（Head-of-Line Blocking）。

对于实时语音来说，等到 Packet 2 重传成功，可能已经没有意义了。

因为：

```
当前播放时间：10:00:00.100
Packet 2 的播放时间：10:00:00.020
```

这段语音已经过期。

因此实时通信可能更倾向于：

```
Packet 1 -> 正常播放
Packet 2 -> 丢弃
Packet 3 -> 正常播放
Packet 4 -> 正常播放
```

这就是 UDP 在实时音视频、在线游戏等场景中的重要价值。

## 3.6 UDP 的可靠性应该由谁实现？

UDP 不保证可靠性，但并不意味着 UDP 应用程序不能实现可靠传输。

例如 QUIC 就是在 UDP 之上实现可靠传输。

假设你自己基于 UDP 实现一个可靠传输协议，可能需要设计：

| 机制                 | 作用    |
| ------------------ | ----- |
| Sequence Number    | 数据包编号 |
| ACK                | 接收确认  |
| Retransmission     | 超时重传  |
| Sliding Window     | 滑动窗口  |
| Congestion Control | 拥塞控制  |
| Flow Control       | 流量控制  |
| Connection ID      | 连接标识  |
| Encryption         | 数据加密  |

你会发现：

这几乎就是重新设计一个 TCP。

而 QUIC 正是对这类问题的一套成熟解决方案。

# 四、QUIC 协议的设计

QUIC 是三种协议中设计思想最现代化、也最值得深入研究的一个。

它由 Google 最初推动，后来经过 IETF 标准化，主要目标是解决传统 TCP 在现代互联网环境中的一些性能和架构问题。

QUIC 的标准定义是：

QUIC：UDP-Based Multiplexed and Secure Transport。

核心标准包括：

- RFC 9000：QUIC Transport。
- RFC 9001：Using TLS to Secure QUIC。
- RFC 9002：QUIC Loss Detection and Congestion Control。

## 4.1 为什么需要 QUIC？

先看传统 HTTPS 的通信过程。

HTTP/2 通常使用：

```
HTTP/2
   |
 TLS 1.2 / TLS 1.3
   |
  TCP
   |
   IP
```

客户端建立 HTTPS 连接，需要完成：

```
TCP 三次握手
       |
TLS 握手
       |
HTTP 请求
       |
HTTP 响应
```

假设网络 RTT 是 100ms。

使用 TCP + TLS 1.3 建立一个全新的 HTTPS 连接，通常需要：

```
TCP 握手：1 RTT
TLS 1.3 握手：1 RTT
HTTP 请求：等待服务端处理
```

也就是在发送正常 HTTP 请求之前，通常需要约 2 RTT 的建连时间。

这在网络延迟比较高的环境中会比较明显。

此外，TCP + HTTP/2 还有一个比较重要的问题：队头阻塞。

### TCP 的队头阻塞问题

假设 HTTP/2 在一条 TCP 连接上同时传输三个 Stream：

```
TCP Connection
    |
    +--- HTTP Stream A
    |
    +--- HTTP Stream B
    |
    +--- HTTP Stream C
```

它们在应用层是相互独立的。

但是最终都要经过同一个 TCP 字节流。

假设 TCP 数据包发生丢失：

```
TCP Packet 1：包含 A 的数据
TCP Packet 2：包含 B 的数据
TCP Packet 3：包含 C 的数据
TCP Packet 4：包含 A 的数据
```

如果 Packet 2 丢失：

```
Packet 1 -> Packet 2(丢失) -> Packet 3 -> Packet 4
```

即使 Packet 3 和 Packet 4 已经到达，TCP 也不会把它们交给上层应用，直到缺失的数据被补齐。

于是：

```
Stream A：可能受影响
Stream B：受影响
Stream C：也受影响
```

这就是 TCP 层的队头阻塞。

注意：这是 TCP 字节流层面的阻塞，不是说 HTTP/2 本身没有多路复用能力。

QUIC 的设计就是要从传输层解决这个问题。

## 4.2 QUIC 的核心设计思想

QUIC 并没有直接修改 TCP，而是在 UDP 上重新实现了一套传输协议。

它的主要设计目标：

1. 降低连接建立延迟。
2. 消除不同应用数据流之间的传输层队头阻塞。
3. 默认集成 TLS 1.3 加密。
4. 支持连接迁移。
5. 改进丢包检测和拥塞控制。
6. 允许协议在用户态实现，减少对操作系统内核的依赖。

QUIC 最重要的设计可以概括为：

Connection + Stream + Packet + Frame。

理解这四个概念，就理解了 QUIC 的主体架构。

## 4.3 Connection：连接

TCP 通过四元组识别一条连接：

```
Source IP
Source Port
Destination IP
Destination Port
```

例如：

```
192.168.1.10:50000
        |
        v
192.168.1.20:443
```

但是在移动互联网中，客户端的网络环境可能变化。

例如：

```
WiFi：
192.168.1.10:50000

切换到 4G：

10.10.1.20:60000
```

对于传统 TCP：

四元组已经发生变化，原来的 TCP 连接通常无法继续使用。

QUIC 为了解决这个问题，引入了 Connection ID（连接 ID）。

QUIC Connection ID 用于识别连接，而不只是依赖 IP 地址和端口。

例如：

```
原来的网络路径：

192.168.1.10:50000
        |
        v
192.168.1.20:443

Connection ID = ABC123
```

切换网络后：

```
新的网络路径：

10.10.1.20:60000
        |
        v
192.168.1.20:443

Connection ID = ABC123
```

只要连接迁移过程满足 QUIC 的路径验证、安全性和连接状态要求，就可以继续使用原来的 QUIC Connection。

[HTTP/3 From A To Z: Core Concepts — Smashing Magazine](https://images.openai.com/static-rsc-4/mljl8mpD1g8qDA9TUtIdmkstZEKMuNZc1BYATSFTvoi52FlFiAPfMsvSWi91j1w0HiYJRS-j-ZeTybgLgVWALmbNvbVHwGrIXG0FxD1oTG8h95upC9Ltdv5kAdGsjIfISSGkJJTRMe3EuJKF-QKiS8fsw3-5rQ-obe0EeUIborY?purpose=inline)

[smashingmagazine.com](https://www.smashingmagazine.com/2021/08/http3-core-concepts-part1/?utm_source=chatgpt.com)

这对于手机网络切换、移动客户端和网络环境变化频繁的应用非常有价值。

需要注意：Connection ID 并不意味着 IP 地址和端口完全不重要。QUIC 仍然需要验证新的网络路径，防止攻击者利用伪造地址实施流量放大等攻击。

## 4.4 Stream：多路复用数据流

这是 QUIC 最重要的设计之一。

QUIC 在一个 Connection 中支持多个独立 Stream。

例如：

```
QUIC Connection
       |
       +--- Stream 0
       |
       +--- Stream 4
       |
       +--- Stream 8
       |
       +--- Stream 12
```

每个 Stream 都拥有自己的数据传输状态。

包括：

- Stream ID。
- 数据偏移量。
- 发送状态。
- 接收状态。
- 流级别流量控制状态。

假设：

```
Stream A：发送用户信息
Stream B：发送图片
Stream C：发送视频
```

即使 Stream B 中的数据丢失：

```
Stream A：继续交付
Stream B：等待丢失数据恢复
Stream C：继续交付
```

这就避免了 TCP 在不同逻辑数据流之间产生的队头阻塞。

但是这里有一个重要细节：

QUIC 并没有彻底消除队头阻塞，而是消除了不同 Stream 之间的传输层队头阻塞。

同一个 Stream 内部仍然需要保证有序交付。

例如：

```
Stream A：

Offset 0-100
Offset 100-200
Offset 200-300
```

如果 Offset 100-200 丢失，那么后面的数据即使已经到达，也不能作为完整、有序的 Stream 字节流交付。

因此 QUIC 只是把队头阻塞的影响范围从整个 Connection 缩小到了单个 Stream。

## 4.5 Packet：数据包

QUIC 还有一个很重要的概念：Packet。

QUIC Packet 是实际通过 UDP 发送的协议单元。

一个 UDP 数据报中可以承载一个或多个 QUIC Packet，具体取决于封装和报文类型。

QUIC Packet 可以包含：

- Packet Header。
- Packet Number。
- 一个或多个 Frame。
- 加密保护数据。

可以简单理解为：

```
UDP Datagram
    |
    +--- QUIC Packet
              |
              +--- Packet Header
              |
              +--- Packet Number
              |
              +--- Frame 1
              |
              +--- Frame 2
              |
              +--- Frame 3
```

这里需要区分 Packet 和 Stream。

- Packet 是网络传输中的数据包。
- Stream 是应用数据的逻辑传输通道。

一个 Packet 可以同时包含多个 Stream 的数据。

例如：

```
QUIC Packet 100
    |
    +--- STREAM Frame (Stream A)
    |
    +--- STREAM Frame (Stream B)
    |
    +--- ACK Frame
```

这样可以减少不必要的报文数量，提高传输效率。

## 4.6 Frame：帧

Frame 是 QUIC 内部真正承载控制信息和数据的基本单元。

常见 Frame 类型：

| Frame            | 作用                     |
| ---------------- | ---------------------- |
| STREAM           | 携带 Stream 数据           |
| ACK              | 确认收到的 Packet           |
| CRYPTO           | 携带 TLS 握手数据            |
| PING             | 探测连接状态                 |
| RESET_STREAM     | 终止一个 Stream            |
| MAX_DATA         | 更新 Connection 级别流量控制限制 |
| MAX_STREAM_DATA  | 更新 Stream 级别流量控制限制     |
| CONNECTION_CLOSE | 关闭连接                   |
| PATH_CHALLENGE   | 验证网络路径                 |
| PATH_RESPONSE    | 响应路径验证                 |

例如：

```
QUIC Packet
    |
    +--- ACK Frame
    |
    +--- STREAM Frame
    |
    +--- STREAM Frame
    |
    +--- PING Frame
```

你可以发现：

QUIC 不再像 TCP 一样，把确认信息、连接状态、数据序号等内容都设计在一个统一的 TCP Header 中。

它把很多功能拆分成独立的 Frame。

这让协议扩展更加灵活。

## 4.7 QUIC 如何保证可靠性？

QUIC 同样需要解决：

- 丢包。
- 乱序。
- 重复。
- 数据完整性。
- 网络拥塞。

但它没有直接复用 TCP 的序列号机制。

### Packet Number

QUIC 为每个发送的 Packet 分配 Packet Number。

例如：

```
Packet 100
Packet 101
Packet 102
Packet 103
```

Packet Number 在对应的 Packet Number Space 内严格递增，不会因为重传而复用。

假设：

```
Packet 100：包含 Stream A 的数据
Packet 101：包含 Stream B 的数据
Packet 102：包含 Stream C 的数据
```

Packet 101 丢失。

QUIC 不会重新发送一个 Packet Number 为 101 的 Packet。

而是：

```
Packet 103：重新携带 Stream B 中尚未确认的数据
```

因此：

```
原始数据：
Packet 101 -> Stream B, Offset 0-100

重传数据：
Packet 103 -> Stream B, Offset 0-100
```

这里有两个重要概念：

- Packet Number：标识一次发送。
- Stream Offset：标识数据在 Stream 中的位置。

这和 TCP 的设计有明显区别。

TCP 的序列号主要标识字节在字节流中的位置，重传时可能再次使用相同的序列号。

QUIC 则把发送次数和数据位置分开了。

这样能够避免 TCP 重传场景中存在的部分发送时间与确认归属歧义。

### ACK Frame

QUIC 使用 ACK Frame 确认 Packet。

例如：

```
收到：

100
101
103
104
106
```

接收端可以通过 ACK Frame 表达多个不连续的 Packet Number Range：

```
ACK:
100-101
103-104
106
```

这比只依赖累计 ACK 更灵活。

### 丢包重传

QUIC 的丢包恢复逻辑主要在 RFC 9002 中定义。

当发现 Packet 丢失时：

1. 判断该 Packet 是否包含需要可靠交付的数据。
2. 找到尚未确认的 Frame 内容。
3. 在新的 Packet 中重新发送需要可靠交付的 Frame。
4. 使用新的 Packet Number。
5. 根据 ACK 和丢包检测结果更新拥塞控制状态。

注意：不是所有 Frame 都需要重传。

例如：

- STREAM 数据通常需要可靠交付。
- CRYPTO 数据需要可靠交付。
- ACK Frame 通常不需要重传。
- PADDING Frame 没有需要恢复的应用数据。

## 4.8 QUIC 如何进行拥塞控制？

QUIC 同样需要拥塞控制。

否则大量 QUIC 客户端同时发送数据，就可能导致网络拥塞。

QUIC 的拥塞控制思想与 TCP 有很多相似之处。

它同样维护：

```
cwnd
bytes_in_flight
RTT
PTO
```

其中：

- cwnd：拥塞窗口。
- bytes_in_flight：当前网络中尚未确认的数据量。
- RTT：往返时延。
- PTO：探测超时时间。

QUIC 的拥塞控制可以使用不同算法。

例如：

- NewReno。
- CUBIC。
- BBR。

QUIC 规范定义了拥塞控制的基本要求和恢复机制，但不意味着所有 QUIC 实现都必须使用同一种算法。

## 4.9 QUIC 如何进行流量控制？

QUIC 不仅有 Connection 级别的流量控制，还有 Stream 级别的流量控制。

这是一个非常值得注意的设计。

TCP 主要通过连接级别的接收窗口限制发送量。

QUIC 则设计了两层限制：

第一层：Connection Flow Control

限制一个 QUIC Connection 中所有 Stream 累计可以发送的数据量。

例如：

```
MAX_DATA = 1 MB
```

表示整个连接的数据接收额度。

第二层：Stream Flow Control

限制每一个 Stream 可以发送的数据量。

例如：

```
Stream A: MAX_STREAM_DATA = 100 KB

Stream B: MAX_STREAM_DATA = 200 KB

Stream C: MAX_STREAM_DATA = 300 KB
```

这样即使某一个 Stream 消耗了大量数据，也不会无限占用接收端资源。

## 4.10 QUIC 如何实现加密？

传统 HTTPS：

```
HTTP
  |
TLS
  |
TCP
```

QUIC：

```
HTTP/3
   |
 QUIC
   |
 UDP
```

QUIC 把 TLS 1.3 的握手机制集成到了 QUIC 的连接建立过程中。

这里要特别注意：

QUIC 并不是重新发明了一套加密算法，而是使用 TLS 1.3 提供的密码学机制进行密钥协商和身份认证。

TLS 握手消息通过 QUIC 的 CRYPTO Frame 传输。

QUIC 自身负责：

- 密钥派生和更新的协议集成。
- Packet Header Protection。
- Packet Payload Encryption。
- 数据完整性验证。
- 不同加密阶段的密钥管理。

### QUIC 的握手过程

[The Fast and Secure Web: Exploring QUIC (HTTP/3) — The Next Generation HTTP and its Robust Security | by Vikas Sharma | Medium](https://images.openai.com/static-rsc-4/P0jWf4NJX7CHAlIqaY5ERAmBqFSlBVQfDZl5dikZV1_k5hGZUmJkNieBtY0yZZSTOBnR3mLIR_7GvoJ_QXxXcZH0b7gRQeqmkoFgyNk9GuuIkC4V5hU5hgrsgS3h1jlsmJLRC-HjI3zDJbnQYjNhoH10tJ4jN9ZA80zfLswD9yc?purpose=inline)

[psychovik.medium.com](https://psychovik.medium.com/the-fast-and-secure-web-exploring-quic-http-3-the-next-generation-http-and-its-robust-security-2e063dfe0262?utm_source=chatgpt.com)

传统 TCP + TLS 1.3：

```
TCP Handshake：1 RTT
TLS Handshake：1 RTT
```

QUIC：

```
QUIC + TLS 1.3 Handshake：通常 1 RTT
```

因为 QUIC 不需要先建立 TCP 连接，再单独进行 TLS 握手。

此外，QUIC 支持 0-RTT 数据。

客户端如果之前已经和服务端建立过连接，并且拥有可用的会话恢复凭据，就有机会在重新连接时直接发送早期数据。

```
Client                         Server

Initial + 0-RTT Data --------->

                              处理允许的早期数据

              <--------------- Handshake / 1-RTT
```

但 0-RTT 存在一个重要的安全限制：

早期数据可能被攻击者重放。

因此，服务端不能把 0-RTT 当成普通的、天然防重放的请求。

例如：

- 查询用户信息：通常比较适合。
- 扣款：需要特别谨慎。
- 转账：不能直接假设只执行一次。

具体是否允许 0-RTT，以及哪些请求可以安全地使用，需要由应用协议和业务共同决定。

## 4.11 QUIC 为什么能够在用户态实现？

这是 QUIC 相对于 TCP 一个非常重要的工程优势。

传统 TCP 通常由操作系统内核实现。

例如 Linux：

```
Application
    |
    | Socket API
    |
Linux Kernel
    |
    +--- TCP
    |
    +--- IP
    |
Network Card
```

如果 TCP 协议需要改进：

- 拥塞控制算法。
- 丢包恢复算法。
- 连接管理。
- 协议扩展。

通常需要依赖操作系统内核提供相应支持。

而 QUIC：

```
Application
    |
    +--- QUIC Library
    |       |
    |       +--- Connection
    |       +--- Stream
    |       +--- Packet
    |       +--- Frame
    |       +--- ACK
    |       +--- Congestion Control
    |       +--- TLS Integration
    |
    +--- UDP Socket
            |
        Linux Kernel
            |
            IP
            |
        Network Card
```

QUIC 的核心协议逻辑可以运行在用户态。

因此应用程序可以通过更新 QUIC 库来改进协议实现，而不必等待操作系统内核升级。

当然，用户态也不是没有代价：

- QUIC 库需要管理更多连接状态。
- 数据包处理会产生用户态与内核态之间的数据搬运成本。
- CPU 加密和协议处理可能成为瓶颈。
- 高性能实现仍然需要配合 UDP GRO、GSO 等网络优化技术。

# 五、TCP、UDP、QUIC 的核心设计对比

现在把三种协议放在一起看。

| 对比维度   | TCP         | UDP      | QUIC       |
| ------ | ----------- | -------- | ---------- |
| 设计模型   | 字节流         | 数据报      | 多路复用可靠传输   |
| 是否连接导向 | 是           | 否        | 是          |
| 可靠性    | 内置          | 不保证      | 内置         |
| 数据顺序   | 有序          | 不保证      | Stream 内有序 |
| 消息边界   | 不保留         | 保留       | Stream 不保留 |
| 重传     | 内置          | 无        | 内置         |
| 流量控制   | 有           | 无        | 有          |
| 拥塞控制   | 有           | 无        | 有          |
| 加密     | 需额外使用 TLS 等 | 需应用层实现   | 集成 TLS 1.3 |
| 多路复用   | 单字节流        | 应用层自行实现  | 原生多 Stream |
| 连接迁移   | 通常不支持       | 无连接状态可迁移 | 支持         |
| 常见实现位置 | 操作系统内核      | 操作系统内核   | 通常用户态      |

需要注意：UDP 本身没有可靠性和拥塞控制机制，但使用 UDP 的应用程序仍然应根据场景自行设计拥塞控制，不能认为 UDP 可以无限速发送。

## 5.1 三种协议的设计哲学

我认为这是最值得记住的部分。

TCP：把复杂性放在传输层

应用程序只需要读写字节流，TCP 负责可靠性、顺序、重传、流量控制和拥塞控制。

UDP：把复杂性留给应用层

传输层只提供简单的数据报服务，应用程序根据业务需要决定是否实现可靠性、重传、顺序等机制。

QUIC：重新设计可靠传输

在 UDP 之上构建现代化传输层，既保留可靠性，又解决多路复用、连接迁移、加密集成等问题。

# 六、实际开发中应该如何选择？

这三种协议并不是谁取代谁的问题，而是适用场景不同。

## 6.1 什么时候选择 TCP？

适合需要可靠、有序传输，并且不希望自己实现可靠性机制的场景。

典型应用：

- 数据库连接：MySQL、PostgreSQL、Redis（RESP over TCP）。
- 微服务 RPC：gRPC 默认使用 HTTP/2 over TCP。
- 文件传输。
- SSH。
- 传统 HTTPS。
- 消息队列客户端连接。

例如，你使用 Rust 的 SQLx 连接 PostgreSQL，本质上就是通过 TCP 建立连接，然后在 TCP 字节流上运行 PostgreSQL Wire Protocol。

## 6.2 什么时候选择 UDP？

适合数据报边界重要、允许丢包，或者应用层有特殊传输策略的场景。

典型应用：

- DNS。
- 在线游戏。
- 实时音视频。
- RTP。
- NTP。
- DHCP。
- 自定义低延迟通信协议。

需要特别注意：UDP 并不天然比 TCP 快。

如果应用层需要实现 ACK、重传、拥塞控制、流量控制等功能，开发复杂度可能非常高。

## 6.3 什么时候选择 QUIC？

适合需要可靠传输，同时希望获得多路复用、低连接延迟、连接迁移等能力的场景。

典型应用：

- HTTP/3。
- 移动互联网通信。
- 对连接建立延迟敏感的应用。
- 多路复用的网络服务。
- 网络环境频繁变化的客户端。

例如 HTTP/3 就是基于 QUIC 实现的。

[File:HTTP-1.1 vs. HTTP-2 vs. HTTP-3 Protocol Stack.svg - Wikimedia Commons](https://images.openai.com/static-rsc-4/VY4DdgqvZc7z4xDl_IjtY6fjcg9XhbAg90ER1JDaK8YcQyWPP4Mm-COvM8YuJ6lbdRYiIYOptFlgFn_ftEUUqH-xZTRiC596r55ZLmqbC_C20t-q_uMQauYP3nEzCDJvaKn3jJqh2W28dtr8F6jLl7E-Qi9sH1zUiUKlurkjP7Y?purpose=inline)

[commons.wikimedia.org](https://commons.wikimedia.org/wiki/File%3AHTTP-1.1_vs._HTTP-2_vs._HTTP-3_Protocol_Stack.svg?utm_source=chatgpt.com)

## 6.4 三种协议的实际选择

| 业务场景           | 推荐协议       | 原因                 |
| -------------- | ---------- | ------------------ |
| PostgreSQL 数据库 | TCP        | 成熟可靠的字节流           |
| Redis 客户端      | TCP        | RESP 协议通常运行在 TCP 上 |
| gRPC           | TCP        | HTTP/2 传输层通常使用 TCP |
| HTTP/3         | QUIC       | 多路复用、低延迟           |
| 在线 FPS 游戏      | UDP        | 实时性优先              |
| 视频会议           | UDP / QUIC | 根据实时性与可靠性需求选择      |
| 文件上传下载         | TCP / QUIC | 可靠传输               |
| DNS            | UDP / TCP  | 根据报文大小及协议场景选择      |

# 七、从源码实现角度进一步理解

如果你希望真正理解这三种协议，我建议从三个层次入手。

第一层：协议报文结构

先研究：

- TCP Header。
- UDP Header。
- QUIC Packet Header。
- QUIC Frame。

重点理解每个字段的含义以及它们为什么存在。

第二层：协议状态机

研究：

- TCP 三次握手、四次挥手。
- TCP 重传、ACK、滑动窗口。
- UDP Socket 的收发过程。
- QUIC Connection 和 Stream 的状态变化。
- QUIC 丢包检测与恢复。

第三层：实际代码实现

如果从 Rust 生态入手，可以研究以下项目：

[Tokio——Rust 编程语言的异步运行时。](https://images.openai.com/static-rsc-4/uBxXuPVnzK81A4NbiDp_RS3l7-LXqDu4yswPXDSP-XPLYPJi2IB1RAEkcIwn_4bVTyr6WZLENhlsW4J8HqdTfPGZjqUP8fTtGqUNi20yXk3s5KAGJJ4hGcHYnSTO4OkGgtDsAm70NzkjS7lr1AqPhpiXOO1yIZhfVzCf_Nza6mM?purpose=inline)

Tokio

Rust 异步网络编程基础，研究 TCP、UDP Socket、异步 IO。

[GitHub 源码](https://github.com/tokio-rs/tokio?utm_source=chatgpt.com)

[Quinn — Rust network library // Lib.rs](https://images.openai.com/static-rsc-4/dg5EpJeN_dWAFV5XjVbGQig6bQbT-9dPMbkaEXLwiemdX0BZ0CQpHl0jVd3tGBvjeKD1GIyihGc8T5qx6iD8aNUWcDJ0NoycrAXx1tTucmJAGEnIysf0FIuqUBem-_Ib_jll74RLfjnkpksTb5cMhuEEYwxxd7_2sM3MmZLdxMs?purpose=inline)

Quinn

Rust 实现的 QUIC 协议库，适合研究 Connection、Stream、Packet、Frame。

[GitHub 源码](https://github.com/quinn-rs/quinn?utm_source=chatgpt.com)

[GitHub - cloudflare/quiche: 🥧 Savoury implementation of the QUIC transport protocol and HTTP/3 · GitHub](https://images.openai.com/static-rsc-4/VdMeohhsweX2_9albKkdNzrv-6C8FfDwxnEwczfk8Zoxnz1NlCrrY53jlLF_hc5Yua80zL2MG1WAt7LeYun8SLN7dXh1kFh1yxu-7raviRj-JW9NLBuCDgaPZ2xbWF95saTebc-tcnyz2ZR4gh5HrM92n05mle_T96sY8TGs3WU?purpose=inline)

Quiche

Cloudflare 开源的 QUIC 和 HTTP/3 实现，适合研究协议底层实现。

[GitHub 源码](https://github.com/cloudflare/quiche?utm_source=chatgpt.com)

# 八、推荐阅读的协议标准

如果想深入理解协议设计，最终还是要回到 RFC。

| 协议            | RFC      | 主要内容       |
| ------------- | -------- | ---------- |
| TCP           | RFC 9293 | TCP 基础规范   |
| UDP           | RFC 768  | UDP 基础规范   |
| QUIC          | RFC 9000 | QUIC 传输协议  |
| QUIC + TLS    | RFC 9001 | QUIC 的加密设计 |
| QUIC Recovery | RFC 9002 | 丢包恢复与拥塞控制  |

原始标准文档：

- [RFC 9293 - TCP](https://www.rfc-editor.org/rfc/rfc9293.html?utm_source=chatgpt.com)
- [RFC 768 - UDP](https://www.rfc-editor.org/rfc/rfc768.html?utm_source=chatgpt.com)
- [RFC 9000 - QUIC](https://www.rfc-editor.org/rfc/rfc9000.html?utm_source=chatgpt.com)
- [RFC 9002 - QUIC Loss Detection and Congestion Control](https://www.rfc-editor.org/rfc/rfc9002.html?utm_source=chatgpt.com)

最后给你一个学习建议：

如果你的目标是理解现代网络协议的设计，我建议重点研究 QUIC。

因为 QUIC 集合了 TCP 过去几十年积累的可靠传输设计，又针对 TCP 的一些架构限制进行了重新设计。

尤其值得深入研究三个问题：

1. 为什么 QUIC 使用 Packet Number 和 Stream Offset 两套编号机制，而 TCP 主要使用 Sequence Number？
2. QUIC 为什么能够解决 HTTP/2 在 TCP 层的跨 Stream 队头阻塞？
3. QUIC 为什么把连接状态和传输逻辑放在用户态，而 TCP 通常放在内核态？

这三个问题搞明白，你对现代传输层协议的设计就会有比较系统的认识。