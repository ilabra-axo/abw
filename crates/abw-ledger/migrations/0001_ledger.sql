-- 0001_ledger.sql: the event store (target-system-design §3.1, as written), the migration
-- record (ADR-0004 item 3), the lock helper (§3.2.1) and the grants of §13.2.
--
-- Roles (abw_owner, abw_app, abw_read, abw_crud) are cluster-level and provisioned outside
-- migrations (ADR-0004 item 6). This file runs as abw_owner, which owns every object below.
--
-- Additions to the §3.1 DDL, each cited where it appears:
--   * ledger.schema_migrations (ADR-0004 item 3), immutable like the rest of history;
--   * TRUNCATE triggers on streams, event_types, command_dedup and policy_versions (T6:
--     history rejects UPDATE, DELETE and TRUNCATE by trigger);
--   * USAGE on the events identity sequence for abw_app, so the append can allocate the
--     positions its artifacts cite before step 8 (§3.2.1 step 7, deferred created_pos FK).

CREATE SCHEMA ledger;
REVOKE ALL ON SCHEMA ledger FROM PUBLIC;

-- §3.1.1 Immutability primitive
CREATE FUNCTION ledger.forbid_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'ABW-T6: % on %.% is forbidden (append-only)', TG_OP,
    TG_TABLE_SCHEMA, TG_TABLE_NAME
    USING ERRCODE = 'insufficient_privilege';
END $$;

-- ADR-0004 item 3: applied migrations, by name and sha256: of the file bytes.
CREATE TABLE ledger.schema_migrations (
  name       text PRIMARY KEY,
  sha        text NOT NULL CHECK (sha ~ '^sha256:[0-9a-f]{64}$'),
  applied_at timestamptz NOT NULL DEFAULT now());
CREATE TRIGGER t_migrations_immutable BEFORE UPDATE OR DELETE ON ledger.schema_migrations
  FOR EACH ROW EXECUTE FUNCTION ledger.forbid_mutation();
CREATE TRIGGER t_migrations_no_truncate BEFORE TRUNCATE ON ledger.schema_migrations
  FOR EACH STATEMENT EXECUTE FUNCTION ledger.forbid_mutation();

-- §3.1.2 stream_kinds and streams
CREATE TABLE ledger.stream_kinds (
  kind        text PRIMARY KEY CHECK (kind ~ '^[a-z][a-z_]{1,31}$'),
  concurrency text NOT NULL CHECK (concurrency IN ('exact','commutative')),
  description text NOT NULL);

CREATE TABLE ledger.streams (
  stream_id  text PRIMARY KEY COLLATE "C"
             CHECK (stream_id ~ '^[a-z][a-z_]{1,31}:[A-Za-z0-9._@-]{1,128}$'),
  kind       text NOT NULL REFERENCES ledger.stream_kinds ON DELETE RESTRICT,
  tenant_id  text NOT NULL,
  head       bigint NOT NULL CHECK (head >= 0),
  created_at timestamptz NOT NULL DEFAULT now(),
  CHECK (split_part(stream_id, ':', 1) = kind));
CREATE INDEX streams_tenant_kind ON ledger.streams (tenant_id, kind);

-- head may only move forward; identity columns never change; rows are never deleted
CREATE FUNCTION ledger.streams_head_monotonic() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'ABW-T6: streams rows are never deleted'; END IF;
  IF NEW.stream_id IS DISTINCT FROM OLD.stream_id OR NEW.kind IS DISTINCT FROM OLD.kind
     OR NEW.tenant_id IS DISTINCT FROM OLD.tenant_id OR NEW.head <= OLD.head THEN
    RAISE EXCEPTION 'ABW-T2: streams.% may only advance head (% -> %)',
      OLD.stream_id, OLD.head, NEW.head;
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER t_streams_monotonic BEFORE UPDATE OR DELETE ON ledger.streams
  FOR EACH ROW EXECUTE FUNCTION ledger.streams_head_monotonic();
CREATE TRIGGER t_streams_no_truncate BEFORE TRUNCATE ON ledger.streams
  FOR EACH STATEMENT EXECUTE FUNCTION ledger.forbid_mutation();

-- gap-free: at COMMIT, every advance of head is matched by exactly that many events
CREATE FUNCTION ledger.streams_gap_free() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE lo bigint := CASE WHEN TG_OP = 'INSERT' THEN 0 ELSE OLD.head END; n bigint;
BEGIN
  SELECT count(*) INTO n FROM ledger.events
   WHERE stream_id = NEW.stream_id AND seq > lo AND seq <= NEW.head;
  IF n <> NEW.head - lo THEN
    RAISE EXCEPTION 'ABW-T2: stream % advanced % -> % but % events were written',
      NEW.stream_id, lo, NEW.head, n;
  END IF;
  RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER t_streams_gap_free AFTER INSERT OR UPDATE ON ledger.streams
  DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ledger.streams_gap_free();

