-- Migration script from pg_lexo 0.0.7 to 0.1.0
-- This script adds the new lexo custom type and updates existing functions

-- Create the new lexo type as a shell type first
CREATE TYPE lexo;

-- Create shell type I/O functions
CREATE FUNCTION lexo_in(cstring) RETURNS lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_in_wrapper';

CREATE FUNCTION lexo_out(lexo) RETURNS cstring
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_out_wrapper';

-- Complete the type definition with I/O functions
CREATE TYPE lexo (
    INPUT = lexo_in,
    OUTPUT = lexo_out,
    LIKE = text
);

-- Create comparison functions for the lexo type
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

-- Create operators for the lexo type
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

-- Create operator class for btree index
CREATE OPERATOR CLASS lexo_btree_ops
    DEFAULT FOR TYPE lexo USING btree AS
        OPERATOR 1 <,
        OPERATOR 2 <=,
        OPERATOR 3 =,
        OPERATOR 4 >=,
        OPERATOR 5 >,
        FUNCTION 1 lexo_cmp(lexo, lexo);

-- Create operator class for hash index
CREATE OPERATOR CLASS lexo_hash_ops
    DEFAULT FOR TYPE lexo USING hash AS
        OPERATOR 1 =,
        FUNCTION 1 lexo_hash(lexo);

-- Drop old functions that returned text
DROP FUNCTION IF EXISTS lexo.between(text, text);
DROP FUNCTION IF EXISTS lexo.first();
DROP FUNCTION IF EXISTS lexo.after(text);
DROP FUNCTION IF EXISTS lexo.before(text);

-- Create new functions that return lexo type
CREATE FUNCTION lexo."between"("before" text DEFAULT NULL, "after" text DEFAULT NULL) RETURNS lexo
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_between_wrapper';

CREATE FUNCTION lexo."first"() RETURNS lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_first_wrapper';

CREATE FUNCTION lexo."after"("current" text) RETURNS lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_after_wrapper';

CREATE FUNCTION lexo."before"("current" text) RETURNS lexo
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_before_wrapper';

-- Add the new last function
CREATE FUNCTION lexo."last"("table_name" text, "column_name" text) RETURNS lexo
    STRICT PARALLEL SAFE
    LANGUAGE c AS 'MODULE_PATHNAME', 'lexo_last_wrapper';
