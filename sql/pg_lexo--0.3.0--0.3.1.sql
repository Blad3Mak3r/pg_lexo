-- Migration script from pg_lexo 0.3.0 to 0.3.1
-- This script:
-- 1. Moves the lexo type from pg_catalog schema to lexo schema
-- 2. Fixes the negator operator conflict that occurred in pg_catalog
-- 3. Moves the I/O and comparison functions to lexo schema
-- 4. Keeps the public API functions (lexo_first, lexo_after, lexo_before, lexo_between, lexo_next) in the extension schema

-- Drop existing public API functions first (they depend on the type)
DROP FUNCTION IF EXISTS lexo_next(text, text, text, text);
DROP FUNCTION IF EXISTS lexo_before(pg_catalog.lexo);
DROP FUNCTION IF EXISTS lexo_after(pg_catalog.lexo);
DROP FUNCTION IF EXISTS lexo_first();
DROP FUNCTION IF EXISTS lexo_between(pg_catalog.lexo, pg_catalog.lexo);

-- Drop existing operator classes (they depend on operators)
DROP OPERATOR CLASS IF EXISTS pg_catalog.lexo_hash_ops USING hash CASCADE;
DROP OPERATOR CLASS IF EXISTS pg_catalog.lexo_btree_ops USING btree CASCADE;

-- Drop existing operators (they depend on functions)
DROP OPERATOR IF EXISTS pg_catalog.>= (pg_catalog.lexo, pg_catalog.lexo) CASCADE;
DROP OPERATOR IF EXISTS pg_catalog.> (pg_catalog.lexo, pg_catalog.lexo) CASCADE;
DROP OPERATOR IF EXISTS pg_catalog.<= (pg_catalog.lexo, pg_catalog.lexo) CASCADE;
DROP OPERATOR IF EXISTS pg_catalog.< (pg_catalog.lexo, pg_catalog.lexo) CASCADE;
DROP OPERATOR IF EXISTS pg_catalog.<> (pg_catalog.lexo, pg_catalog.lexo) CASCADE;
DROP OPERATOR IF EXISTS pg_catalog.= (pg_catalog.lexo, pg_catalog.lexo) CASCADE;

-- Drop comparison functions from pg_catalog schema
DROP FUNCTION IF EXISTS pg_catalog.lexo_hash(pg_catalog.lexo);
DROP FUNCTION IF EXISTS pg_catalog.lexo_cmp(pg_catalog.lexo, pg_catalog.lexo);
DROP FUNCTION IF EXISTS pg_catalog.lexo_ge(pg_catalog.lexo, pg_catalog.lexo);
DROP FUNCTION IF EXISTS pg_catalog.lexo_gt(pg_catalog.lexo, pg_catalog.lexo);
DROP FUNCTION IF EXISTS pg_catalog.lexo_le(pg_catalog.lexo, pg_catalog.lexo);
DROP FUNCTION IF EXISTS pg_catalog.lexo_lt(pg_catalog.lexo, pg_catalog.lexo);
DROP FUNCTION IF EXISTS pg_catalog.lexo_ne(pg_catalog.lexo, pg_catalog.lexo);
DROP FUNCTION IF EXISTS pg_catalog.lexo_eq(pg_catalog.lexo, pg_catalog.lexo);

-- Drop type and I/O functions from pg_catalog schema
DROP TYPE IF EXISTS pg_catalog.lexo CASCADE;
DROP FUNCTION IF EXISTS pg_catalog.lexo_out(pg_catalog.lexo);
DROP FUNCTION IF EXISTS pg_catalog.lexo_in(cstring);

-- Create the lexo schema
CREATE SCHEMA IF NOT EXISTS lexo;

-- Create the new lexo type in lexo schema
CREATE TYPE lexo.lexo;

-- Create I/O functions for the lexo type in lexo schema
CREATE FUNCTION lexo.lexo_in(cstring) RETURNS lexo.lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_in_wrapper';

CREATE FUNCTION lexo.lexo_out(lexo.lexo) RETURNS cstring
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_out_wrapper';

-- Complete the type definition in lexo schema
CREATE TYPE lexo.lexo (
    INPUT = lexo.lexo_in,
    OUTPUT = lexo.lexo_out,
    LIKE = text
);