-- §3.1.3 event_types: the contract-versioning mechanism (G33)
CREATE TABLE ledger.event_types (
  event_type      text NOT NULL CHECK (event_type ~ '^[a-z][a-z_]*\.[a-z][a-z_]*$'),
  schema_version  int NOT NULL CHECK (schema_version >= 1),
  stream_kind     text NOT NULL REFERENCES ledger.stream_kinds ON DELETE RESTRICT,
  epistemic       text NOT NULL CHECK (epistemic IN
                    ('evidentiary','interpretive','structural','decision','observation')),
  json_schema     jsonb NOT NULL,
  json_schema_sha text NOT NULL CHECK (json_schema_sha ~ '^sha256:[0-9a-f]{64}$'),
  upcasts_from    int CHECK (upcasts_from IS NULL OR upcasts_from = schema_version - 1),
  introduced_by   text NOT NULL,                       -- migration file name
  PRIMARY KEY (event_type, schema_version));

CREATE TABLE ledger.event_type_retirements (
  event_type     text NOT NULL,
  schema_version int NOT NULL,
  retired_by     text NOT NULL,
  retired_at     timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (event_type, schema_version),
  FOREIGN KEY (event_type, schema_version) REFERENCES ledger.event_types ON DELETE RESTRICT);

CREATE TRIGGER t_types_immutable BEFORE UPDATE OR DELETE ON ledger.event_types
  FOR EACH ROW EXECUTE FUNCTION ledger.forbid_mutation();
CREATE TRIGGER t_types_no_truncate BEFORE TRUNCATE ON ledger.event_types
  FOR EACH STATEMENT EXECUTE FUNCTION ledger.forbid_mutation();

-- §3.1.4 events
CREATE TABLE ledger.events (
  global_pos     bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  stream_id      text NOT NULL COLLATE "C" REFERENCES ledger.streams ON DELETE RESTRICT,
  seq            bigint NOT NULL CHECK (seq >= 1),
  event_type     text NOT NULL,
  schema_version int NOT NULL,
  txid           xid8 NOT NULL DEFAULT pg_current_xact_id(),
  payload        jsonb NOT NULL,
  payload_sha    text NOT NULL CHECK (payload_sha ~ '^sha256:[0-9a-f]{64}$'),
  command_id     uuid NOT NULL,                        -- the command that produced this event
  correlation_id uuid NOT NULL,                        -- request / claim correlation
  causation_pos  bigint REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  actor          text NOT NULL,
  actor_kind     text NOT NULL CHECK (actor_kind IN ('human','agent','platform','tool')),
  recorded_at    timestamptz NOT NULL DEFAULT now(),
  UNIQUE (stream_id, seq),
  FOREIGN KEY (event_type, schema_version) REFERENCES ledger.event_types ON DELETE RESTRICT);

CREATE INDEX events_fence       ON ledger.events (txid, global_pos);        -- async projector fence (3.5)
CREATE INDEX events_type_pos    ON ledger.events (event_type, global_pos);  -- per-type projector scans
CREATE INDEX events_correlation ON ledger.events (correlation_id);          -- request reconstruction
CREATE UNIQUE INDEX events_money_idem ON ledger.events (stream_id, (payload->>'idempotency_key'))
  WHERE payload ? 'idempotency_key';

-- seq may not run ahead of the head the CAS just set
CREATE FUNCTION ledger.events_seq_matches_head() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE h bigint;
BEGIN
  SELECT head INTO h FROM ledger.streams WHERE stream_id = NEW.stream_id;
  IF NEW.seq > h THEN
    RAISE EXCEPTION 'ABW-T2: event seq % beyond stream head % for %', NEW.seq, h, NEW.stream_id;
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER t_events_seq BEFORE INSERT ON ledger.events
  FOR EACH ROW EXECUTE FUNCTION ledger.events_seq_matches_head();
CREATE TRIGGER t_events_immutable BEFORE UPDATE OR DELETE ON ledger.events
  FOR EACH ROW EXECUTE FUNCTION ledger.forbid_mutation();
CREATE TRIGGER t_events_no_truncate BEFORE TRUNCATE ON ledger.events
  FOR EACH STATEMENT EXECUTE FUNCTION ledger.forbid_mutation();

