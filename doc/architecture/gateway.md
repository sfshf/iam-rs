# Gateway

This project is a custom-built API Gateway developed using [Pingora](https://github.com/cloudflare/pingora), Cloudflare's high-performance Rust-based proxy framework.

It is designed to provide a high-performance, extensible, and reliable gateway solution, with the following key features:

## Key Features

- **High Performance** – Built with Rust and asynchronous I/O to deliver high throughput and low latency.
- **Reverse Proxy** – Supports HTTP/1.1 and HTTP/2 reverse proxying.
- **Load Balancing** – Supports multiple load balancing algorithms, including Round Robin and consistent hashing.
- **Health Checks** – Monitors upstream server health and automatically excludes unhealthy instances from load balancing.
- **Fault Tolerance** – Supports upstream failover, retry policies, and configurable failure handling.
- **TLS Termination** – Supports HTTPS and flexible TLS configuration.
- **WebSocket & gRPC Proxying** – Supports WebSocket and gRPC traffic forwarding.
- **Dynamic Routing** – Supports flexible and customizable request routing based on HTTP methods, headers, paths, and other request attributes.
- **Request & Response Transformation** – Allows custom manipulation of HTTP headers, request bodies, and response data.
- **Connection Management** – Provides connection pooling, keep-alive, and efficient upstream connection reuse.
- **Graceful Reload & Shutdown** – Supports graceful restarts and shutdowns without abruptly interrupting in-flight requests.
- **Observability** – Integrates with logging, metrics, and distributed tracing systems for monitoring and troubleshooting.
- **Extensible Architecture** – Provides customizable middleware and request-processing hooks to support application-specific gateway logic.

---

<p align="center">
← <a href="overview.md">Overview</a> &nbsp; | &nbsp; <a href="identity.md">Identity Server</a> →
</p>
