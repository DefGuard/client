CREATE TABLE tunnel (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    pubkey TEXT NOT NULL,
    prvkey TEXT NOT NULL,
    address TEXT NOT NULL,
    server_pubkey TEXT NOT NULL,
    allowed_ips TEXT,
    endpoint TEXT NOT NULL,
    dns TEXT,
    route_all_traffic BOOLEAN NOT NULL,
    persistent_keep_alive INTEGER NOT NULL,
    pre_up TEXT,
    post_up TEXT,
    pre_down TEXT,
    post_down TEXT
);

CREATE TABLE tunnel_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tunnel_id BIGINT NOT NULL,
    upload BIGINT NOT NULL,
    download BIGINT NOT NULL,
    last_handshake BIGINT NOT NULL,
    collected_at TIMESTAMP NOT NULL,
    listen_port INTEGER NOT NULL,
    persistent_keepalive_interval INTEGER NOT NULL,
    FOREIGN KEY (tunnel_id) REFERENCES tunnel(id) ON DELETE CASCADE
);
CREATE TABLE tunnel_connection (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  tunnel_id INTEGER NOT NULL,
  connected_from TEXT NOT NULL, -- Renamed from 'from' as it reserved
  start TIMESTAMP NOT NULL,
  end TIMESTAMP NOT NULL,
  FOREIGN KEY (tunnel_id) REFERENCES tunnel(id) ON DELETE CASCADE
);
