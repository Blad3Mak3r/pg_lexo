-- Migration script from pg_lexo 0.2.0 to 0.3.0
-- This script:
-- 1. Moves the lexo type from public schema to pg_catalog schema
-- 2. Moves the I/O and comparison functions to pg_catalog schema
-- 3. Keeps the public API functions (lexo_first, lexo_after, lexo_before, lexo_between, lexo_next) in the extension schema

-- Drop existing operator classes first (they depend on operators)
DROP OPERATOR CLASS IF EXISTS lexo_hash_ops USING hash;
DROP OPERATOR CLASS IF EXISTS lexo_btree_ops USING btree;

-- Drop existing operators (they depend on functions)
DROP OPERATOR IF EXISTS >= (lexo, lexo);
DROP OPERATOR IF EXISTS > (lexo, lexo);
DROP OPERATOR IF EXISTS <= (lexo, lexo);
DROP OPERATOR IF EXISTS < (lexo, lexo);
DROP OPERATOR IF EXISTS <> (lexo, lexo);
DROP OPERATOR IF EXISTS = (lexo, lexo);

-- Drop old comparison functions from public schema
DROP FUNCTION IF EXISTS lexo_hash(lexo);
DROP FUNCTION IF EXISTS lexo_cmp(lexo, lexo);
DROP FUNCTION IF EXISTS lexo_ge(lexo, lexo);
DROP FUNCTION IF EXISTS lexo_gt(lexo, lexo);
DROP FUNCTION IF EXISTS lexo_le(lexo, lexo);
DROP FUNCTION IF EXISTS lexo_lt(lexo, lexo);
DROP FUNCTION IF EXISTS lexo_ne(lexo, lexo);
DROP FUNCTION IF EXISTS lexo_eq(lexo, lexo);

-- Drop old public API functions
DROP FUNCTION IF EXISTS lexo_next(text, text, text, text);
DROP FUNCTION IF EXISTS lexo_before(lexo);
DROP FUNCTION IF EXISTS lexo_after(lexo);
DROP FUNCTION IF EXISTS lexo_first();
DROP FUNCTION IF EXISTS lexo_between(lexo, lexo);

-- Drop old type and I/O functions from public schema
DROP TYPE IF EXISTS lexo CASCADE;
DROP FUNCTION IF EXISTS lexo_out(lexo);
DROP FUNCTION IF EXISTS lexo_in(cstring);

-- Create the new lexo type in pg_catalog schema (globally accessible)
CREATE TYPE pg_catalog.lexo;

-- Create I/O functions for the lexo type in pg_catalog
CREATE FUNCTION pg_catalog.lexo_in(cstring) RETURNS pg_catalog.lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_in_wrapper';

CREATE FUNCTION pg_catalog.lexo_out(pg_catalog.lexo) RETURNS cstring
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_out_wrapper';

-- Complete the type definition in pg_catalog
CREATE TYPE pg_catalog.lexo (
    INPUT = pg_catalog.lexo_in,
    OUTPUT = pg_catalog.lexo_out,
    LIKE = text
);

