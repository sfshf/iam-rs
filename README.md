# iam-rs

IAM implemented by rust.

## Intro

A modern Identity and Access Management (IAM) platform built with Rust, leveraging a microservices architecture and a decoupled frontend-backend design.

## HLD (High Level Design)

## LLD (Low Level Design)

## Tech Stack

This project is built on a microservices architecture, leveraging the following tech stack:

- Async Runtime: [tokio](https://github.com/tokio-rs/tokio) for high-performance asynchronous programming in Rust.
- gRPC Development: [tonic](https://github.com/grpc/grpc-rust) and [prost](https://github.com/tokio-rs/prost) for efficient RPC communication.
- Gateway: A custom-built gateway using [pingora](https://github.com/cloudflare/pingora) for request routing and load balancing.
- Database: [sqlx](https://github.com/transact-rs/sqlx) for type-safe database operations, with migrations managed via [sqlx-cli](https://github.com/transact-rs/sqlx/tree/main/sqlx-cli).
- Scripting: [just](https://github.com/casey/just) for streamlining common development tasks.
- Evenet Broker: Using [Apache Iggy](https://github.com/apache/iggy) as an event broker.

## License

This project is licensed under the Apache License 2.0 - see the LICENSE file for details.

## 手动执行grpc代码构建

```sh
cargo clean
cargo check -p proto -vv
```
