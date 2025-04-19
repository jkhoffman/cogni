-- Create memory entries table
CREATE TABLE memory_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Create index on session_id for faster lookups
CREATE INDEX idx_memory_entries_session_id ON memory_entries(session_id);

-- Create index on timestamp for chronological ordering
CREATE INDEX idx_memory_entries_timestamp ON memory_entries(timestamp); 