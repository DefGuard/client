PRAGMA foreign_keys=off;

-- Migrate table wireguard_keys

ALTER TABLE wireguard_keys RENAME TO _wireguard_keys_old;

CREATE TABLE wireguard_keys
(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    instance_id INTEGER NOT NULL,
    pubkey TEXT NOT NULL,
    prvkey TEXT NOT NULL,
    FOREIGN KEY (instance_id) REFERENCES instance(id) ON DELETE CASCADE
);

INSERT INTO wireguard_keys SELECT * FROM _wireguard_keys_old;

DROP TABLE _wireguard_keys_old;

-- Migrate location

ALTER TABLE location RENAME TO _location_old;

CREATE TABLE location
(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  instance_id INTEGER NOT NULL,
  network_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  address TEXT NOT NULL,
  pubkey TEXT NOT NULL,
  endpoint TEXT NOT NULL,
  allowed_ips TEXT NOT NULL,
  dns TEXT,
  route_all_traffic BOOLEAN NOT NULL DEFAULT false,
  FOREIGN KEY (instance_id) REFERENCES instance(id) ON DELETE CASCADE
);

INSERT INTO location SELECT * FROM _location_old;

DROP TABLE _location_old;

-- update location_stats

ALTER TABLE location_stats RENAME TO _location_stats_old;

CREATE TABLE  location_stats (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  location_id INTEGER NOT NULL,
  upload INTEGER NOT NULL,
  download INTEGER NOT NULL,
  last_handshake INTEGER NOT NULL,
  collected_at TIMESTAMP NOT NULL,
  listen_port INTEGER NOT NULL DEFAULT 0,
  persistent_keepalive_interval INTEGER NULL,
  FOREIGN KEY (location_id) REFERENCES location(id) ON DELETE CASCADE
);

INSERT INTO location_stats SELECT * FROM _location_stats_old;

DROP TABLE _location_stats_old;

-- restore index
CREATE INDEX idx_collected_location ON location_stats (collected_at, location_id);

-- Migrate connections

ALTER TABLE connection RENAME TO _connection_old;

CREATE TABLE connection (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  location_id INTEGER NOT NULL,
  connected_from TEXT NOT NULL, -- Renamed from 'from' as it reserved
  start TIMESTAMP NOT NULL,
  end TIMESTAMP NOT NULL,
  FOREIGN KEY (location_id) REFERENCES location(id) ON DELETE CASCADE
);

INSERT INTO connection SELECT * FROM _connection_old;

DROP TABLE _connection_old;

PRAGMA foreign_keys=on;