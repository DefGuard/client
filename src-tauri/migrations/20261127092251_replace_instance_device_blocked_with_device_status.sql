ALTER TABLE instance
ADD COLUMN device_status INTEGER NULL
    CHECK (device_status IN (1, 2, 3));
UPDATE instance SET device_status = 3 WHERE device_blocked;
ALTER TABLE instance DROP COLUMN device_blocked;
