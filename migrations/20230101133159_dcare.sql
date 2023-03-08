-- DROP TABLE IF EXISTS sessions;
-- DROP TABLE IF EXISTS order_gsheets;
-- DROP TABLE IF EXISTS order_histories;
-- DROP TABLE IF EXISTS orders;
-- DROP TABLE IF EXISTS users;
-- DROP TABLE IF EXISTS titles;
-- DROP TABLE IF EXISTS department_orgs;
-- DROP TABLE IF EXISTS departments;
-- DROP TABLE IF EXISTS department_types;
-- DROP TABLE IF EXISTS models;
-- DROP TABLE IF EXISTS accessories;
-- DROP TABLE IF EXISTS faults;
-- DROP TABLE IF EXISTS status;

-- 職稱
CREATE TABLE IF NOT EXISTS titles (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name text NOT NULL UNIQUE           -- 職稱
);

-- CREATE TABLE IF NOT EXISTS department_types (
--     id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
--     name text NOT NULL UNIQUE           -- 維保中心,營業點,總部,其他
-- );
-- INSERT INTO department_types ( name ) VALUES ( '總部' )
-- ON CONFLICT (name) DO NOTHING;

-- 部門(涵 門市&維保中心)
CREATE TABLE IF NOT EXISTS departments (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    create_at timestamptz NOT NULL DEFAULT NOW(),   -- 創建時間
    update_at timestamptz,

    shorten text NOT NULL UNIQUE,       -- 門市代號
    store_name text UNIQUE,             -- 門市名稱
    owner text,                         -- 負責人
    telephone text,                     -- 門市電話
    address text,                       -- 門市地址
    type_mask bit(8),			-- Disable (X), ADM(Y)....
    extra text                       	-- future usage?
);
INSERT INTO departments ( shorten, store_name, type_mask ) VALUES ( 'ADM', '總部', b'10000000')
ON CONFLICT (shorten) DO NOTHING;

CREATE TABLE IF NOT EXISTS department_orgs (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    create_at timestamptz NOT NULL DEFAULT NOW(),   -- 創建時間

    parent_id integer REFERENCES departments (id) ON DELETE CASCADE,
    child_id integer REFERENCES departments (id) ON DELETE CASCADE,
    extra text                       	-- future usage?
);

-- 工作人員, admin也算進來
CREATE TABLE IF NOT EXISTS users (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,

    account text NOT NULL UNIQUE, -- 帳號
    password text NOT NULL,
    permission bit(8),        -- 群組, ADM(0),GM(1),Maintenance(2),Commissioner(3),JSHall(4),Disable(?)
    username text,                -- 姓名
    worker_id text,               -- 工號
    title_id integer REFERENCES titles (id) ON DELETE CASCADE,           -- 職稱(in titles table)
    department_id integer REFERENCES departments (id) ON DELETE CASCADE, -- 部門單位(in departments table)
    phone text NOT NULL,
    email text NOT NULL,

    create_at timestamptz NOT NULL DEFAULT NOW(),   -- 創建時間
    login_at timestamptz,            -- 登人時間(最後)
    update_at timestamptz,            -- 修改時間
    extra text                       	-- future usage?
);
-- '{"account":"administrator","password":"fy90676855","username":"i-am-superuser","worker_id":"AA00001","title":"superuser","department":"backend","phone":"0900123456","email":"admin@fika.com","permission":{"storage":[1],"nbits":8}}'

CREATE TABLE IF NOT EXISTS models (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    brand text NOT NULL,
    model text NOT NULL,
    price integer
);

-- 配件分類
CREATE TABLE IF NOT EXISTS accessories (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    item text NOT NULL,
    price integer NOT NULL
);

-- 故障分類
CREATE TABLE IF NOT EXISTS faults (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    item text NOT NULL,
    cost integer NOT NULL
);

-- 工單狀態分類
CREATE TABLE IF NOT EXISTS status (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    flow text NOT NULL         -- 收件 -> 報價 -> 更新 -> 鎖定 -> 退件/完成 
);

--INSERT INTO status(flow) values ('收件');
--INSERT INTO status(flow) values ('報價');

-- 工單
CREATE TABLE IF NOT EXISTS orders (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    issue_at timestamptz NOT NULL DEFAULT NOW(),    -- 開單時間

    -- for future
    issuer_id integer REFERENCES users (id) ON DELETE CASCADE, -- 敲單人員
    sn text NOT NULL UNIQUE, -- 工單號

    department_id integer REFERENCES departments (id) ON DELETE CASCADE, -- 收件地點
    contact_id integer REFERENCES users (id) ON DELETE CASCADE,          -- 直服專員
    customer_name text,           -- 客戶名稱
    customer_phone text NOT NULL, -- 客戶手機
    customer_address text,        -- 客戶地址
    model_id integer REFERENCES models (id) ON DELETE CASCADE,           -- 品牌/型號
    purchase_at date,    -- 購買時間
    accessory_id1 integer REFERENCES accessories (id) ON DELETE CASCADE, -- 配件1
    accessory_id2 integer REFERENCES accessories (id) ON DELETE CASCADE, -- 配件2
    accessory_other text,                                                -- 其它配件
    appearance bit(8) NOT NULL,   -- 外觀
    appearance_other text,        -- 外觀(其它)
    service text,                 -- 服務項目
    fault_id1 integer REFERENCES faults (id) ON DELETE CASCADE,  -- 故障1
    fault_id2 integer REFERENCES faults (id) ON DELETE CASCADE,  -- 故障2
    fault_other text,                                           -- 其它故障
    photo_url text,               -- 照片地址
    remark text,                  -- 備註
    cost integer,                 -- 報價
    prepaid_free integer,         -- 預收款
    confirmed_paid integer,
    warranty_expired bool,
    life_cycle text,
    status_id integer REFERENCES status (id) ON DELETE CASCADE,    -- 工單狀態
    servicer_id integer REFERENCES users (id) ON DELETE CASCADE,        -- 客服專員
    maintainer_id integer REFERENCES users (id) ON DELETE CASCADE,       -- 維保人員
    extra text                       	-- future usage?
);

CREATE TABLE IF NOT EXISTS order_gsheets (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,

    order_id integer REFERENCES orders (id) ON DELETE CASCADE,     -- 工單
    sheet_column text,
    sheet_row integer
);

CREATE TABLE IF NOT EXISTS order_histories (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    change_at timestamptz NOT NULL DEFAULT NOW(),    -- 開單時間

    order_id integer REFERENCES orders (id) ON DELETE CASCADE,     -- 工單
    issuer_id integer REFERENCES users (id) ON DELETE CASCADE, -- 敲單人員
    status_id integer REFERENCES status (id) ON DELETE CASCADE,    -- 工單狀態
    life_cycle text,
    remark text,                  -- 備註
    cost integer
);

-- internal table for user login
CREATE TABLE IF NOT EXISTS sessions (
    session_token BYTEA PRIMARY KEY,
    user_id integer REFERENCES users (id) ON DELETE CASCADE
);
