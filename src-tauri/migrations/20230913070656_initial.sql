CREATE TABLE instance (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL
);
CREATE TABLE location (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  instance_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  address TEXT NOT NULL,
  pubkey TEXT NOT NULL,
  endpoint TEXT NOT NULL,
  allowed_ips TEXT,
  FOREIGN KEY (instance_id) REFERENCES instance(id)
);

-- Create the Connection table
CREATE TABLE connection (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  location_id INTEGER NOT NULL,
  connected_from TEXT NOT NULL, -- Renamed from 'from' as it reservedr
  start INTEGER,
  end INTEGER,
  FOREIGN KEY (location_id) REFERENCES location(id)
);

-- Create the LocationStats table
CREATE TABLE  location_stats (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  location_id INTEGER NOT NULL,
  upload INTEGER,
  download INTEGER,
  last_handshake INTEGER,
  collected_at INTEGER,
  FOREIGN KEY (location_id) REFERENCES location(id)
);