-- Create comparison functions in pg_catalog (internal, not public API)
CREATE FUNCTION pg_catalog.lexo_eq(pg_catalog.lexo, pg_catalog.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_eq_wrapper';

CREATE FUNCTION pg_catalog.lexo_ne(pg_catalog.lexo, pg_catalog.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_ne_wrapper';

CREATE FUNCTION pg_catalog.lexo_lt(pg_catalog.lexo, pg_catalog.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_lt_wrapper';

CREATE FUNCTION pg_catalog.lexo_le(pg_catalog.lexo, pg_catalog.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_le_wrapper';

CREATE FUNCTION pg_catalog.lexo_gt(pg_catalog.lexo, pg_catalog.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_gt_wrapper';

CREATE FUNCTION pg_catalog.lexo_ge(pg_catalog.lexo, pg_catalog.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_ge_wrapper';

CREATE FUNCTION pg_catalog.lexo_cmp(pg_catalog.lexo, pg_catalog.lexo) RETURNS integer
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_cmp_wrapper';

CREATE FUNCTION pg_catalog.lexo_hash(pg_catalog.lexo) RETURNS integer
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_hash_wrapper';

-- Create operators in pg_catalog (they reference pg_catalog.lexo type)
CREATE OPERATOR pg_catalog.= (
    LEFTARG = pg_catalog.lexo,
    RIGHTARG = pg_catalog.lexo,
    FUNCTION = pg_catalog.lexo_eq,
    COMMUTATOR = OPERATOR(pg_catalog.=),
    NEGATOR = OPERATOR(pg_catalog.<>),
    RESTRICT = eqsel,
    JOIN = eqjoinsel,
    HASHES,
    MERGES
);

CREATE OPERATOR pg_catalog.<> (
    LEFTARG = pg_catalog.lexo,
    RIGHTARG = pg_catalog.lexo,
    FUNCTION = pg_catalog.lexo_ne,
    COMMUTATOR = OPERATOR(pg_catalog.<>),
    NEGATOR = OPERATOR(pg_catalog.=),
    RESTRICT = neqsel,
    JOIN = neqjoinsel
);

CREATE OPERATOR pg_catalog.< (
    LEFTARG = pg_catalog.lexo,
    RIGHTARG = pg_catalog.lexo,
    FUNCTION = pg_catalog.lexo_lt,
    COMMUTATOR = OPERATOR(pg_catalog.>),
    NEGATOR = OPERATOR(pg_catalog.>=),
    RESTRICT = scalarltsel,
    JOIN = scalarltjoinsel
);

CREATE OPERATOR pg_catalog.<= (
    LEFTARG = pg_catalog.lexo,
    RIGHTARG = pg_catalog.lexo,
    FUNCTION = pg_catalog.lexo_le,
    COMMUTATOR = OPERATOR(pg_catalog.>=),
    NEGATOR = OPERATOR(pg_catalog.>),
    RESTRICT = scalarlesel,
    JOIN = scalarlejoinsel
);

CREATE OPERATOR pg_catalog.> (
    LEFTARG = pg_catalog.lexo,
    RIGHTARG = pg_catalog.lexo,
    FUNCTION = pg_catalog.lexo_gt,
    COMMUTATOR = OPERATOR(pg_catalog.<),
    NEGATOR = OPERATOR(pg_catalog.<=),
    RESTRICT = scalargtsel,
    JOIN = scalargtjoinsel
);

CREATE OPERATOR pg_catalog.>= (
    LEFTARG = pg_catalog.lexo,
    RIGHTARG = pg_catalog.lexo,
    FUNCTION = pg_catalog.lexo_ge,
    COMMUTATOR = OPERATOR(pg_catalog.<=),
    NEGATOR = OPERATOR(pg_catalog.<),
    RESTRICT = scalargesel,
    JOIN = scalargejoinsel
);

-- Create operator classes for indexing in pg_catalog
CREATE OPERATOR CLASS pg_catalog.lexo_btree_ops
    DEFAULT FOR TYPE pg_catalog.lexo USING btree AS
        OPERATOR 1 pg_catalog.<,
        OPERATOR 2 pg_catalog.<=,
        OPERATOR 3 pg_catalog.=,
        OPERATOR 4 pg_catalog.>=,
        OPERATOR 5 pg_catalog.>,
        FUNCTION 1 pg_catalog.lexo_cmp(pg_catalog.lexo, pg_catalog.lexo);

CREATE OPERATOR CLASS pg_catalog.lexo_hash_ops
    DEFAULT FOR TYPE pg_catalog.lexo USING hash AS
        OPERATOR 1 pg_catalog.=,
        FUNCTION 1 pg_catalog.lexo_hash(pg_catalog.lexo);

-- Create the public API functions (these remain in the extension schema)
CREATE FUNCTION lexo_between("before" pg_catalog.lexo DEFAULT NULL, "after" pg_catalog.lexo DEFAULT NULL) RETURNS pg_catalog.lexo
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_between_wrapper';

CREATE FUNCTION lexo_first() RETURNS pg_catalog.lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_first_wrapper';

CREATE FUNCTION lexo_after("current" pg_catalog.lexo) RETURNS pg_catalog.lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_after_wrapper';

CREATE FUNCTION lexo_before("current" pg_catalog.lexo) RETURNS pg_catalog.lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_before_wrapper';

-- lexo_next function with explicit filter parameters
-- Allows filtering by a specific column and value for relationship tables
CREATE FUNCTION lexo_next(
    "table_name" text, 
    "lexo_column_name" text, 
    "identifier_column_name" text DEFAULT NULL,
    "identifier_value" text DEFAULT NULL
) RETURNS pg_catalog.lexo
    STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_next_wrapper';
