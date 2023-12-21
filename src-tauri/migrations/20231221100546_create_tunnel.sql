CREATE TABLE tunnel (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    pubkey TEXT NOT NULL,
    prvkey TEXT NOT NULL,
    address TEXT NOT NULL,
    server_pubkey TEXT NOT NULL,
    allowed_ips TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    dns TEXT,
    route_all_traffic BOOLEAN NOT NULL,
    persistent_keep_alive INTEGER NOT NULL,
    pre_up TEXT,
    post_up TEXT,
    pre_down TEXT,
    post_down TEXT
);
