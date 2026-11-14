# --- 环境变量 ---
# PostgreSQL 连接信息（可以从 .env 文件加载或直接配置）
export PG_HOST := "localhost"
export PG_PORT := "5432"
export PG_USER := "postgres"
export PG_PASSWORD := "dev123"
export PG_DATABASE_PREFIX := "iam_"  # 数据库名前缀

# --- 工具函数 ---
# 获取微服务列表
get_services: # 这里可以列出所有微服务的名称，按需添加
    @echo "aud" # 审计微服务
    @echo "authz" # 授权微服务
    @echo "identity" # 身份管理微服务
    @echo "domain" # 组织管理微服务
    @echo "daily" # 日常微服务

# 为微服务生成数据库名
get_db_name service:
    @echo "{{PG_DATABASE_PREFIX}}{{service}}"

# --- 数据库操作 ---

# docker启动postgres数据库
docker-start-postgres:
    docker run --name postgres -e POSTGRES_PASSWORD=dev123 -p 5432:5432 -d postgres:latest

# 创建新的数据库（为单个微服务）
create-db service:
    #!/usr/bin/env bash
    set -euo pipefail
    db_name=$(just get_db_name {{service}})
    sqlx database create -D "postgres://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$db_name" || true
    echo "✅ Database '$db_name' created or already exists."

# 为所有微服务创建数据库
create-all-dbs:
    #!/usr/bin/env bash
    set -euo pipefail
    for service in $(just get_services); do
        just create-db "$service"
    done
    echo "✅ All databases created."

# --- 迁移操作 ---

# 为单个微服务初始化migrations目录
migrate-init service:
    #!/usr/bin/env bash
    set -euo pipefail
    # 检查是否初始化过 migrations 目录
    if [ -d "{{service}}/migrations" ]; then
        echo "⚠️ Migrations directory already exists for service '{{service}}'. Skipping initialization."
        exit 0
    fi
    cd {{service}}
    sqlx migrate add -r init
    echo "✅ Migrations initialized for service '{{service}}'."

# 为所有微服务初始化migrations目录
migrate-init-all:
    #!/usr/bin/env bash
    set -euo pipefail
    for service in $(just get_services); do
        just migrate-init "$service"
    done
    echo "✅ Migrations initialized for all services."

# 为单个微服务执行迁移（使用 sqlx）
migrate service:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{service}}
    db_url="postgres://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$(just get_db_name {{service}})"
    sqlx migrate run -D "$db_url"

# 为所有微服务执行迁移
migrate-all:
    #!/usr/bin/env bash
    set -euo pipefail
    for service in $(just get_services); do
        echo "🔄 Migrating $service..."
        just migrate "$service"
    done
    echo "✅ All migrations applied."

# 回滚单个微服务的迁移
migrate-revert service:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{service}}
    db_url="postgres://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$(just get_db_name {{service}})"
    export DATABASE_URL="$db_url"
    sqlx migrate revert -D "$db_url" || true
    echo "✅ Migration rolled back for service '{{service}}'."

# 回滚所有微服务的迁移
migrate-revert-all:
    #!/usr/bin/env bash
    set -euo pipefail
    for service in $(just get_services); do
        echo "🔄 Rolling back $service..."
        just migrate-revert "$service"
    done
    echo "✅ All migrations rolled back."

# --- 辅助命令 ---

# 重置数据库（删除并重新创建 + 迁移）
reset-db service:
    #!/usr/bin/env bash
    set -euo pipefail
    db_name=$(just get_db_name {{service}})
    sqlx database drop -y -D "postgres://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$db_name"
    just create-db "{{service}}"
    just migrate "{{service}}"
    echo "✅ Database '$db_name' reset and migrated."

# 重置所有数据库
reset-all-dbs:
    #!/usr/bin/env bash
    set -euo pipefail
    for service in $(just get_services); do
        just reset-db "$service"
    done
    echo "✅ All databases reset and migrated."

# 创建新的微服务（假设已手动创建目录和 Cargo.toml）
new-service service:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo new --bin {{service}}
    mkdir -p {{service}}/migrations
    just create-db "{{service}}"
    cd {{service}}
    sqlx migrate add -r init
    echo "✅ New service '{{service}}' created with database and migrations."

# 删除微服务（删除数据库和目录）
delete-service service:
    #!/usr/bin/env bash
    set -euo pipefail
    db_name=$(just get_db_name {{service}})
    sqlx database drop -D "postgres://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$db_name" || true
    if [ ! -d "{{service}}" ]; then
        echo "❌ Service '{{service}}' does not exist."
        exit 1
    fi
    echo "🔄 Removing '{{service}}' from Cargo.toml..."
    # 清理缓存
    echo "🧹 Cleaning Cargo cache..."
    cargo clean -p {{service}}
    # 删除目录
    echo "🗑️  Deleting '{{service}}' directory..."
    rm -rf {{service}}
    echo "✅ Service '{{service}}' directory deleted along with its database. Please remember to manually remove '{{service}}' member from Cargo.toml if necessary."

# ping微服务
ping-service service version="v1" :
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -d "./proto/{{service}}/{{version}}/" ]; then
        echo "❌ Service '{{service}}/{{version}}' does not exist."
        exit 1
    fi
    grpcurl -plaintext -import-path ./proto/vendor -import-path ./proto/{{service}}/{{version}} -proto {{service}}.proto -d "{\"ts\": $(date +%s)}" '[::1]:50051' {{service}}.{{version}}.$(echo {{service}}Service | awk '{print toupper(substr($0,1,1)) substr($0,2)}')/Ping