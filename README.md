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
  - [Available Functions](#available-functions)
  - [Adding a Lexo Column](#adding-a-lexo-column)
  - [Basic Examples](#basic-examples)
  - [Real-World Example: Playlist Ordering](#real-world-example-playlist-ordering)
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

- **Dedicated `lexo` Schema**: All functions are organized under the `lexo` schema (`lexo.first()`, `lexo.after()`, etc.)
- **Simple TEXT Columns**: Uses `TEXT COLLATE "C"` columns for proper byte-order sorting
- **Helper Function**: `lexo.add_lexo_column_to()` automatically adds properly configured columns
- **Base62 Encoding**: Uses 62 characters (0-9, A-Z, a-z) for compact, efficient position strings
- **Efficient Insertions**: Insert items between any two positions without updating other rows
- **Unlimited Insertions**: Can always generate a position between any two existing positions
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

pg_lexo provides all functions under the `lexo` schema. Position values are stored as `TEXT COLLATE "C"` for proper byte-order sorting.

```sql
-- Create the extension
CREATE EXTENSION pg_lexo;

-- Create a table with a lexo position column
CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    position TEXT COLLATE "C" NOT NULL
);

-- Or use the helper function to add the column
SELECT lexo.add_lexo_column_to('items', 'position');

-- Insert items with lexo positions
INSERT INTO items (name, position) VALUES ('First', lexo.first());
INSERT INTO items (name, position) VALUES ('Second', lexo.next('items', 'position', NULL, NULL));

-- Query in order
SELECT * FROM items ORDER BY position;
```

### Available Functions

All functions are in the `lexo` schema:

| Function | Description |
|----------|-------------|
| `lexo.first()` | Returns the initial position (`'V'`) |
| `lexo.after(position)` | Returns a position after the given position |
| `lexo.before(position)` | Returns a position before the given position |
| `lexo.between(before, after)` | Returns a position between two positions (either can be NULL) |
| `lexo.next(table, column, filter_col, filter_val)` | Returns the next position after the maximum in a table |
| `lexo.add_lexo_column_to(table, column)` | Adds a `TEXT COLLATE "C"` column to a table |

### Adding a Lexo Column

The easiest way to add a properly configured position column is with `lexo.add_lexo_column_to()`:

```sql
-- Create your table
CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

-- Add a position column with proper collation
SELECT lexo.add_lexo_column_to('items', 'position');

-- This is equivalent to:
-- ALTER TABLE items ADD COLUMN position TEXT COLLATE "C";
```

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
SELECT lexo.next('playlist_songs', 'position', NULL, NULL);
-- Returns: the next position after the current maximum, or 'V' if table is empty

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
    position TEXT COLLATE "C" NOT NULL,
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

-- Query songs in order
SELECT song_id, position
FROM playlist_songs
WHERE playlist_id = 'playlist-1'
ORDER BY position;

-- Result:
-- song_id   | position
-- ----------|----------
-- song-101  | V
-- song-104  | c        (inserted between 101 and 102)
-- song-102  | k
-- song-103  | u

-- Move song-103 to the beginning
UPDATE playlist_songs
SET position = (
    SELECT lexo.before(MIN(position))
    FROM playlist_songs
    WHERE playlist_id = 'playlist-1'
)
WHERE playlist_id = 'playlist-1' AND song_id = 'song-103';

-- Query songs in new order
SELECT song_id, position
FROM playlist_songs
WHERE playlist_id = 'playlist-1'
ORDER BY position;

-- Result:
-- song_id   | position
-- ----------|----------
-- song-103  | B        (now at the beginning)
-- song-101  | V
-- song-104  | c
-- song-102  | k
```

## How It Works

### Base62 Encoding

The extension uses a Base62 character set for position strings:

```
0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz
```

This provides 62 possible characters per position, allowing for efficient string representation while maintaining proper lexicographic ordering with `COLLATE "C"`.

### Position Generation Algorithm

1. **First Position**: Returns `'V'` (the midpoint of Base62, index 31)
2. **After Position**: Finds the midpoint between the current position and the maximum (`'z'`)
3. **Before Position**: Finds the midpoint between the minimum (`'0'`) and the current position
4. **Between Positions**: Finds the midpoint between two given positions

When the midpoint would result in the same character, the algorithm extends the string by appending additional characters.

### Why TEXT COLLATE "C"?

The `C` collation (or `POSIX`) uses byte-value ordering, which ensures that:
- `'0'` < `'9'` < `'A'` < `'Z'` < `'a'` < `'z'`

This is exactly what pg_lexo expects for correct ordering. Using `lexo.add_lexo_column_to()` ensures your columns are created with the proper collation.

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

**Returns**: `TEXT` - The position `'V'`

**Example**:
```sql
SELECT lexo.first();  -- Returns 'V'
```

### `lexo.after(current TEXT)`

Generates a position that comes after the given position.

**Parameters**:
- `current` - The current position (must be valid Base62)

**Returns**: `TEXT` - A position greater than `current`

**Example**:
```sql
SELECT lexo.after('V');  -- Returns 'k'
```

### `lexo.before(current TEXT)`

Generates a position that comes before the given position.

**Parameters**:
- `current` - The current position (must be valid Base62)

**Returns**: `TEXT` - A position less than `current`

**Example**:
```sql
SELECT lexo.before('V');  -- Returns 'B'
```

### `lexo.between(before TEXT, after TEXT)`

Generates a position between two existing positions. Either parameter can be NULL.

**Parameters**:
- `before` - The position before the new position (NULL for beginning)
- `after` - The position after the new position (NULL for end)

**Returns**: `TEXT` - A position between `before` and `after`

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

### `lexo.next(table_name, column_name, filter_column, filter_value)`

Returns the next position after the maximum in a table column.

**Parameters**:
- `table_name` - The name of the table (can be schema-qualified)
- `column_name` - The name of the position column
- `filter_column` - Optional: column to filter by (e.g., 'collection_id')
- `filter_value` - Optional: value to filter by

**Returns**: `TEXT` - A position after the maximum, or `'V'` if table is empty

**Example**:
```sql
-- Get next position for entire table
SELECT lexo.next('items', 'position', NULL, NULL);

-- Get next position for a specific collection
SELECT lexo.next('collection_songs', 'position', 'collection_id', 'abc-123');
```

### `lexo.add_lexo_column_to(table_name, column_name)`

Adds a `TEXT COLLATE "C"` column to an existing table.

**Parameters**:
- `table_name` - The name of the table (can be schema-qualified)
- `column_name` - The name of the new column

**Example**:
```sql
SELECT lexo.add_lexo_column_to('items', 'position');
-- Equivalent to: ALTER TABLE items ADD COLUMN position TEXT COLLATE "C";
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
