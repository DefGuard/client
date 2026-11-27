CREATE TABLE device_identity (
    instance_id INTEGER NOT NULL PRIMARY KEY REFERENCES instance(id) ON DELETE CASCADE,
    core_device_id INTEGER NOT NULL CHECK (core_device_id > 0),
    identity_id TEXT CHECK (identity_id IS NULL OR length(identity_id) > 0),
    material_version INTEGER NOT NULL CHECK (material_version > 0),
    material BLOB NOT NULL CHECK (length(material) > 0)
);
