//! Shared Postgres statements for `formula_measure_kind` and JSON rewrites.
//!
//! Machinery migrations run before finance products, so each caller installs the
//! type and functions again. `CREATE` is idempotent and `CREATE OR REPLACE`
//! keeps one definition.

pub const ENSURE_FORMULA_MEASURE: &str = r#"
DO $$ BEGIN
    CREATE TYPE formula_measure_kind AS ENUM (
        'length', 'weight', 'duration', 'quantity', 'decimal', 'percent'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE formula_measure AS (
        kind formula_measure_kind,
        value numeric,
        unit text
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

CREATE OR REPLACE FUNCTION lariv_formula_accepted_unit(kind text)
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT CASE kind
        WHEN 'length' THEN 'mm'
        WHEN 'weight' THEN 'kg'
        WHEN 'duration' THEN 's'
        WHEN 'quantity' THEN 'Number'
        WHEN 'decimal' THEN 'Seconds'
        WHEN 'percent' THEN '%'
        ELSE NULL
    END
$$;

CREATE OR REPLACE FUNCTION lariv_migrate_formula_schema(schema jsonb)
RETURNS jsonb
LANGUAGE plpgsql
AS $$
DECLARE
    result jsonb := '{}'::jsonb;
    entry record;
    kind text;
    accepted text;
BEGIN
    IF schema IS NULL OR jsonb_typeof(schema) <> 'object' THEN
        RETURN '{}'::jsonb;
    END IF;
    FOR entry IN SELECT * FROM jsonb_each(schema) LOOP
        IF jsonb_typeof(entry.value) = 'object'
           AND entry.value ? 'type'
           AND entry.value ? 'accepted_unit' THEN
            kind := entry.value->>'type';
            PERFORM kind::formula_measure_kind;
            accepted := entry.value->>'accepted_unit';
        ELSIF jsonb_typeof(entry.value) = 'object' AND entry.value ? 'type' THEN
            kind := entry.value->>'type';
            PERFORM kind::formula_measure_kind;
            accepted := lariv_formula_accepted_unit(kind);
        ELSIF jsonb_typeof(entry.value) = 'string' THEN
            kind := entry.value #>> '{}';
            PERFORM kind::formula_measure_kind;
            accepted := lariv_formula_accepted_unit(kind);
        ELSE
            RAISE EXCEPTION 'variable % has no type', entry.key;
        END IF;
        IF accepted IS NULL OR btrim(accepted) = '' THEN
            RAISE EXCEPTION 'variable % has no accepted unit', entry.key;
        END IF;
        result := result || jsonb_build_object(
            entry.key,
            jsonb_build_object('type', kind, 'accepted_unit', accepted)
        );
    END LOOP;
    RETURN result;
END;
$$;

CREATE OR REPLACE FUNCTION lariv_migrate_formula_values(vals jsonb, schema jsonb, extra jsonb)
RETURNS jsonb
LANGUAGE plpgsql
AS $$
DECLARE
    result jsonb := '{}'::jsonb;
    entry record;
    kind text;
    unit text;
    magnitude text;
    spec jsonb;
    extras jsonb;
BEGIN
    IF vals IS NULL OR jsonb_typeof(vals) <> 'object' THEN
        RETURN '{}'::jsonb;
    END IF;
    extras := extra;
    IF extras IS NULL OR jsonb_typeof(extras) <> 'object' THEN
        extras := '{}'::jsonb;
    END IF;
    FOR entry IN SELECT * FROM jsonb_each(vals) LOOP
        IF jsonb_typeof(entry.value) = 'object'
           AND entry.value ? 'kind'
           AND entry.value ? 'value'
           AND entry.value ? 'unit' THEN
            PERFORM (entry.value->>'kind')::formula_measure_kind;
            PERFORM (entry.value->>'value')::numeric;
            result := result || jsonb_build_object(entry.key, entry.value);
            CONTINUE;
        END IF;
        spec := schema->entry.key;
        IF spec IS NULL THEN
            RAISE EXCEPTION 'variable % has no schema', entry.key;
        END IF;
        IF jsonb_typeof(spec) = 'string' THEN
            kind := spec #>> '{}';
        ELSIF jsonb_typeof(spec) = 'object' THEN
            kind := spec->>'type';
        ELSE
            kind := NULL;
        END IF;
        IF kind IS NULL THEN
            RAISE EXCEPTION 'variable % has no schema', entry.key;
        END IF;
        PERFORM kind::formula_measure_kind;
        IF jsonb_typeof(entry.value) = 'object' AND entry.value ? 'value' THEN
            magnitude := entry.value->>'value';
            unit := NULLIF(btrim(entry.value->>'unit'), '');
        ELSE
            magnitude := entry.value #>> '{}';
            unit := NULL;
        END IF;
        IF magnitude IS NULL OR btrim(magnitude) = '' THEN
            RAISE EXCEPTION 'variable % has no value', entry.key;
        END IF;
        IF kind = 'duration'
           AND unit IS NULL
           AND jsonb_typeof(entry.value) = 'number'
           AND abs(magnitude::numeric) >= 1000000000 THEN
            magnitude := (magnitude::numeric / 1000000000)::text;
        END IF;
        PERFORM magnitude::numeric;
        IF kind = 'length' THEN
            IF unit IS NULL THEN
                unit := extras->'dim_units'->>entry.key;
            END IF;
            IF unit IS NULL OR btrim(unit) = '' THEN
                unit := NULLIF(btrim(extras->>'dim_unit'), '');
            END IF;
            unit := COALESCE(NULLIF(btrim(unit), ''), 'mm');
        ELSIF kind = 'weight' THEN
            unit := COALESCE(NULLIF(btrim(unit), ''), 'kg');
        ELSE
            unit := lariv_formula_accepted_unit(kind);
        END IF;
        result := result || jsonb_build_object(
            entry.key,
            jsonb_build_object('kind', kind, 'value', magnitude, 'unit', unit)
        );
    END LOOP;
    RETURN result;
END;
$$;
"#;

pub const MIGRATE_PRODUCT_SCHEMAS: &str = r#"
UPDATE products
SET variables = lariv_migrate_formula_schema(variables::jsonb)::text
WHERE variables IS NOT NULL AND btrim(variables) NOT IN ('', '{}');
"#;

pub const MIGRATE_INVOICE_VALUES: &str = r#"
UPDATE draft_invoice_lines AS line
SET variable_values = lariv_migrate_formula_values(
    line.variable_values::jsonb, product.variables::jsonb, '{}'::jsonb
)::text
FROM products AS product
WHERE product.id = line.product_id
  AND line.variable_values IS NOT NULL
  AND btrim(line.variable_values) NOT IN ('', '{}');

UPDATE posted_invoice_lines AS line
SET variable_values = lariv_migrate_formula_values(
    line.variable_values::jsonb, product.variables::jsonb, '{}'::jsonb
)::text
FROM products AS product
WHERE product.id = line.product_id
  AND line.variable_values IS NOT NULL
  AND btrim(line.variable_values) NOT IN ('', '{}');

UPDATE cancelled_invoice_lines AS line
SET variable_values = lariv_migrate_formula_values(
    line.variable_values::jsonb, product.variables::jsonb, '{}'::jsonb
)::text
FROM products AS product
WHERE product.id = line.product_id
  AND line.variable_values IS NOT NULL
  AND btrim(line.variable_values) NOT IN ('', '{}');
"#;

pub const MIGRATE_PURCHASE_VALUES: &str = r#"
UPDATE draft_purchase_lines AS line
SET variable_values = lariv_migrate_formula_values(
    line.variable_values::jsonb, product.variables::jsonb, '{}'::jsonb
)::text
FROM products AS product
WHERE product.id = line.product_id
  AND line.variable_values IS NOT NULL
  AND btrim(line.variable_values) NOT IN ('', '{}');

UPDATE posted_purchase_lines AS line
SET variable_values = lariv_migrate_formula_values(
    line.variable_values::jsonb, product.variables::jsonb, '{}'::jsonb
)::text
FROM products AS product
WHERE product.id = line.product_id
  AND line.variable_values IS NOT NULL
  AND btrim(line.variable_values) NOT IN ('', '{}');

UPDATE cancelled_purchase_lines AS line
SET variable_values = lariv_migrate_formula_values(
    line.variable_values::jsonb, product.variables::jsonb, '{}'::jsonb
)::text
FROM products AS product
WHERE product.id = line.product_id
  AND line.variable_values IS NOT NULL
  AND btrim(line.variable_values) NOT IN ('', '{}');
"#;

/// Point quantity at `Number` and decimal at `Seconds` on schemas and measures.
pub const REFRESH_FORMULA_FIXED_UNITS: &str = r#"
CREATE OR REPLACE FUNCTION lariv_formula_accepted_unit(kind text)
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT CASE kind
        WHEN 'length' THEN 'mm'
        WHEN 'weight' THEN 'kg'
        WHEN 'duration' THEN 's'
        WHEN 'quantity' THEN 'Number'
        WHEN 'decimal' THEN 'Seconds'
        WHEN 'percent' THEN '%'
        ELSE NULL
    END
$$;

CREATE OR REPLACE FUNCTION lariv_refresh_formula_schema_units(schema jsonb)
RETURNS jsonb
LANGUAGE plpgsql
AS $$
DECLARE
    result jsonb := '{}'::jsonb;
    entry record;
    kind text;
    accepted text;
BEGIN
    IF schema IS NULL OR jsonb_typeof(schema) <> 'object' THEN
        RETURN '{}'::jsonb;
    END IF;
    FOR entry IN SELECT * FROM jsonb_each(schema) LOOP
        IF jsonb_typeof(entry.value) = 'object' AND entry.value ? 'type' THEN
            kind := entry.value->>'type';
            PERFORM kind::formula_measure_kind;
            IF kind IN ('quantity', 'decimal') THEN
                accepted := lariv_formula_accepted_unit(kind);
            ELSE
                accepted := NULLIF(btrim(entry.value->>'accepted_unit'), '');
                accepted := COALESCE(accepted, lariv_formula_accepted_unit(kind));
            END IF;
        ELSIF jsonb_typeof(entry.value) = 'string' THEN
            kind := entry.value #>> '{}';
            PERFORM kind::formula_measure_kind;
            accepted := lariv_formula_accepted_unit(kind);
        ELSE
            RAISE EXCEPTION 'variable % has no type', entry.key;
        END IF;
        result := result || jsonb_build_object(
            entry.key,
            jsonb_build_object('type', kind, 'accepted_unit', accepted)
        );
    END LOOP;
    RETURN result;
END;
$$;

CREATE OR REPLACE FUNCTION lariv_refresh_formula_value_units(vals jsonb)
RETURNS jsonb
LANGUAGE plpgsql
AS $$
DECLARE
    result jsonb := '{}'::jsonb;
    entry record;
    kind text;
    unit text;
BEGIN
    IF vals IS NULL OR jsonb_typeof(vals) <> 'object' THEN
        RETURN '{}'::jsonb;
    END IF;
    FOR entry IN SELECT * FROM jsonb_each(vals) LOOP
        IF jsonb_typeof(entry.value) <> 'object' OR NOT (entry.value ? 'kind') THEN
            RAISE EXCEPTION 'variable % is not a measure', entry.key;
        END IF;
        kind := entry.value->>'kind';
        PERFORM kind::formula_measure_kind;
        unit := entry.value->>'unit';
        IF kind IN ('quantity', 'decimal') THEN
            unit := lariv_formula_accepted_unit(kind);
        END IF;
        result := result || jsonb_build_object(
            entry.key,
            jsonb_build_object(
                'kind', kind,
                'value', entry.value->>'value',
                'unit', unit
            )
        );
    END LOOP;
    RETURN result;
END;
$$;
"#;

pub const REFRESH_PRODUCT_SCHEMA_UNITS: &str = r#"
UPDATE products
SET variables = lariv_refresh_formula_schema_units(variables::jsonb)::text
WHERE variables IS NOT NULL AND btrim(variables) NOT IN ('', '{}');
"#;

pub const REFRESH_INVOICE_VALUE_UNITS: &str = r#"
UPDATE draft_invoice_lines
SET variable_values = lariv_refresh_formula_value_units(variable_values::jsonb)::text
WHERE variable_values IS NOT NULL AND btrim(variable_values) NOT IN ('', '{}');

UPDATE posted_invoice_lines
SET variable_values = lariv_refresh_formula_value_units(variable_values::jsonb)::text
WHERE variable_values IS NOT NULL AND btrim(variable_values) NOT IN ('', '{}');

UPDATE cancelled_invoice_lines
SET variable_values = lariv_refresh_formula_value_units(variable_values::jsonb)::text
WHERE variable_values IS NOT NULL AND btrim(variable_values) NOT IN ('', '{}');
"#;

pub const REFRESH_PURCHASE_VALUE_UNITS: &str = r#"
UPDATE draft_purchase_lines
SET variable_values = lariv_refresh_formula_value_units(variable_values::jsonb)::text
WHERE variable_values IS NOT NULL AND btrim(variable_values) NOT IN ('', '{}');

UPDATE posted_purchase_lines
SET variable_values = lariv_refresh_formula_value_units(variable_values::jsonb)::text
WHERE variable_values IS NOT NULL AND btrim(variable_values) NOT IN ('', '{}');

UPDATE cancelled_purchase_lines
SET variable_values = lariv_refresh_formula_value_units(variable_values::jsonb)::text
WHERE variable_values IS NOT NULL AND btrim(variable_values) NOT IN ('', '{}');
"#;
