-- Migration script from pg_lexo 0.1.0 to 0.2.0
-- This script:
-- 1. Removes the lexo schema and moves everything to public schema
-- 2. Renames functions from lexo.function_name() to lexo_function_name()
-- 3. Renames lexo.last() to lexo_next() with new filter_value parameter
-- 4. Moves the lexo type from lexo.lexo to public.lexo

-- Drop the old schema-based functions
DROP FUNCTION IF EXISTS lexo.between(lexo.lexo, lexo.lexo);
DROP FUNCTION IF EXISTS lexo.first();
DROP FUNCTION IF EXISTS lexo.after(lexo.lexo);
DROP FUNCTION IF EXISTS lexo.before(lexo.lexo);
DROP FUNCTION IF EXISTS lexo.last(text, text);

-- Drop old operators and operator classes
DROP OPERATOR CLASS IF EXISTS lexo.lexo_hash_ops USING hash;
DROP OPERATOR CLASS IF EXISTS lexo.lexo_btree_ops USING btree;
DROP OPERATOR IF EXISTS lexo.>= (lexo.lexo, lexo.lexo);
DROP OPERATOR IF EXISTS lexo.> (lexo.lexo, lexo.lexo);
DROP OPERATOR IF EXISTS lexo.<= (lexo.lexo, lexo.lexo);
DROP OPERATOR IF EXISTS lexo.< (lexo.lexo, lexo.lexo);
DROP OPERATOR IF EXISTS lexo.<> (lexo.lexo, lexo.lexo);
DROP OPERATOR IF EXISTS lexo.= (lexo.lexo, lexo.lexo);

-- Drop old comparison functions
DROP FUNCTION IF EXISTS lexo.lexo_hash(lexo.lexo);
DROP FUNCTION IF EXISTS lexo.lexo_cmp(lexo.lexo, lexo.lexo);
DROP FUNCTION IF EXISTS lexo.lexo_ge(lexo.lexo, lexo.lexo);
DROP FUNCTION IF EXISTS lexo.lexo_gt(lexo.lexo, lexo.lexo);
DROP FUNCTION IF EXISTS lexo.lexo_le(lexo.lexo, lexo.lexo);
DROP FUNCTION IF EXISTS lexo.lexo_lt(lexo.lexo, lexo.lexo);
DROP FUNCTION IF EXISTS lexo.lexo_ne(lexo.lexo, lexo.lexo);
DROP FUNCTION IF EXISTS lexo.lexo_eq(lexo.lexo, lexo.lexo);

-- Drop old type and I/O functions
DROP TYPE IF EXISTS lexo.lexo CASCADE;
DROP FUNCTION IF EXISTS lexo.lexo_out(lexo.lexo);
DROP FUNCTION IF EXISTS lexo.lexo_in(cstring);

-- Drop the lexo schema
DROP SCHEMA IF EXISTS lexo CASCADE;

-- Create the new lexo type in public schema
CREATE TYPE lexo;

-- Create I/O functions for the lexo type
CREATE FUNCTION lexo_in(cstring) RETURNS lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_in_wrapper';

CREATE FUNCTION lexo_out(lexo) RETURNS cstring
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_out_wrapper';

-- Complete the type definition
CREATE TYPE lexo (
    INPUT = lexo_in,
    OUTPUT = lexo_out,
    LIKE = text
);

-- Create comparison functions
CREATE FUNCTION lexo_eq(lexo, lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_eq_wrapper';

CREATE FUNCTION lexo_ne(lexo, lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_ne_wrapper';

CREATE FUNCTION lexo_lt(lexo, lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_lt_wrapper';

CREATE FUNCTION lexo_le(lexo, lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_le_wrapper';

CREATE FUNCTION lexo_gt(lexo, lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_gt_wrapper';

CREATE FUNCTION lexo_ge(lexo, lexo) RETURNS bool
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_ge_wrapper';

CREATE FUNCTION lexo_cmp(lexo, lexo) RETURNS integer
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_cmp_wrapper';

CREATE FUNCTION lexo_hash(lexo) RETURNS integer
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_hash_wrapper';

-- Create operators
CREATE OPERATOR = (
    LEFTARG = lexo,
    RIGHTARG = lexo,
    FUNCTION = lexo_eq,
    COMMUTATOR = =,
    NEGATOR = <>,
    RESTRICT = eqsel,
    JOIN = eqjoinsel,
    HASHES,
    MERGES
);

CREATE OPERATOR <> (
    LEFTARG = lexo,
    RIGHTARG = lexo,
    FUNCTION = lexo_ne,
    COMMUTATOR = <>,
    NEGATOR = =,
    RESTRICT = neqsel,
    JOIN = neqjoinsel
);

CREATE OPERATOR < (
    LEFTARG = lexo,
    RIGHTARG = lexo,
    FUNCTION = lexo_lt,
    COMMUTATOR = >,
    NEGATOR = >=,
    RESTRICT = scalarltsel,
    JOIN = scalarltjoinsel
);

CREATE OPERATOR <= (
    LEFTARG = lexo,
    RIGHTARG = lexo,
    FUNCTION = lexo_le,
    COMMUTATOR = >=,
    NEGATOR = >,
    RESTRICT = scalarlesel,
    JOIN = scalarlejoinsel
);

CREATE OPERATOR > (
    LEFTARG = lexo,
    RIGHTARG = lexo,
    FUNCTION = lexo_gt,
    COMMUTATOR = <,
    NEGATOR = <=,
    RESTRICT = scalargtsel,
    JOIN = scalargtjoinsel
);

CREATE OPERATOR >= (
    LEFTARG = lexo,
    RIGHTARG = lexo,
    FUNCTION = lexo_ge,
    COMMUTATOR = <=,
    NEGATOR = <,
    RESTRICT = scalargesel,
    JOIN = scalargejoinsel
);

-- Create operator classes for indexing
CREATE OPERATOR CLASS lexo_btree_ops
    DEFAULT FOR TYPE lexo USING btree AS
        OPERATOR 1 <,
        OPERATOR 2 <=,
        OPERATOR 3 =,
        OPERATOR 4 >=,
        OPERATOR 5 >,
        FUNCTION 1 lexo_cmp(lexo, lexo);

CREATE OPERATOR CLASS lexo_hash_ops
    DEFAULT FOR TYPE lexo USING hash AS
        OPERATOR 1 =,
        FUNCTION 1 lexo_hash(lexo);

-- Create the new public functions with lexo_ prefix
CREATE FUNCTION lexo_between("before" lexo DEFAULT NULL, "after" lexo DEFAULT NULL) RETURNS lexo
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_between_wrapper';

CREATE FUNCTION lexo_first() RETURNS lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_first_wrapper';

CREATE FUNCTION lexo_after("current" lexo) RETURNS lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_after_wrapper';

CREATE FUNCTION lexo_before("current" lexo) RETURNS lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_before_wrapper';

-- New lexo_next function (replaces lexo.last) with optional filter parameter
-- The filter_value parameter allows filtering by the first column of the primary key
-- This is useful for relationship tables like collection_songs
CREATE FUNCTION lexo_next("table_name" text, "column_name" text, "filter_value" text DEFAULT NULL) RETURNS lexo
    STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_next_wrapper';
