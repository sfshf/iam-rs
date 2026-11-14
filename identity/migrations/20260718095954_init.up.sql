-- Add up migration script here

-- `t_user` 用户表，用户的账号信息
CREATE TABLE IF NOT EXISTS "public"."t_user" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(), -- 记录ID
    "username" VARCHAR(255) NOT NULL UNIQUE, -- 用户名
    "email" VARCHAR(255) NOT NULL UNIQUE, -- 邮箱
    "mobile" VARCHAR(255) NOT NULL, -- 手机号
    "status" SMALLINT NOT NULL, -- 用户状态，1:启用，2:禁用，3:冻结
    "created_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    "updated_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP -- 更新时间
);

-- `t_profile` 用户资料表，用户经常修改的身份信息
CREATE TABLE IF NOT EXISTS "public"."t_profile" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(), -- 记录ID
    "user_id" UUID NOT NULL UNIQUE, -- 用户ID，关联t_user表
    "nickname" VARCHAR(255) NOT NULL, -- 昵称
    "avatar" VARCHAR(255) NOT NULL, -- 头像url
    "real_name" VARCHAR(255), -- 真实姓名
    "gender" SMALLINT, -- 性别，1:男，2:女
    "birthday" DATE, -- 生日
    "address" TEXT, -- 地址
    "created_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    "updated_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP -- 更新时间
);

-- `t_session` 会话表
CREATE TABLE IF NOT EXISTS "public"."t_session" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(), -- 记录ID
    "type" SMALLINT NOT NULL, -- session类型，1:web密码登录，2:app密码登录
    "user_id" UUID NOT NULL, -- 用户ID，关联t_user表
    "trace_id" VARCHAR(255) NOT NULL, -- 追踪ID，关联请求链路
    "ip" VARCHAR(255) NOT NULL, -- 登录IP
    "ua" TEXT NOT NULL, -- 登录UA
    "status" SMALLINT NOT NULL, -- session状态，1:在线，2:离线
    "created_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    "updated_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP -- 更新时间
);

-- `t_passphrase` 密码表
CREATE TABLE IF NOT EXISTS "public"."t_passphrase" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(), -- 记录ID
    "user_id" UUID NOT NULL, -- 用户ID，关联t_user表
    "phrase_salt" VARCHAR(32) NOT NULL, -- 密码盐
    "phrase_hash" VARCHAR(255) NOT NULL, -- 密码摘要
    "created_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    "updated_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP -- 更新时间
);