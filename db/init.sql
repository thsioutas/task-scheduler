CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY,
    status TEXT NOT NULL,
    error_message TEXT
);