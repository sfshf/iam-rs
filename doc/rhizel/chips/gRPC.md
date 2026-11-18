# [gRPC](https://github.com/grpc/grpc)

gRPC 是 Google 开源的高性能、跨语言的远程过程调用（RPC）框架。

它的核心特性包括：

- 基于 HTTP/2（多路复用、头压缩、双向流）作为传输层
- 使用 Protocol Buffers（protobuf） 作为默认的接口定义与序列化格式（也可用 JSON 等）
- 原生支持 双向流（streaming）： Unary / Server streaming / Client streaming / Bidirectional streaming
- 提供跨语言代码生成（客户端 + 服务端存根）
- 支持拦截器、超时、重试、元数据（metadata）、负载均衡、认证（TLS/mTLS）、健康检查与监控集成

gRPC 的目标是把远程调用尽量做得像本地函数调用，同时提供生产级的网络能力（高并发、低延迟、流控）。

## 为什么 gRPC 内部会有负载均衡能力（设计动机）

gRPC 并不是单纯的“发出 HTTP 请求”的客户端库：它是面向大规模微服务环境设计的 RPC 运行时。在这种环境下，客户端通常遇到的需求和问题包括：

1. 服务实例众多且动态：后端服务会横向扩容、滚动更新、节点上下线。
2. 需要低延迟高并发的连接复用：把很多 RPC 多路复用在少量长连接（HTTP/2）上。
3. 对流量分配、故障转移、优先级、地域感知有要求：需要复杂策略而非简单 DNS。
4. 可插拔的发现系统：可以和 Consul、etcd、k8s、DNS、xDS 等结合。
5. 客户端希望控制如何选取后端（round_robin、least_conn、pinning、consistent-hash）。

因此 gRPC 在客户端栈里实现了 `NameResolver + Balancer + Channel + Transport` 的分层设计，目的是把「发现」和「选择后端」从用户代码里抽象出来，由运行时管理：resolver 告诉有哪些后端地址，balancer 维护连接（SubConn）并决定每次 RPC 要用哪个连接（Picker）。这样可以在客户端做到智能路由、快速 failover、流控和按策略分配请求。

## gRPC 的核心架构（文字 + 图解）

下面先给出一张简化但完整的架构图（ASCII），然后逐块解释。

```

+-------------------------------------------------------------+
|                          Application                        |
|  (Your client code)                                         |
|                                                             |
|  call() -> gRPC Stub / Generated Client -> Channel          |
+-------------------------------+-----------------------------+
                                |
                                v
+-------------------------------+-----------------------------+
|                gRPC Client Core (Runtime)                   |
|                                                             |
|  ┌─────────────┐   ┌───────────────┐   ┌───────────────┐     |
|  | NameResolver|-->| Balancer      |-->| SubConn Pool  |     |
|  | (Consul/DNS)|   | (policy: RR)  |   | (connections) |     |
|  └─────────────┘   └───────────────┘   └───────────────┘     |
|         ^                 ^                    ^             |
|         |                 |                    |             |
|   service discovery   Balancer config      Conn health / TLS  |
|                                                     (HTTP/2)  |
+-------------------------------------------------------------+
                                |
                                v
+-------------------------------+-----------------------------+
|                         Transport Layer                     |
|           HTTP/2 Streams over TCP + TLS (per SubConn)       |
|  - each RPC → a stream on chosen SubConn (connection)       |
|  - stream-level flow control, headers, gRPC framing         |
+-------------------------------------------------------------+
                                |
                                v
+-------------------------------+-----------------------------+
|                       Upstream Server Nodes                 |
|  [svc1:ip:port]  [svc2:ip:port]  [svc3:ip:port]             |
+-------------------------------------------------------------+

```

### 架构各组件详解

#### 1. Channel（客户端通道）

- `Channel` 是 gRPC 的客户端抽象，代表到某个“服务名”的逻辑连接。
- 应用通过生成的 Stub 使用 Channel 发起 RPC。
- Channel 负责把 RPC 分发到内部的 Balancer/Transport。

#### 2. NameResolver（名称解析器）

- 负责将逻辑目标（如 `consul://svc`、`dns:///svc`、`xds:///...`）解析为一组 `addr:port`。
- 可以动态 watch（blocking query / watch）并推送地址更新给 Balancer。

#### 3. Balancer（负载均衡器） —— 核心

- 职责：
  - 接收 NameResolver 更新的地址集合；
  - 为每个后端维护一个或多个 SubConn（代表一个 TCP/TLS/HTTP2 连接）；
  - 管理连接生命周期（connect / reconnect / teardown）；
  - 根据策略（pick_first、round_robin、least_conn、consistent_hash、xDS-driven）产生一个 Picker；
  - 提供 Picker 给 Channel：每次 RPC 调用时，Channel 向 Picker 请求选择哪个 SubConn。

- 实现要点：
  - SubConn 抽象：代表与某个后端建立的底层连接（并发流复用的实体）；
  - Picker：选取 SubConn 的算法/策略实现，负责快速决策；
  - Health / Priority / Locality：高级策略支持优先级、Locality-aware 路由（例如 GCP Traffic Director、xDS）。