-- §3.1.5 artifacts
CREATE TABLE ledger.artifacts (
  sha              text PRIMARY KEY CHECK (sha ~ '^sha256:[0-9a-f]{64}$'),
  media_type       text NOT NULL,
  canonicalization text NOT NULL CHECK (canonicalization IN ('jcs-rfc8785','raw-utf8','raw-bytes')),
  byte_len         bigint NOT NULL CHECK (byte_len >= 0),
  bytes            bytea,                               -- inline when byte_len <= 262144
  storage_ref      text,                                -- object key = sha, otherwise
  created_pos      bigint NOT NULL,                     -- the event that first cited it
  created_at       timestamptz NOT NULL DEFAULT now(),
  CHECK ((bytes IS NULL) <> (storage_ref IS NULL)),
  CHECK (bytes IS NULL OR octet_length(bytes) = byte_len),
  CHECK (bytes IS NULL OR sha = 'sha256:' || encode(sha256(bytes), 'hex')));
ALTER TABLE ledger.artifacts ADD FOREIGN KEY (created_pos) REFERENCES ledger.events (global_pos)
  DEFERRABLE INITIALLY DEFERRED;
CREATE TRIGGER t_artifacts_immutable BEFORE UPDATE OR DELETE ON ledger.artifacts
  FOR EACH ROW EXECUTE FUNCTION ledger.forbid_mutation();
CREATE TRIGGER t_artifacts_no_trunc BEFORE TRUNCATE ON ledger.artifacts
  FOR EACH STATEMENT EXECUTE FUNCTION ledger.forbid_mutation();

-- §3.1.6 command_dedup, aggregate_snapshots, projector_checkpoints, outbox
CREATE TABLE ledger.command_dedup (
  idempotency_key text PRIMARY KEY,                     -- client- or caller-derived
  command_type    text NOT NULL,
  request_sha     text NOT NULL CHECK (request_sha ~ '^sha256:[0-9a-f]{64}$'),
  first_pos       bigint REFERENCES ledger.events (global_pos) DEFERRABLE INITIALLY DEFERRED,
  result          jsonb NOT NULL,                       -- the response returned the first time
  created_at      timestamptz NOT NULL DEFAULT now());
CREATE TRIGGER t_dedup_immutable BEFORE UPDATE OR DELETE ON ledger.command_dedup
  FOR EACH ROW EXECUTE FUNCTION ledger.forbid_mutation();
CREATE TRIGGER t_dedup_no_truncate BEFORE TRUNCATE ON ledger.command_dedup
  FOR EACH STATEMENT EXECUTE FUNCTION ledger.forbid_mutation();

CREATE TABLE ledger.aggregate_snapshots (
  stream_id         text NOT NULL COLLATE "C" REFERENCES ledger.streams ON DELETE RESTRICT,
  seq               bigint NOT NULL,
  aggregate_type    text NOT NULL,
  aggregate_version int NOT NULL,                       -- bump when apply() changes
  state             jsonb NOT NULL,
  state_sha         text NOT NULL,
  created_at        timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (stream_id, aggregate_type, aggregate_version, seq));

CREATE TABLE ledger.projector_checkpoints (
  projector   text PRIMARY KEY,
  fence_txid  xid8 NOT NULL DEFAULT '0'::xid8,
  fence_pos   bigint NOT NULL DEFAULT 0,
  lease_owner text,
  lease_until timestamptz,
  updated_at  timestamptz NOT NULL DEFAULT now());

CREATE TABLE ledger.outbox (
  id              bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  topic           text NOT NULL,
  dedupe_key      text NOT NULL UNIQUE,
  source_pos      bigint NOT NULL REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  payload         jsonb NOT NULL,
  attempts        int NOT NULL DEFAULT 0,
  next_attempt_at timestamptz NOT NULL DEFAULT now(),
  last_error      text,
  done_at         timestamptz,
  dead_at         timestamptz,
  CHECK (NOT (done_at IS NOT NULL AND dead_at IS NOT NULL)));
CREATE INDEX outbox_ready ON ledger.outbox (topic, next_attempt_at)
  WHERE done_at IS NULL AND dead_at IS NULL;

