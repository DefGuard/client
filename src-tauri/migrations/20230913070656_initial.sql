CREATE TABLE instance (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  uuid TEXT NOT NULL,
  name TEXT NOT NULL,
  url TEXT NOT NULL
);
CREATE TABLE location (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  instance_id INTEGER NOT NULL,
  network_id INTEGER NOT NULL, -- Native id of network in defguard system
  name TEXT NOT NULL,
  address TEXT NOT NULL,
  pubkey TEXT NOT NULL,
  endpoint TEXT NOT NULL,
  allowed_ips TEXT NOT NULL,
  dns TEXT,
  FOREIGN KEY (instance_id) REFERENCES instance(id)
);

-- Create the Connection table
CREATE TABLE connection (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  location_id INTEGER NOT NULL,
  connected_from TEXT NOT NULL, -- Renamed from 'from' as it reserved
  start TIMESTAMP NOT NULL,
  end TIMESTAMP NOT NULL,
  FOREIGN KEY (location_id) REFERENCES location(id)
);

-- Create the LocationStats table
CREATE TABLE  location_stats (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  location_id INTEGER NOT NULL,
  upload INTEGER NOT NULL,
  download INTEGER NOT NULL,
  last_handshake INTEGER NOT NULL,
  collected_at TIMESTAMP NOT NULL,
  FOREIGN KEY (location_id) REFERENCES location(id)
);

-- Create the LocationStats table
CREATE TABLE  wireguard_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  instance_id INTEGER NOT NULL,
  pubkey TEXT NOT NULL,
  prvkey TEXT NOT NULL,
  FOREIGN KEY (instance_id) REFERENCES instance(id)
);
