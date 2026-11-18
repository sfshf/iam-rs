# IAM 产品需求文档

## 1. 产品简介

### 1.1 产品名称

IAM（Identity & Access Management）

### 1.2 产品定位

IAM 是一个现代化、云原生、微服务架构的身份认证与权限管理平台。

它提供统一的身份管理、认证、授权、组织架构管理、多租户管理以及开放身份协议（OAuth2、OIDC、SAML）的支持，帮助企业快速搭建统一身份中心（Identity Provider）。

产品支持作为独立身份中心部署，也支持作为企业内部基础设施接入各种业务系统。

---

## 2. 产品目标

### 主要目标

- 统一身份认证
- 集中权限管理
- 统一用户中心
- 多租户支持
- 微服务统一认证
- 零信任安全架构
- Cloud Native
- 高性能
- 高可扩展

---

## 3. 产品架构

```
                +----------------+
                |     Web UI     |
                +-------+--------+
                        |
                API Gateway
                        |
      +-----------------+------------------+
      |                 |                  |
+-----v-----+   +-------v------+   +--------v-------+
| Identity  |   | Authorization|   | Organization  |
|  Service  |   |    Service   |   |    Service    |
+-----------+   +--------------+   +---------------+

      +-------------+--------------+
                    |
            Message Queue
                    |
      +-------------+-------------+
      |                           |
 Audit Service             Notification

```

---

## 4. 核心功能

### 4.1 用户管理

功能：

- 用户注册
- 用户登录
- 用户禁用
- 用户冻结
- 用户启用
- 修改密码
- 重置密码
- 修改邮箱
- 修改手机号
- 修改头像
- MFA管理
- 登录历史

属性：

- Username
- Nickname
- Avatar
- Email
- Mobile
- Status
- Password Policy
- Login History

---

### 4.2 身份认证（Authentication）

支持：

- 用户名密码
- 手机验证码
- 邮箱验证码
- OAuth2 Login
- OIDC Login
- LDAP Login
- AD Login
- Passkey(WebAuthn)
- TOTP
- SMS OTP
- Email OTP
- MFA

登录流程：

```
User
 ↓
Credential Verification
 ↓
MFA Verification
 ↓
Issue JWT Token
 ↓
Access Resource
```

---

### 4.3 授权管理（Authorization）

支持：

- RBAC
- ABAC
- ACL
- Policy Engine
- Casbin

权限对象：

- API
- Menu
- Button
- Page
- Data
- Resource

权限粒度：

- Read
- Write
- Update
- Delete
- Execute

---

### 4.4 角色管理

角色功能：

- 创建角色
- 删除角色
- 修改角色
- 分配权限
- 分配菜单
- 分配用户
- 角色继承

支持：

```
Super Admin
    │
 Admin
    │
Operator
    │
 Guest
```

---

### 4.5 组织管理（Organization）

支持：

- 企业
- 部门
- 岗位
- 用户组

组织结构：

```
Company

├──研发部
│   ├──后台组
│   └──前端组
│
├──市场部
│
└──运营部
```

---

### 4.6 多租户（Multi Tenant）

支持：

- Tenant
- Project
- Namespace
- Domain

租户隔离：

- 数据隔离
- 用户隔离
- 权限隔离
- 配置隔离

---

### 4.7 OAuth2 服务

支持：

- Authorization Code
- PKCE
- Client Credential
- Refresh Token
- Device Code
- Password（可选）

支持：

- OpenID Connect
- JWT
- JWKS
- Discovery Endpoint

---

### 4.8 API Key

支持：

- 创建API Key
- Secret管理
- 权限绑定
- 生命周期管理
- 自动过期

---

### 4.9 Session管理

支持：

- JWT
- Redis Session
- Refresh Token
- Remember Me
- Logout Everywhere

---

### 4.10 审计日志（Audit）

记录：

- 登录
- 登出
- 权限修改
- 用户修改
- API调用
- Token刷新
- 管理员操作

支持：

- 查询
- 导出
- 检索
- 风险分析

---

### 4.11 通知中心

支持：

- Email
- SMS
- Webhook
- 企业微信
- 钉钉
- Slack

通知事件：

- 登录成功
- 登录失败
- 修改密码
- MFA绑定
- 权限变更

---

## 5. 管理后台

模块：

```
Dashboard

Identity

Authentication

Authorization

Users

Roles

Permissions

Resources

Organizations

Departments

Groups

Tenants

OAuth2 Clients

API Keys

Audit Logs

Settings

System
```

---

## 6. 微服务拆分

推荐拆分：

```
iam-api

identity-service

user-service

organization-service

role-service

permission-service

policy-service

oauth-service

session-service

audit-service

notification-service

gateway
```

---

## 7. 数据模型

核心实体：

```
Tenant

Organization

Department

Group

User

Role

Permission

Policy

Menu

API

OAuth Client

Session

Audit Log

Notification
```

关系：

```
Tenant

 ├── Organization

 │      ├── Department

 │      │      └── User

 │      │

 │      └── Group

 │

 └── Role

        └── Permission

                └── Resource
```

---

## 8. 非功能需求

### 性能

- API响应 < 100ms
- 登录 < 300ms
- 权限校验 < 5ms
- 支持100万用户
- 支持10万QPS

---

### 安全

支持：

- HTTPS
- JWT
- OAuth2
- OIDC
- MFA
- RBAC
- ABAC
- API Rate Limit
- IP Blacklist
- Device Fingerprint
- Risk Control

---

### 可扩展

支持：

- Plugin
- Hook
- Event Bus
- Webhook
- SDK
- OpenAPI

---

## 9. 技术栈

后端：

- Rust
- Tokio
- Tonic
- Axum
- SQLx
- SeaORM（可选）
- Casbin
- Redis
- PostgreSQL
- Kafka/NATS

前端：

- React
- TypeScript
- Vite
- Ant Design

部署：

- Docker
- Kubernetes
- Helm
- Prometheus
- Grafana
- OpenTelemetry

---

## 10. 产品路线图

### v0.1

- 用户管理
- RBAC
- JWT登录
- 权限管理
- 菜单管理

### v0.2

- OAuth2
- OIDC
- 多租户
- API Key
- 审计日志

### v0.3

- MFA
- LDAP
- AD
- Passkey
- Webhook

### v1.0

- 插件系统
- Workflow
- Policy Engine
- SSO
- Federation
- 高可用集群
