-- Add up migration script here

-- `t_domain` 域表
CREATE TABLE IF NOT EXISTS "public"."t_domain" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(), -- 记录ID
    "name" VARCHAR(255) NOT NULL, -- 域名称
    "intro" TEXT NOT NULL, -- 域简介
    "created_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    "updated_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP -- 更新时间
);

-- `t_resource` 资源（菜单、控件、API）表
CREATE TABLE IF NOT EXISTS "public"."t_resource" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(), -- 记录ID
    "type" SMALLINT NOT NULL, -- 资源类型，1:菜单，2:控件，3:API
    "code" VARCHAR(255) NOT NULL UNIQUE, -- 资源编码（如 menu::user::list, widget::user::add, api::/v1/identity/user::get），唯一
    "name" VARCHAR(255) NOT NULL, -- 资源名称
    "intro" TEXT NOT NULL, -- 资源简介
    "icon" VARCHAR(255) NULL, -- 资源图标url
    "created_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    "updated_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP -- 更新时间
);

-- `t_role` 角色表
CREATE TABLE IF NOT EXISTS "public"."t_role" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(), -- 记录ID
    "code" VARCHAR(255) NOT NULL UNIQUE, -- 角色编码（如 admin_01, user_01），唯一
    "name" VARCHAR(255) NOT NULL, -- 角色名称
    "intro" TEXT NOT NULL, -- 角色简介
    "created_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    "updated_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP -- 更新时间
);