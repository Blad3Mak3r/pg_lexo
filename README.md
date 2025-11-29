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
  - [Available Functions](#available-functions)
  - [Basic Examples](#basic-examples)
  - [Real-World Example: Playlist Ordering](#real-world-example-playlist-ordering)
- [Important: Collation for Correct Ordering](#important-collation-for-correct-ordering)
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

- **Base62 Encoding**: Uses 62 characters (0-9, A-Z, a-z) for compact, efficient position strings
- **Lexicographic Ordering**: Positions sort correctly using standard string comparison (`ORDER BY position COLLATE "C"`)
- **Efficient Insertions**: Insert items between any two positions without updating other rows
- **Unlimited Insertions**: Can always generate a position between any two existing positions
- **Cross-Platform**: Supports Linux x64
- **PostgreSQL Compatibility**: Works with PostgreSQL 16, 17, and 18

## Installation

> **Note**: PostgreSQL extensions must be compiled separately for each major PostgreSQL version due to ABI (Application Binary Interface) differences between versions. This means you need to download or build the extension specifically for your PostgreSQL version (e.g., pg16, pg17, pg18). A single binary cannot be compatible with multiple PostgreSQL major versions.

### From Pre-built Releases

Download the pre-built extension for your platform from the [Releases](https://github.com/Blad3Mak3r/pg_lexo/releases) page.

#### Linux x64

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

# Initialize pgrx (this downloads and configures PostgreSQL)
cargo pgrx init

# Build and install for your PostgreSQL version (e.g., pg16)
cargo pgrx install --pg-config $(which pg_config)

# Or package for distribution
cargo pgrx package
```

#### Run Tests

```bash
# Run unit tests
cargo test

# Run PostgreSQL integration tests
cargo pgrx test pg16  # Replace with your PG version
```

## Usage

### Available Functions

All functions are available under the `lexo` schema (similar to how pg_cron uses the `cron` schema).

| Function | Description |
|----------|-------------|
| `lexo.first()` | Returns the initial position for a new list (`'V'`) |
| `lexo.after(position TEXT)` | Returns a position that comes after the given position |
| `lexo.before(position TEXT)` | Returns a position that comes before the given position |
| `lexo.between(before TEXT, after TEXT)` | Returns a position between two positions (either can be NULL) |
| `lexo.next_on_table(table_name TEXT, column_name TEXT)` | Returns the next position after the maximum in a table column |

### Basic Examples

```sql
-- Create the extension
CREATE EXTENSION pg_lexo;

-- Get the first position for a new list
SELECT lexo.first();
-- Returns: 'V'

-- Get a position after 'V'
SELECT lexo.after('V');
-- Returns: 'k' (midpoint between 'V' and 'z')

-- Get a position before 'V'
SELECT lexo.before('V');
-- Returns: 'B' (midpoint between '0' and 'V')

-- Get a position between two existing positions
SELECT lexo.between('A', 'Z');
-- Returns: 'N' (midpoint)

-- Get first position (both NULL)
SELECT lexo.between(NULL, NULL);
-- Returns: 'V'

-- Get position at the end (after = NULL)
SELECT lexo.between('V', NULL);
-- Returns: 'k'

-- Get position at the beginning (before = NULL)
SELECT lexo.between(NULL, 'V');
-- Returns: 'B'

-- Get the next position after the maximum in a table column
-- (useful for appending to the end of an ordered list)
SELECT lexo.next_on_table('playlist_songs', 'position');
-- Returns: the next position after the current maximum, or 'V' if table is empty
```

### Real-World Example: Playlist Ordering

```sql
-- Create a table for playlist songs
CREATE TABLE playlist_songs (
    playlist_id INTEGER NOT NULL,
    song_id INTEGER NOT NULL,
    position TEXT COLLATE "C" NOT NULL,  -- Use C collation for correct ordering
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (playlist_id, song_id)
);

-- Create an index for efficient ordering queries
CREATE INDEX idx_playlist_position ON playlist_songs (playlist_id, position);

-- Add the first song to playlist 1
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES (1, 101, lexo.first());

-- Add a second song at the end (using next_on_table for simplicity)
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES (1, 102, lexo.next_on_table('playlist_songs', 'position'));

-- Add a third song at the end
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES (1, 103, lexo.next_on_table('playlist_songs', 'position'));

-- Insert a song between the first and second songs
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES (1, 104, (
    SELECT lexo.between(
        (SELECT position FROM playlist_songs WHERE playlist_id = 1 AND song_id = 101),
        (SELECT position FROM playlist_songs WHERE playlist_id = 1 AND song_id = 102)
    )
));

-- Query songs in order
SELECT song_id, position
FROM playlist_songs
WHERE playlist_id = 1
ORDER BY position;

-- Result:
-- song_id | position
-- --------|----------
--     101 | V
--     104 | c        (inserted between 101 and 102)
--     102 | k
--     103 | u

-- Move song 103 to the beginning
UPDATE playlist_songs
SET position = (
    SELECT lexo.before(MIN(position))
    FROM playlist_songs
    WHERE playlist_id = 1
)
WHERE playlist_id = 1 AND song_id = 103;

-- Query songs in new order
SELECT song_id, position
FROM playlist_songs
WHERE playlist_id = 1
ORDER BY position;

-- Result:
-- song_id | position
-- --------|----------
--     103 | B        (now at the beginning)
--     101 | V
--     104 | c
--     102 | k
```

## Important: Collation for Correct Ordering

> **⚠️ Critical**: For lexicographic ordering to work correctly, you **must** use the `C` collation (also known as `POSIX` collation) when ordering by position columns.

### Why Collation Matters

PostgreSQL's default collation is locale-aware, which means it may sort characters differently based on your database's locale settings. For example, in some locales, uppercase and lowercase letters may be sorted together, which would break the expected lexicographic ordering of pg_lexo positions.

The `C` collation (or `POSIX`) uses byte-value ordering, which ensures that:
- `'0'` < `'9'` < `'A'` < `'Z'` < `'a'` < `'z'`

This is exactly what pg_lexo expects for correct ordering.

### Option 1: Define Column with COLLATE "C" (Recommended)

The best approach is to define your position column with the `C` collation:

```sql
CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    position TEXT COLLATE "C" NOT NULL
);

-- Now ORDER BY works correctly without specifying collation each time
SELECT * FROM items ORDER BY position;
```

### Option 2: Use COLLATE "C" in ORDER BY

If you can't change the column definition, specify the collation in your queries:

```sql
-- Always use COLLATE "C" when ordering by position
SELECT * FROM items ORDER BY position COLLATE "C";
```

### Option 3: Create Index with COLLATE "C"

For better performance with the collation, create an index that uses the `C` collation:

```sql
-- Create index with C collation for efficient ordering
CREATE INDEX idx_items_position ON items (position COLLATE "C");

-- Queries using COLLATE "C" will use this index
SELECT * FROM items ORDER BY position COLLATE "C";
```

### Complete Example with Correct Collation

```sql
-- Create table with proper collation
CREATE TABLE playlist_songs (
    playlist_id INTEGER NOT NULL,
    song_id INTEGER NOT NULL,
    position TEXT COLLATE "C" NOT NULL,  -- Use C collation
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (playlist_id, song_id)
);

-- Create index for efficient ordering
CREATE INDEX idx_playlist_position ON playlist_songs (playlist_id, position);

-- Insert songs
INSERT INTO playlist_songs (playlist_id, song_id, position) VALUES
    (1, 101, lexo.first()),
    (1, 102, lexo.next_on_table('playlist_songs', 'position')),
    (1, 103, lexo.next_on_table('playlist_songs', 'position'));

-- Query with correct ordering (no COLLATE needed since column has C collation)
SELECT song_id, position
FROM playlist_songs
WHERE playlist_id = 1
ORDER BY position;
```

## How It Works

### Base62 Encoding

The extension uses a Base62 character set for position strings:

```
0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz
```

This provides 62 possible characters per position, allowing for efficient string representation while maintaining proper lexicographic ordering.

### Position Generation Algorithm

1. **First Position**: Returns `'V'` (the midpoint of Base62, index 31)
2. **After Position**: Finds the midpoint between the current position and the maximum (`'z'`)
3. **Before Position**: Finds the midpoint between the minimum (`'0'`) and the current position
4. **Between Positions**: Finds the midpoint between two given positions

When the midpoint would result in the same character, the algorithm extends the string by appending additional characters.

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

Returns the initial position string for starting a new ordered list.

**Returns**: `TEXT` - The position string `'V'`

**Example**:
```sql
SELECT lexo.first();  -- Returns 'V'
```

### `lexo.after(current TEXT)`

Generates a position that lexicographically comes after the given position.

**Parameters**:
- `current` - The current position string

**Returns**: `TEXT` - A position string greater than `current`

**Example**:
```sql
SELECT lexo.after('V');  -- Returns 'k'
```

### `lexo.before(current TEXT)`

Generates a position that lexicographically comes before the given position.

**Parameters**:
- `current` - The current position string

**Returns**: `TEXT` - A position string less than `current`

**Example**:
```sql
SELECT lexo.before('V');  -- Returns 'B'
```

### `lexo.between(before TEXT, after TEXT)`

Generates a position between two existing positions. Either parameter can be NULL.

**Parameters**:
- `before` - The position before the new position (NULL for beginning)
- `after` - The position after the new position (NULL for end)

**Returns**: `TEXT` - A position string between `before` and `after`

**Behavior**:
- `(NULL, NULL)` - Returns the first position (`'V'`)
- `(position, NULL)` - Returns a position after the given position
- `(NULL, position)` - Returns a position before the given position
- `(pos1, pos2)` - Returns a position between pos1 and pos2

**Example**:
```sql
SELECT lexo.between(NULL, NULL);    -- Returns 'V'
SELECT lexo.between('A', 'Z');      -- Returns 'N'
SELECT lexo.between('V', NULL);     -- Returns 'k'
SELECT lexo.between(NULL, 'V');     -- Returns 'B'
```

### `lexo.next_on_table(table_name TEXT, column_name TEXT)`

Queries the specified table to find the maximum position value in the given column, then returns a position that comes after it. If the table is empty or the column contains only NULL values, it returns the initial position.

**Parameters**:
- `table_name` - The name of the table (can be schema-qualified, e.g., 'public.my_table')
- `column_name` - The name of the column containing position values

**Returns**: `TEXT` - A position string after the maximum existing position, or `'V'` if no positions exist

**Example**:
```sql
-- Simple usage with table and column name
SELECT lexo.next_on_table('items', 'position');

-- Insert with automatic position assignment
INSERT INTO items (id, position) 
VALUES (1, lexo.next_on_table('items', 'position'));

-- With schema-qualified table name
SELECT lexo.next_on_table('public.items', 'position');
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