#### 4. SubConn Pool（连接池）

- Balancer 为每个后端地址或地址组建立 SubConn（通常是长连接，HTTP/2）。
- SubConn 上可以承载多个 RPC（每个 RPC 对应一个 HTTP/2 stream）。

#### 5. Transport Layer（HTTP/2）

- 真正的数据传输使用 HTTP/2（或 HTTP/3/QUIC 在新实现里）。
- 每个 SubConn 上建立一个或若干 HTTP/2 连接（根据实现），然后在连接上建立 stream 载荷 RPC 数据。
- HTTP/2 提供流级流控、优先级、multiplexing —— 非常适合高并发 RPC 场景。

#### 6. Server（上游）

- gRPC 服务端接收 HTTP/2 stream，按 proto 定义反序列化并执行业务逻辑，返回流或响应。

### gRPC 中负载均衡的工作流程（时序说明）

1. 客户端代码 `Dial("consul://svc", WithBalancer("round_robin"))` 创建 Channel。
2. NameResolver 启动，向 Consul 查询 svc 的实例地址并持续 watch。
3. Resolver 把地址列表推给 Balancer（初始与后续变化）。
4. Balancer 为每个地址创建 SubConn 并尝试连接（建立 HTTP/2）。
5. Balancer 构建/更新 Picker（例如新的 round_robin 列表）。
6. 应用发起 RPC → Channel 调用 Picker 选 SubConn → 在该 SubConn 上开一个 HTTP/2 stream 发送请求。
7. 若某 SubConn 故障，Balancer 标记其状态为 unavailable、更新 Picker，后续 RPC 不会再选它；同时可做重试/熔断。
8. 当 Resolver 报告新的节点加入/下线，Balancer 再次调整 SubConns 与 Picker。

### gRPC 支持的负载均衡模式（常见）

- **_pick_first_**：选第一个可用 SubConn（最简单，常用于单实例或 proxy-lb）。
- **_round_robin_**：在可用 SubConn 间轮询，适合比较均衡的请求分配。
- **_least_connection / least_request（部分实现）_**：选择连接负载最小的后端。
- **_consistent_hashing_**：按 key（session id）哈希实现会话粘滞。
- **_xDS（Envoy/GCP Traffic Director）_**：控制面驱动的复杂负载均衡（优先、环节、区域感知、熔断、权重、LRS 报告）。
- **_grpclb（旧方案，已弱化/逐步替代）_**：使用专门的负载均衡服务返回背后实例与权重。

**注意：** `Resolver 只是“提供地址”`，真正的“如何分配请求”由 Balancer（以及 gRPC 提供的 balancer 实现或你自定义的 balancer）来完成。

### 典型部署模式（客户端侧 LB vs 代理/服务网格）

- **客户端侧负载均衡（client-side LB）**：gRPC 客户端直接从 resolver 获得多个后端并在本地做选择（常见）。优点：减少单点、直连后端；缺点：每个客户端要维护连接，复杂策略分散。
- **代理/边车（sidecar）或网关（proxy-side LB）**：客户端只连接到一个代理（如 Envoy），由代理负责全部 L7 负载均衡。优点：集中策略、统一监控、流量控制；缺点：增加跳转、需部署数据平面。
- **混合（xDS）**：使用控制平面（Consul Connect / Istio / GCP）下发配置，数据平面（Envoy）做实际路由，客户端使用 xDS-aware gRPC 或直接与 Envoy 通信。

### 扩展能力（生产级特性）

- **健康检查**：gRPC/Envoy 支持主动 L7 健康检查，Balancer 可以据状态移除节点。
- **负载报告（LRS）**：客户端/Envoy 可以向控制平面汇报负载信息，控制平面据此调整。
- **优先级 / 重试策略**：xDS 支持复杂路由规则（按权重、优先级、流式限制）。
- **连接窃取 / Warmup**：Balancer 可以预建 SubConn（连接池）以降低首次延迟。
- **流控与背压**：HTTP/2 在 transport 层天然支持流级别流控，gRPC 利用此控制推送速率。

### 总结（重点回顾）

- gRPC 内置负载均衡能力是为了满足微服务环境中`动态服务发现 + 高并发低延迟 + 智能路由`的需求。
- 设计上把职责分为 `NameResolver（发现）` 与 `Balancer（选择与连接管理）`，二者分工明确：resolver 提供地址，balancer 决定如何分配请求。
- gRPC 的数据传输基于 HTTP/2（或未来 HTTP/3/QUIC），在单连接上支持多流，从而高效复用连接并把负载均衡决策提升到每次 RPC 级别。
- 在生产中，你可以选用`客户端侧 LB`（gRPC 自身）或`代理/网格侧 LB`（Envoy + xDS）——两者常常混合使用以达到可观测、可控、可扩展的目标。

# [Protobuf](https://protobuf.com/docs/introduction)
