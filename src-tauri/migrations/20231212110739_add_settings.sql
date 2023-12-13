-- Add migration script here
CREATE TABLE settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    theme TEXT DEFAULT 'light' NOT NULL,
    log_level TEXT DEFAULT 'info' NOT NULL,
    tray_icon_theme TEXT DEFAULT 'color' NOT NULL,
);