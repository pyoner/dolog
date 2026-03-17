CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users (email, name, active)
VALUES
    ('ada@example.com', 'Ada Lovelace', 1),
    ('grace@example.com', 'Grace Hopper', 1),
    ('linus@example.com', 'Linus Torvalds', 0);
