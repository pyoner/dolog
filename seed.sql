CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS posts (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    published INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS audit_notes (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    note TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

INSERT INTO users (email, name, active)
VALUES
    ('ada@example.com', 'Ada Lovelace', 1),
    ('grace@example.com', 'Grace Hopper', 1),
    ('linus@example.com', 'Linus Torvalds', 0);

INSERT INTO posts (user_id, title, body, published)
VALUES
    (1, 'Analytical Engine Notes', 'Early notes about programmable machines.', 1),
    (2, 'Compiler Design', 'Thoughts on compilers and machine-independent programming.', 1),
    (3, 'Kernel Draft', 'Rough notes about a small operating system kernel.', 0);

INSERT INTO audit_notes (user_id, note)
VALUES
    (1, 'Imported from historical demo dataset'),
    (2, 'Enabled as active maintainer'),
    (3, 'Marked inactive for archive testing');
