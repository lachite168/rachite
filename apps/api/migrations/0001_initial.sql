CREATE TABLE users (id TEXT PRIMARY KEY NOT NULL, email TEXT NOT NULL UNIQUE, password_hash TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE items (
id TEXT PRIMARY KEY NOT NULL, owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 200), description TEXT NOT NULL DEFAULT '',
completed BOOLEAN NOT NULL DEFAULT FALSE, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE INDEX items_owner_created ON items(owner_id, created_at DESC);
