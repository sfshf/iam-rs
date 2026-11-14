-- Add up migration script here

-- `t_casbin_rule` casbin规则表
-- 主要是用casbin的 `RBAC with Domains` 模型
-- ptype -> p, g
-- 当ptype为g时：v0->user uuid, v1->role uuid, v2->domain uuid
-- 当ptype为p时：v0->role uuid, v1->domain uuid, v2->resource identifier, v3->method
--      当resource为非API（菜单、控件）时，v3存储空字符串
--      当resource为API时，v3存储相应的GET、POST、DELETE方法
CREATE TABLE IF NOT EXISTS "public"."t_casbin_rule" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "ptype" VARCHAR(255) NOT NULL, -- 规则类型，p/g
    "v0" VARCHAR(255) NOT NULL, -- 规则参数1
    "v1" VARCHAR(255) NOT NULL, -- 规则参数2
    "v2" VARCHAR(255) NOT NULL, -- 规则参数3
    "v3" VARCHAR(255) NOT NULL, -- 规则参数4
    "v4" VARCHAR(255) NOT NULL, -- 规则参数5
    "v5" VARCHAR(255) NOT NULL, -- 规则参数6
    "created_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    "updated_at" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP -- 更新时间
);