-- §3.1.7 policy_versions
CREATE TABLE ledger.policy_versions (
  policy_id   uuid NOT NULL,
  version     int NOT NULL CHECK (version >= 1),
  policy_kind text NOT NULL CHECK (policy_kind IN ('lattice','gate','evaluator')),
  tenant_id   text,                                     -- NULL = platform default
  doc         jsonb NOT NULL,                           -- see 11.2 for the schema
  doc_sha     text NOT NULL CHECK (doc_sha ~ '^sha256:[0-9a-f]{64}$'),
  supersedes  int,
  accepted_by text NOT NULL,                            -- human actor id
  rationale   text NOT NULL,
  active_from timestamptz NOT NULL,
  event_pos   bigint NOT NULL REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  PRIMARY KEY (policy_id, version),
  CHECK (supersedes IS NULL OR supersedes = version - 1));
CREATE TRIGGER t_policy_immutable BEFORE UPDATE OR DELETE ON ledger.policy_versions
  FOR EACH ROW EXECUTE FUNCTION ledger.forbid_mutation();
CREATE TRIGGER t_policy_no_truncate BEFORE TRUNCATE ON ledger.policy_versions
  FOR EACH STATEMENT EXECUTE FUNCTION ledger.forbid_mutation();

-- §3.2.1 step 3: the lock helper. FOR NO KEY UPDATE on the write set, FOR SHARE on the
-- rest of the read set, once each, in stream_id COLLATE "C" order (rule L1, §3.2.2).
CREATE FUNCTION ledger.lock_streams(p_write text[], p_read text[])
RETURNS TABLE (stream_id text, head bigint, mode text) LANGUAGE plpgsql AS $$
DECLARE r record;
BEGIN
  FOR r IN SELECT s.id, bool_or(s.w) AS w
             FROM (SELECT unnest(p_write) COLLATE "C" AS id, true AS w
                   UNION ALL SELECT unnest(p_read) COLLATE "C", false) s
            GROUP BY s.id ORDER BY s.id
  LOOP
    IF r.w THEN
      RETURN QUERY SELECT x.stream_id::text, x.head, 'write'::text
                     FROM ledger.streams x WHERE x.stream_id = r.id FOR NO KEY UPDATE;
    ELSE
      RETURN QUERY SELECT x.stream_id::text, x.head, 'read'::text
                     FROM ledger.streams x WHERE x.stream_id = r.id FOR SHARE;
    END IF;
  END LOOP;
END $$;

-- §3.4.2: the projection schema and its justification trigger function. Each inline
-- projection table attaches it as a deferred constraint trigger naming its position column.
CREATE SCHEMA proj;
REVOKE ALL ON SCHEMA proj FROM PUBLIC;

CREATE FUNCTION proj.row_justified_by_event() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE pos bigint;
BEGIN
  IF TG_OP = 'DELETE' THEN
    IF NOT EXISTS (SELECT 1 FROM ledger.events WHERE txid = pg_current_xact_id()) THEN
      RAISE EXCEPTION 'ABW-T3: delete on %.% without an event in this transaction',
        TG_TABLE_SCHEMA, TG_TABLE_NAME;
    END IF;
    RETURN NULL;
  END IF;
  pos := (to_jsonb(NEW) ->> TG_ARGV[0])::bigint;
  IF NOT EXISTS (SELECT 1 FROM ledger.events
                  WHERE global_pos = pos AND txid = pg_current_xact_id()) THEN
    RAISE EXCEPTION 'ABW-T3: %.% row cites event % that this transaction did not append',
      TG_TABLE_SCHEMA, TG_TABLE_NAME, pos USING ERRCODE = 'insufficient_privilege';
  END IF;
  RETURN NULL;
END $$;

-- §13.2 grants. abw_app: SELECT on ledger; INSERT on the append tables; UPDATE only on
-- streams.head, outbox status columns and projector checkpoint columns. No UPDATE, DELETE
-- or TRUNCATE on any history table. abw_read: SELECT only.
GRANT USAGE ON SCHEMA ledger, proj TO abw_app, abw_read;
GRANT SELECT ON ALL TABLES IN SCHEMA ledger TO abw_app, abw_read;
GRANT INSERT ON ledger.events, ledger.artifacts, ledger.streams, ledger.command_dedup,
  ledger.aggregate_snapshots, ledger.outbox TO abw_app;
GRANT UPDATE (head) ON ledger.streams TO abw_app;
GRANT UPDATE (attempts, next_attempt_at, last_error, done_at, dead_at) ON ledger.outbox TO abw_app;
GRANT UPDATE (fence_txid, fence_pos, lease_owner, lease_until, updated_at)
  ON ledger.projector_checkpoints TO abw_app;
GRANT USAGE ON SEQUENCE ledger.events_global_pos_seq TO abw_app;
REVOKE EXECUTE ON FUNCTION ledger.lock_streams(text[], text[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ledger.lock_streams(text[], text[]) TO abw_app;
