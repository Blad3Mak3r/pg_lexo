# pg_lexo

[![Release](https://img.shields.io/github/v/release/Blad3Mak3r/pg_lexo)](https://github.com/Blad3Mak3r/pg_lexo/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A PostgreSQL extension written in Rust using [pgrx](https://github.com/pgcentralfoundation/pgrx) for generating lexicographic ordering values. This enables efficient reordering of items in database tables without requiring updates to other rows.

## Table of Contents

- [Overview](#overview)
- [Use Cases](#use-cases)
- [Features](#features)
- [Installation](#installation)
  - [From Pre-built Releases](#from-pre-built-releases)
  - [Building from Source](#building-from-source)
- [Usage](#usage)
  - [The `lexo` Schema](#the-lexo-schema)
  - [The `lexo.lexorank` Type](#the-lexolexorank-type)
  - [Available Functions](#available-functions)
  - [Adding a Lexo Column](#adding-a-lexo-column)
  - [Basic Examples](#basic-examples)
  - [Real-World Example: Playlist Ordering](#real-world-example-playlist-ordering)
  - [Advanced Usage: Automatic Position Generation with Triggers](#advanced-usage-automatic-position-generation-with-triggers)
- [Migration from TEXT columns](#migration-from-text-columns)
- [How It Works](#how-it-works)
- [API Reference](#api-reference)
- [Contributing](#contributing)
- [License](#license)

## Overview

`pg_lexo` solves the common problem of maintaining ordered lists in relational databases. Traditional approaches using integer positions require updating multiple rows when inserting or reordering items. This extension uses lexicographic (string-based) positioning, allowing insertions between any two existing positions without modifying other rows.

## Use Cases

This extension is ideal for scenarios where you need to maintain an ordered list in a relational database:

- **Playlist/Queue Management**: Order songs in a playlist, videos in a queue
- **Task Lists & Kanban Boards**: Order tasks within projects or columns
- **Drag-and-Drop Interfaces**: Any UI where items can be reordered
- **Document Sections**: Order chapters, paragraphs, or any hierarchical content
- **E-commerce**: Order products in categories, images in galleries

## Features

- **Dedicated `lexo` Schema**: All functions and types are organized under the `lexo` schema
- **Custom `lexo.lexorank` Type**: Native PostgreSQL type with proper comparison operators (no `COLLATE "C"` needed!)
- **Helper Function**: `lexo.add_lexo_column_to()` automatically adds properly configured columns
- **Base62 Encoding**: Uses 62 characters (0-9, A-Z, a-z) for compact, efficient position strings
- **Efficient Insertions**: Insert items between any two positions without updating other rows
- **Unlimited Insertions**: Can always generate a position between any two existing positions
- **Backwards Compatible**: TEXT-based functions available for legacy code (`lexo.first_text()`, etc.)
- **Cross-Platform**: Supports Linux x64
- **PostgreSQL Compatibility**: Works with PostgreSQL 16, 17, and 18

## Installation

> **Note**: PostgreSQL extensions must be compiled separately for each major PostgreSQL version due to ABI differences between versions.

### From Pre-built Releases

Download the pre-built extension from the [Releases](https://github.com/Blad3Mak3r/pg_lexo/releases) page.

#### Quick Install (Recommended)

```bash
# Auto-detect PostgreSQL version
curl -sSL https://raw.githubusercontent.com/Blad3Mak3r/pg_lexo/main/install.sh | sudo sh

# Or specify PostgreSQL version explicitly (e.g., 17)
curl -sSL https://raw.githubusercontent.com/Blad3Mak3r/pg_lexo/main/install.sh | sudo sh -s 17
```

#### Manual Installation

```bash
# Download and extract (replace VERSION and PG_VERSION as needed)
wget https://github.com/Blad3Mak3r/pg_lexo/releases/download/vVERSION/pg_lexo-VERSION-linux-x64-pgPG_VERSION.tar.gz
tar -xzf pg_lexo-VERSION-linux-x64-pgPG_VERSION.tar.gz

# Copy files to PostgreSQL directories
sudo cp pg_lexo.so $(pg_config --pkglibdir)/
sudo cp pg_lexo.control $(pg_config --sharedir)/extension/
sudo cp pg_lexo--VERSION.sql $(pg_config --sharedir)/extension/
```

#### Enable the Extension

```sql
CREATE EXTENSION pg_lexo;
```

### Building from Source

#### Prerequisites

- Rust (latest stable) - [Install Rust](https://rustup.rs/)
- PostgreSQL 16-18 with development headers
- [cargo-pgrx](https://github.com/pgcentralfoundation/pgrx)

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/Blad3Mak3r/pg_lexo.git
cd pg_lexo

# Install cargo-pgrx
cargo install cargo-pgrx --version "0.16.1" --locked

# Initialize pgrx
cargo pgrx init

# Build and install
cargo pgrx install --pg-config $(which pg_config)
```

#### Run Tests

```bash
# Run unit tests
cargo test

# Run PostgreSQL integration tests
cargo pgrx test pg16  # Replace with your PG version
```

## Usage

### The `lexo` Schema

pg_lexo provides all functions and types under the `lexo` schema.

```sql
-- Create the extension
CREATE EXTENSION pg_lexo;

-- Create a table with a lexo.lexorank position column
CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    position lexo.lexorank NOT NULL
);

-- Or use the helper function to add the column
SELECT lexo.add_lexo_column_to('items', 'position');

-- Insert items with lexo positions
INSERT INTO items (name, position) VALUES ('First', lexo.first());
INSERT INTO items (name, position) VALUES ('Second', lexo.next('items', 'position', NULL, NULL));

-- Query in order (no COLLATE needed with lexo.lexorank!)
SELECT * FROM items ORDER BY position;
```

### The `lexo.lexorank` Type

The `lexo.lexorank` type is a custom PostgreSQL type that provides:

- **Proper ordering**: No need for `COLLATE "C"` - just use `ORDER BY position`
- **Comparison operators**: `=`, `<>`, `<`, `<=`, `>`, `>=`
- **Index support**: B-tree and hash indexes work out of the box
- **Type safety**: Ensures only valid Base62 values are stored

```sql
-- Create a table with the lexorank type
CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    position lexo.lexorank NOT NULL
);

-- Create an index (optional, for large tables)
CREATE INDEX idx_tasks_position ON tasks (position);

-- Insert and query
INSERT INTO tasks (title, position) VALUES ('Task 1', lexo.first());
INSERT INTO tasks (title, position) VALUES ('Task 2', lexo.after(lexo.first()));

SELECT * FROM tasks ORDER BY position;
```

### Available Functions

All functions are in the `lexo` schema:

| Function | Description |
|----------|-------------|
| `lexo.first()` | Returns the initial position (`'H'`) as `lexo.lexorank` |
| `lexo.after(position)` | Returns a position after the given position |
| `lexo.before(position)` | Returns a position before the given position |
| `lexo.between(before, after)` | Returns a position between two positions (either can be NULL) |
| `lexo.next(table, column, filter_col, filter_val)` | Returns the next position after the maximum in a table |
| `lexo.add_lexo_column_to(table, column)` | Adds a `lexo.lexorank` column to a table |
| `lexo.rebalance(table, column, filter_col, filter_val)` | Rebalances positions in a table for optimal spacing |

#### Backwards-Compatible TEXT Functions

For legacy code or when you need TEXT output:

| Function | Description |
|----------|-------------|
| `lexo.first_text()` | Returns the initial position as `TEXT` |
| `lexo.after_text(TEXT)` | Returns a position after the given TEXT position |
| `lexo.before_text(TEXT)` | Returns a position before the given TEXT position |
| `lexo.between_text(TEXT, TEXT)` | Returns a position between two TEXT positions |

### Adding a Lexo Column

The easiest way to add a properly configured position column is with `lexo.add_lexo_column_to()`:

```sql
-- Create your table
CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

-- Add a position column with the lexo.lexorank type
SELECT lexo.add_lexo_column_to('items', 'position');

-- This is equivalent to:
-- ALTER TABLE items ADD COLUMN position lexo.lexorank;
```

### Basic Examples

```sql
-- Create the extension
CREATE EXTENSION pg_lexo;

-- Get the first position for a new list
SELECT lexo.first();
-- Returns: 'H' (as lexo.lexorank)

-- Get a position after 'H'
SELECT lexo.after(lexo.first());
-- Returns: 'I'

-- Get a position before 'H'
SELECT lexo.before(lexo.first());
-- Returns: 'Gz'

-- Get a position between two existing positions
SELECT lexo.between('A'::lexo.lexorank, 'Z'::lexo.lexorank);
-- Returns: 'N'

-- Get first position (both NULL)
SELECT lexo.between(NULL, NULL);
-- Returns: 'H'

-- Get position at the end (after = NULL)
SELECT lexo.between(lexo.first(), NULL);
-- Returns: 'I'

-- Get position at the beginning (before = NULL)
SELECT lexo.between(NULL, lexo.first());
-- Returns: 'Gz'

-- Get the next position after the maximum in a table column
SELECT lexo.next('playlist_songs', 'position', NULL, NULL);
-- Returns: the next position after the current maximum, or 'H' if table is empty

-- Get the next position for a specific collection (filtered)
SELECT lexo.next('collection_songs', 'position', 'collection_id', 'collection-uuid-here');
-- Returns: the next position for that specific collection
```

### Real-World Example: Playlist Ordering

```sql
-- Create a table for playlist songs
CREATE TABLE playlist_songs (
    playlist_id TEXT NOT NULL,
    song_id TEXT NOT NULL,
    position lexo.lexorank NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (playlist_id, song_id)
);

-- Create an index for efficient ordering queries
CREATE INDEX idx_playlist_position ON playlist_songs (playlist_id, position);

-- Add the first song to playlist 'playlist-1'
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES ('playlist-1', 'song-101', lexo.first());

-- Add a second song at the end
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES ('playlist-1', 'song-102', lexo.next('playlist_songs', 'position', 'playlist_id', 'playlist-1'));

-- Add a third song at the end
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES ('playlist-1', 'song-103', lexo.next('playlist_songs', 'position', 'playlist_id', 'playlist-1'));

-- Insert a song between the first and second songs
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES ('playlist-1', 'song-104', (
    SELECT lexo.between(
        (SELECT position FROM playlist_songs WHERE playlist_id = 'playlist-1' AND song_id = 'song-101'),
        (SELECT position FROM playlist_songs WHERE playlist_id = 'playlist-1' AND song_id = 'song-102')
    )
));

-- Query songs in order (no COLLATE needed!)
SELECT song_id, position
FROM playlist_songs
WHERE playlist_id = 'playlist-1'
ORDER BY position;

-- Move song-103 to the beginning
UPDATE playlist_songs
SET position = (
    SELECT lexo.before(MIN(position))
    FROM playlist_songs
    WHERE playlist_id = 'playlist-1'
)
WHERE playlist_id = 'playlist-1' AND song_id = 'song-103';
```

### Advanced Usage: Automatic Position Generation with Triggers

While you can manually specify positions using `lexo.first()` or `lexo.next()`, you might want to automatically generate positions when inserting new rows. PostgreSQL triggers are perfect for this use case.

#### Basic Trigger: Auto-generate positions

This trigger automatically generates the next position when inserting a row without specifying a position:

```sql
-- Create the trigger function
CREATE OR REPLACE FUNCTION auto_lexo_position()
RETURNS TRIGGER AS $$
BEGIN
    -- Only generate position if not provided
    IF NEW.position IS NULL THEN
        NEW.position := lexo.next(TG_TABLE_NAME, 'position', NULL, NULL);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create a table with the trigger
CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    position lexo.lexorank
);

CREATE TRIGGER set_position_before_insert
    BEFORE INSERT ON tasks
    FOR EACH ROW
    EXECUTE FUNCTION auto_lexo_position();

-- Now you can insert without specifying position
INSERT INTO tasks (title) VALUES ('First task');   -- position: 'H'
INSERT INTO tasks (title) VALUES ('Second task');  -- position: 'I'
INSERT INTO tasks (title) VALUES ('Third task');   -- position: 'J'

-- Or override the auto-generated position
INSERT INTO tasks (title, position) VALUES ('Between first and second', 
    lexo.between(
        (SELECT position FROM tasks WHERE title = 'First task'),
        (SELECT position FROM tasks WHERE title = 'Second task')
    )
);

-- Query in order
SELECT * FROM tasks ORDER BY position;
```

#### Advanced Trigger: Partitioned Lists

For tables with multiple independent lists (e.g., songs in different playlists), you need a trigger that respects the partition:

```sql
-- Create a smarter trigger function for partitioned data
CREATE OR REPLACE FUNCTION auto_lexo_position_partitioned()
RETURNS TRIGGER AS $$
DECLARE
    partition_column TEXT;
    partition_value TEXT;
BEGIN
    -- Only generate position if not provided
    IF NEW.position IS NULL THEN
        -- Get the partition column name from trigger arguments
        -- Usage: CREATE TRIGGER ... EXECUTE FUNCTION auto_lexo_position_partitioned('playlist_id')
        IF TG_NARGS > 0 THEN
            partition_column := TG_ARGV[0];
            
            -- Get the partition value from the NEW row
            EXECUTE format('SELECT ($1).%I::text', partition_column) 
                INTO partition_value 
                USING NEW;
            
            -- Generate next position for this specific partition
            NEW.position := lexo.next(
                TG_TABLE_NAME, 
                'position', 
                partition_column, 
                partition_value
            );
        ELSE
            -- Fallback to non-partitioned behavior
            NEW.position := lexo.next(TG_TABLE_NAME, 'position', NULL, NULL);
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Example: Playlist songs with automatic positioning
CREATE TABLE playlist_songs (
    playlist_id TEXT NOT NULL,
    song_id TEXT NOT NULL,
    position lexo.lexorank,
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (playlist_id, song_id)
);

CREATE INDEX idx_playlist_position ON playlist_songs (playlist_id, position);

-- Create trigger with partition column argument
CREATE TRIGGER set_playlist_song_position
    BEFORE INSERT ON playlist_songs
    FOR EACH ROW
    EXECUTE FUNCTION auto_lexo_position_partitioned('playlist_id');

-- Now inserting is simple - positions are auto-generated per playlist
INSERT INTO playlist_songs (playlist_id, song_id) 
VALUES 
    ('playlist-1', 'song-101'),  -- Gets position 'H' in playlist-1
    ('playlist-1', 'song-102'),  -- Gets position 'I' in playlist-1
    ('playlist-2', 'song-201'),  -- Gets position 'H' in playlist-2
    ('playlist-1', 'song-103'),  -- Gets position 'J' in playlist-1
    ('playlist-2', 'song-202');  -- Gets position 'I' in playlist-2

-- Each playlist maintains its own ordering
SELECT playlist_id, song_id, position 
FROM playlist_songs 
WHERE playlist_id = 'playlist-1' 
ORDER BY position;
-- Results: song-101 (H), song-102 (I), song-103 (J)

SELECT playlist_id, song_id, position 
FROM playlist_songs 
WHERE playlist_id = 'playlist-2' 
ORDER BY position;
-- Results: song-201 (H), song-202 (I)
```

#### Benefits of Using Triggers

- **Simplified Application Code**: No need to call `lexo.next()` in your application
- **Consistent Behavior**: Position generation logic is centralized in the database
- **Optional Override**: You can still manually specify positions when needed
- **Type Safety**: Works seamlessly with the `lexo.lexorank` type
- **Performance**: Automatic positions are generated only when needed

#### Important Notes

1. **NULL Columns**: Make sure your position column allows NULL if you want the trigger to work:
   ```sql
   position lexo.lexorank  -- Allows NULL (trigger will fill it)
   -- vs
   position lexo.lexorank NOT NULL  -- Must provide value or DEFAULT
   ```

2. **DEFAULT vs TRIGGER**: You can't use both a DEFAULT value and expect the trigger to work for NULL values. Choose one approach:
   - Use a trigger for dynamic position generation (recommended)
   - Use `DEFAULT lexo.first()` for a static default (always `'H'`, not dynamically calculated)

3. **Performance Considerations**: The trigger queries the table to find the maximum position. For very large tables, consider:
   - Adding appropriate indexes on the position column
   - Using the partitioned trigger for filtered queries
   - Periodically running `lexo.rebalance(table, column, filter_col, filter_val)` to keep positions optimal

## Migration from TEXT columns

If you're upgrading from a previous version that used `TEXT COLLATE "C"` columns:

### Option 1: Keep using TEXT (recommended for existing data)

Use the backwards-compatible `_text` functions:

```sql
-- These functions work with TEXT columns
SELECT lexo.first_text();           -- Returns TEXT
SELECT lexo.after_text('H');        -- Returns TEXT
SELECT lexo.before_text('H');       -- Returns TEXT
SELECT lexo.between_text('A', 'Z'); -- Returns TEXT
```

### Option 2: Migrate to lexo.lexorank

```sql
-- 1. Add a new lexo.lexorank column
ALTER TABLE your_table ADD COLUMN position_new lexo.lexorank;

-- 2. Copy data (lexo.lexorank accepts the same Base62 strings)
UPDATE your_table SET position_new = position::lexo.lexorank;

-- 3. Drop the old column and rename
ALTER TABLE your_table DROP COLUMN position;
ALTER TABLE your_table RENAME COLUMN position_new TO position;
```

## How It Works

### Base62 Encoding

The extension uses a Base62 character set for position strings:

```
0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz
```

This provides 62 possible characters per position, allowing for efficient string representation while maintaining proper lexicographic ordering.

### Position Generation Algorithm

1. **First Position**: Returns `'H'` (a position in the middle of Base62)
2. **After Position**: Generates the next position after the current one
3. **Before Position**: Generates a position before the current one
4. **Between Positions**: Finds a position between two given positions

When the algorithm needs more precision, it extends the string by appending additional characters.

### Why `lexo.lexorank` instead of TEXT?

The `lexo.lexorank` type provides several advantages over `TEXT COLLATE "C"`:

1. **Type Safety**: Only valid Base62 values can be stored
2. **No Collation Issues**: Ordering works correctly without specifying `COLLATE "C"`
3. **Better Performance**: Custom comparison operators optimized for lexicographic ordering
4. **Cleaner Queries**: No need to remember collation specifications

### Why Lexicographic Ordering?

Traditional integer-based ordering requires updating all positions when inserting:

```sql
-- Integer approach: Insert at position 2 requires updating positions 2, 3, 4...
UPDATE items SET position = position + 1 WHERE position >= 2;
INSERT INTO items (position) VALUES (2);
```

With lexicographic ordering:

```sql
-- Lexicographic approach: Just insert between existing positions
INSERT INTO items (position) VALUES (
    lexo.between(
        (SELECT position FROM items WHERE id = 1),
        (SELECT position FROM items WHERE id = 2)
    )
);
```

## API Reference

### `lexo.first()`

Returns the initial position for starting a new ordered list.

**Returns**: `lexo.lexorank` - The position `'H'`

**Example**:
```sql
SELECT lexo.first();  -- Returns 'H'
```

### `lexo.after(current lexo.lexorank)`

Generates a position that comes after the given position.

**Parameters**:
- `current` - The current position

**Returns**: `lexo.lexorank` - A position greater than `current`

**Example**:
```sql
SELECT lexo.after(lexo.first());  -- Returns 'I'
```

### `lexo.before(current lexo.lexorank)`

Generates a position that comes before the given position.

**Parameters**:
- `current` - The current position

**Returns**: `lexo.lexorank` - A position less than `current`

**Example**:
```sql
SELECT lexo.before(lexo.first());  -- Returns 'Gz'
```

### `lexo.between(before lexo.lexorank, after lexo.lexorank)`

Generates a position between two existing positions. Either parameter can be NULL.

**Parameters**:
- `before` - The position before the new position (NULL for beginning)
- `after` - The position after the new position (NULL for end)

**Returns**: `lexo.lexorank` - A position between `before` and `after`

**Behavior**:
- `(NULL, NULL)` - Returns the first position (`'H'`)
- `(position, NULL)` - Returns a position after the given position
- `(NULL, position)` - Returns a position before the given position
- `(pos1, pos2)` - Returns a position between pos1 and pos2

**Example**:
```sql
SELECT lexo.between(NULL, NULL);    -- Returns 'H'
SELECT lexo.between('A'::lexo.lexorank, 'Z'::lexo.lexorank);  -- Returns 'N'
```

### `lexo.next(table_name, column_name, filter_column, filter_value)`

Returns the next position after the maximum in a table column.

**Parameters**:
- `table_name` - The name of the table (can be schema-qualified)
- `column_name` - The name of the position column
- `filter_column` - Optional: column to filter by (e.g., 'collection_id')
- `filter_value` - Optional: value to filter by

**Returns**: `lexo.lexorank` - A position after the maximum, or `'H'` if table is empty

**Example**:
```sql
-- Get next position for entire table
SELECT lexo.next('items', 'position', NULL, NULL);

-- Get next position for a specific collection
SELECT lexo.next('collection_songs', 'position', 'collection_id', 'abc-123');
```

### `lexo.add_lexo_column_to(table_name, column_name)`

Adds a `lexo.lexorank` column to an existing table.

**Parameters**:
- `table_name` - The name of the table (can be schema-qualified)
- `column_name` - The name of the new column

**Example**:
```sql
SELECT lexo.add_lexo_column_to('items', 'position');
-- Equivalent to: ALTER TABLE items ADD COLUMN position lexo.lexorank;
```

### `lexo.rebalance(table_name, column_name, filter_column, filter_value)`

Rebalances positions in a table to optimize spacing between items.

**Parameters**:
- `table_name` - The name of the table (can be schema-qualified)
- `column_name` - The name of the position column
- `filter_column` - Optional: column to filter by
- `filter_value` - Optional: value to filter by

**Returns**: `BIGINT` - Number of rows rebalanced

**Example**:
```sql
-- Rebalance all positions in a table
SELECT lexo.rebalance('items', 'position', NULL, NULL);

-- Rebalance positions for a specific playlist
SELECT lexo.rebalance('playlist_songs', 'position', 'playlist_id', 'abc-123');
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
