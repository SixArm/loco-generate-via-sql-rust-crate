-- A small notes app in SQLite.
-- Demonstrates: INTEGER PRIMARY KEY AUTOINCREMENT (skipped by name),
-- TEXT (with and without NOT NULL), foreign key rule 1, BLOB.

CREATE TABLE notebooks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT
);

CREATE TABLE notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    notebook_id INTEGER NOT NULL REFERENCES notebooks(id),
    title TEXT NOT NULL,
    body TEXT,
    pinned INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    note_id INTEGER NOT NULL REFERENCES notes(id),
    filename TEXT NOT NULL,
    data BLOB NOT NULL
);
