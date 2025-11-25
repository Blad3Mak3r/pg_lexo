# pg_order

A PostgreSQL extension written in Rust using [pgrx](https://github.com/pgcentralfoundation/pgrx) for generating lexicographic ordering values for fields in relationship tables.

## Use Case

This extension is ideal for scenarios where you need to maintain an ordered list in a relational database, such as:

- **Playlist song ordering**: A table with `playlist_id`, `song_id`, and `lexical_position`
- **Task lists**: Ordering tasks within projects
- **Drag-and-drop reordering**: Any UI where items can be reordered without updating all positions

## Features

- **Lexicographic ordering**: Generate string positions that can be sorted alphabetically
- **Efficient insertions**: Insert items between existing positions without reordering other items
- **No gaps**: Positions are always valid and sortable

## Functions

### `lexical_position_first()`

Returns the initial position for a new ordered list.

```sql
SELECT lexical_position_first();  -- Returns 'n'
```

### `lexical_position_after(current TEXT)`

Generates a position after the given position.

```sql
SELECT lexical_position_after('n');  -- Returns a position after 'n'
```

### `lexical_position_before(current TEXT)`

Generates a position before the given position.

```sql
SELECT lexical_position_before('n');  -- Returns a position before 'n'
```

### `lexical_position_between(before TEXT, after TEXT)`

Generates a position between two existing positions. Either parameter can be NULL.

```sql
-- First position (both NULL)
SELECT lexical_position_between(NULL, NULL);  -- Returns 'n'

-- Position after existing
SELECT lexical_position_between('n', NULL);  -- Returns position after 'n'

-- Position before existing
SELECT lexical_position_between(NULL, 'n');  -- Returns position before 'n'

-- Position between two existing
SELECT lexical_position_between('a', 'c');  -- Returns 'b'
```

## Example Usage

### Creating a playlist songs table

```sql
CREATE EXTENSION pg_order;

CREATE TABLE playlist_songs (
    playlist_id INTEGER NOT NULL,
    song_id INTEGER NOT NULL,
    position TEXT NOT NULL,
    PRIMARY KEY (playlist_id, song_id)
);

CREATE INDEX ON playlist_songs (playlist_id, position);

-- Add first song
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES (1, 101, lexical_position_first());

-- Add song at the end
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES (1, 102, (
    SELECT lexical_position_after(MAX(position))
    FROM playlist_songs
    WHERE playlist_id = 1
));

-- Insert song between two existing songs
INSERT INTO playlist_songs (playlist_id, song_id, position)
VALUES (1, 103, (
    SELECT lexical_position_between(
        (SELECT position FROM playlist_songs WHERE playlist_id = 1 AND song_id = 101),
        (SELECT position FROM playlist_songs WHERE playlist_id = 1 AND song_id = 102)
    )
));

-- Query songs in order
SELECT song_id, position
FROM playlist_songs
WHERE playlist_id = 1
ORDER BY position;
```

## Building

### Prerequisites

- Rust (latest stable)
- PostgreSQL 13-17 development headers
- [pgrx](https://github.com/pgcentralfoundation/pgrx)

### Install pgrx

```bash
cargo install cargo-pgrx
cargo pgrx init
```

### Build and Install

```bash
# Build for your PostgreSQL version
cargo pgrx package

# Or run directly for development
cargo pgrx run
```

### Run Tests

```bash
cargo pgrx test
```

## License

MIT