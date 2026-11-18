-- Audit trigger for every public base table with a single bigint primary key.
-- Re-running replaces the functions. logging_install_triggers() attaches the trigger
-- and is called at the end of every migration pass, not only from this script.

CREATE OR REPLACE FUNCTION logging_audit_row() RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $fn$
DECLARE
    uid bigint;
    cid bigint;
    rid bigint;
    kind text;
    detail jsonb;
    new_j jsonb;
    old_j jsonb;
    pk_name text;
BEGIN
    IF TG_TABLE_NAME = 'logs' OR TG_NARGS < 1 THEN
        IF TG_OP = 'DELETE' THEN
            RETURN OLD;
        END IF;
        RETURN NEW;
    END IF;

    BEGIN
        uid := NULLIF(current_setting('app.user_id', true), '')::bigint;
    EXCEPTION
        WHEN OTHERS THEN
            uid := NULL;
    END;

    BEGIN
        cid := NULLIF(current_setting('app.company_id', true), '')::bigint;
    EXCEPTION
        WHEN OTHERS THEN
            cid := NULL;
    END;

    pk_name := TG_ARGV[0];
    IF TG_OP = 'INSERT' THEN
        kind := 'Create';
        new_j := to_jsonb(NEW);
        rid := (new_j ->> pk_name)::bigint;
        detail := new_j;
    ELSIF TG_OP = 'UPDATE' THEN
        kind := 'Update';
        new_j := to_jsonb(NEW);
        old_j := to_jsonb(OLD);
        rid := (new_j ->> pk_name)::bigint;
        detail := jsonb_build_object('before', old_j, 'after', new_j);
    ELSE
        kind := 'Delete';
        old_j := to_jsonb(OLD);
        rid := (old_j ->> pk_name)::bigint;
        detail := old_j;
    END IF;

    INSERT INTO logs (created_at, table_name, row_id, log_kind, user_id, company_id, description)
    VALUES (now(), TG_TABLE_NAME, rid, kind, uid, cid, detail);

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$fn$;

CREATE OR REPLACE FUNCTION logging_install_triggers()
RETURNS void
LANGUAGE plpgsql
AS $fn$
DECLARE
    rec record;
    pk_count int;
    pk_names text[];
    pk_types text[];
    pk_name text;
BEGIN
    FOR rec IN
        SELECT c.relname AS table_name, c.oid AS table_oid
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public'
          AND c.relkind = 'r'
          AND c.relname NOT IN ('logs', 'seaql_migrations')
        ORDER BY c.relname
    LOOP
        SELECT count(*),
               array_agg(a.attname ORDER BY a.attnum),
               array_agg(format_type(a.atttypid, a.atttypmod) ORDER BY a.attnum)
        INTO pk_count, pk_names, pk_types
        FROM pg_index i
        JOIN pg_attribute a
          ON a.attrelid = i.indrelid
         AND a.attnum = ANY (i.indkey)
        WHERE i.indrelid = rec.table_oid
          AND i.indisprimary;

        IF pk_count IS DISTINCT FROM 1 OR pk_types[1] IS DISTINCT FROM 'bigint' THEN
            CONTINUE;
        END IF;
        pk_name := pk_names[1];
        EXECUTE format('DROP TRIGGER IF EXISTS logging_audit ON %I', rec.table_name);
        EXECUTE format(
            'CREATE TRIGGER logging_audit AFTER INSERT OR UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION logging_audit_row(%L)',
            rec.table_name,
            pk_name
        );
    END LOOP;
END
$fn$;
