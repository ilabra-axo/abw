-- 0002_probe.sql: the conformance probe aggregate (ADR-0004 item 1).
--
-- Stream kind `probe` (concurrency exact), event type `probe.recorded` v1 (structural,
-- payload any JSON object) and its inline projection proj.probes with the T3
-- justification trigger (target-system-design §3.4.2). Inert in production: no domain
-- path writes it.

INSERT INTO ledger.stream_kinds (kind, concurrency, description)
VALUES ('probe', 'exact', 'ADR-0004 conformance probe; written only by abw_ledger::probe');

-- json_schema_sha is sha256: over the JCS bytes of json_schema; the boot check compares
-- it with the Rust registry (§3.7, OD-08).
INSERT INTO ledger.event_types
  (event_type, schema_version, stream_kind, epistemic, json_schema, json_schema_sha,
   upcasts_from, introduced_by)
VALUES ('probe.recorded', 1, 'probe', 'structural', '{"type":"object"}',
        'sha256:a2c799262a3ce3c19ef5cdd983bf3d12b43ab3c426227091b909dcb7054738c0',
        NULL, '0002_probe.sql');

-- One row per probe stream (rule L2: keyed 1:1 by its stream), last applied event in
-- applied_pos.
CREATE TABLE proj.probes (
  stream_id   text NOT NULL UNIQUE COLLATE "C",
  seq         bigint NOT NULL CHECK (seq >= 1),
  body        jsonb NOT NULL,
  applied_pos bigint NOT NULL REFERENCES ledger.events (global_pos) ON DELETE RESTRICT);
CREATE CONSTRAINT TRIGGER t_probes_justified AFTER INSERT OR UPDATE OR DELETE ON proj.probes
  DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
  EXECUTE FUNCTION proj.row_justified_by_event('applied_pos');

-- §13.2: abw_app INSERT, UPDATE on non-money proj.* (guarded by the trigger); abw_read SELECT.
GRANT SELECT ON proj.probes TO abw_app, abw_read;
GRANT INSERT, UPDATE ON proj.probes TO abw_app;