-- Create comparison functions in lexo schema
CREATE FUNCTION lexo.lexo_eq(lexo.lexo, lexo.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_eq_wrapper';

CREATE FUNCTION lexo.lexo_ne(lexo.lexo, lexo.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_ne_wrapper';

CREATE FUNCTION lexo.lexo_lt(lexo.lexo, lexo.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_lt_wrapper';

CREATE FUNCTION lexo.lexo_le(lexo.lexo, lexo.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_le_wrapper';

CREATE FUNCTION lexo.lexo_gt(lexo.lexo, lexo.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_gt_wrapper';

CREATE FUNCTION lexo.lexo_ge(lexo.lexo, lexo.lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_ge_wrapper';

CREATE FUNCTION lexo.lexo_cmp(lexo.lexo, lexo.lexo) RETURNS integer
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_cmp_wrapper';

CREATE FUNCTION lexo.lexo_hash(lexo.lexo) RETURNS integer
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_hash_wrapper';

-- Create operators in lexo schema
-- Note: We create = first WITHOUT negator, then <> with negator pointing to =
-- This avoids the "negator operator is already the negator" error
CREATE OPERATOR lexo.= (
    LEFTARG = lexo.lexo,
    RIGHTARG = lexo.lexo,
    FUNCTION = lexo.lexo_eq,
    COMMUTATOR = OPERATOR(lexo.=),
    RESTRICT = eqsel,
    JOIN = eqjoinsel,
    HASHES,
    MERGES
);

CREATE OPERATOR lexo.<> (
    LEFTARG = lexo.lexo,
    RIGHTARG = lexo.lexo,
    FUNCTION = lexo.lexo_ne,
    COMMUTATOR = OPERATOR(lexo.<>),
    NEGATOR = OPERATOR(lexo.=),
    RESTRICT = neqsel,
    JOIN = neqjoinsel
);

-- Now update the = operator to add its negator
-- This is done after <> is created to avoid the circular reference issue
-- PostgreSQL will automatically set the negator for = when we set it for <>

CREATE OPERATOR lexo.< (
    LEFTARG = lexo.lexo,
    RIGHTARG = lexo.lexo,
    FUNCTION = lexo.lexo_lt,
    COMMUTATOR = OPERATOR(lexo.>),
    NEGATOR = OPERATOR(lexo.>=),
    RESTRICT = scalarltsel,
    JOIN = scalarltjoinsel
);

CREATE OPERATOR lexo.<= (
    LEFTARG = lexo.lexo,
    RIGHTARG = lexo.lexo,
    FUNCTION = lexo.lexo_le,
    COMMUTATOR = OPERATOR(lexo.>=),
    NEGATOR = OPERATOR(lexo.>),
    RESTRICT = scalarlesel,
    JOIN = scalarlejoinsel
);

CREATE OPERATOR lexo.> (
    LEFTARG = lexo.lexo,
    RIGHTARG = lexo.lexo,
    FUNCTION = lexo.lexo_gt,
    COMMUTATOR = OPERATOR(lexo.<),
    NEGATOR = OPERATOR(lexo.<=),
    RESTRICT = scalargtsel,
    JOIN = scalargtjoinsel
);

CREATE OPERATOR lexo.>= (
    LEFTARG = lexo.lexo,
    RIGHTARG = lexo.lexo,
    FUNCTION = lexo.lexo_ge,
    COMMUTATOR = OPERATOR(lexo.<=),
    NEGATOR = OPERATOR(lexo.<),
    RESTRICT = scalargesel,
    JOIN = scalargejoinsel
);

-- Create operator classes for indexing in lexo schema
CREATE OPERATOR CLASS lexo.lexo_btree_ops
    DEFAULT FOR TYPE lexo.lexo USING btree AS
        OPERATOR 1 lexo.<,
        OPERATOR 2 lexo.<=,
        OPERATOR 3 lexo.=,
        OPERATOR 4 lexo.>=,
        OPERATOR 5 lexo.>,
        FUNCTION 1 lexo.lexo_cmp(lexo.lexo, lexo.lexo);

CREATE OPERATOR CLASS lexo.lexo_hash_ops
    DEFAULT FOR TYPE lexo.lexo USING hash AS
        OPERATOR 1 lexo.=,
        FUNCTION 1 lexo.lexo_hash(lexo.lexo);

-- Create the public API functions (these remain in the extension schema)
CREATE FUNCTION lexo_between("before" lexo.lexo DEFAULT NULL, "after" lexo.lexo DEFAULT NULL) RETURNS lexo.lexo
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_between_wrapper';

CREATE FUNCTION lexo_first() RETURNS lexo.lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_first_wrapper';

CREATE FUNCTION lexo_after("current" lexo.lexo) RETURNS lexo.lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_after_wrapper';

CREATE FUNCTION lexo_before("current" lexo.lexo) RETURNS lexo.lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_before_wrapper';

-- lexo_next function with explicit filter parameters
-- Allows filtering by a specific column and value for relationship tables
CREATE FUNCTION lexo_next(
    "table_name" text, 
    "lexo_column_name" text, 
    "identifier_column_name" text DEFAULT NULL,
    "identifier_value" text DEFAULT NULL
) RETURNS lexo.lexo
    STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_next_wrapper';
