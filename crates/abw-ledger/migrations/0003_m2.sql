-- 0003_m2.sql: agents, episodes, allowance and policy versions (ADR-0008 §2-§4), and the
-- first CRUD table (ADR-0008 §4 `crud`, target-system-design §14.5).
--
-- Stream kinds and event types are those of ADR-0008 §2, all `exact` and schema version 1.
-- Projections are inline and carry the §3.4.2 justification trigger on their position
-- column. Deviations from the design DDL, each for a reason given where it appears:
--   * proj.episodes has no CHECK ((status = 'running') = (closed_pos IS NULL)): a CHECK
--     runs per statement, before the deferred justification trigger, so it would answer an
--     unjustified UPDATE with 23514 instead of ABW-T3 (the T3 test for this table);
--   * proj.allowances has no foreign key on stream_id, for the same reason as proj.probes
--     (ADR-0005 item 4): an unjustified INSERT must reach the T3 trigger;
--   * agent ids are text, the stream-id grammar (ADR-0008 §3), not uuid.

-- §3.1.2: the M2 stream kinds.
INSERT INTO ledger.stream_kinds (kind, concurrency, description) VALUES
  ('policy', 'exact', 'one policy''s versions (target-system-design §3.1.7, §11.2)'),
  ('agent', 'exact', 'one agent''s versions (target-system-design §4.2)'),
  ('episode', 'exact', 'one run (target-system-design §4.6)'),
  ('allowance', 'exact', 'one owner''s allowance (economy.md §3)');

-- §3.1.3: the M2 event types. json_schema_sha is sha256: over the JCS bytes of json_schema;
-- boot compares both with the Rust registry (§3.7, OD-08). allowance.funded arrives with
-- M4 (ADR-0008 §1 item 6) and is deliberately absent.
INSERT INTO ledger.event_types
  (event_type, schema_version, stream_kind, epistemic, json_schema, json_schema_sha,
   upcasts_from, introduced_by)
VALUES
  ('policy.adjusted', 1, 'policy', 'decision',
   '{"type":"object","required":["policy_id","version","policy_kind","doc_sha","accepted_by","rationale","active_from"]}',
   'sha256:2fb2770b7ccd1f96b383ab14d7f2de281b864b1ced2dea473684c18158d3b38e', NULL, '0003_m2.sql'),
  ('agent.version_created', 1, 'agent', 'structural',
   '{"type":"object","required":["agent_id","version_number","card_sha","prompt_sha","reason"]}',
   'sha256:e615656445c823261874d7047860095d7678108ebc7d0245c5d8d026a0d5c126', NULL, '0003_m2.sql'),
  ('agent.retired', 1, 'agent', 'structural',
   '{"type":"object","required":["agent_id","reason"]}',
   'sha256:e2cdf591e0808237f9de463a7cb6a5839b07b0ba2087096f68141da854122a3f', NULL, '0003_m2.sql'),
  ('episode.opened', 1, 'episode', 'evidentiary',
   '{"type":"object","required":["episode_id","tenant_id","owner_id","agent_id","version_number","card_sha","input_sha","hold_id","hold_cu","price_version","tools_allowed","opened_at"]}',
   'sha256:21149959c5dfd098ef9a7b414f7b21071432cc21beb8f044dbf616743981609f', NULL, '0003_m2.sql'),
  ('tool.invoked', 1, 'episode', 'evidentiary',
   '{"type":"object","required":["iteration","call_index","tool_name","tool_version","allowed","input_sha","output_sha","duration_ms","error_class"]}',
   'sha256:5fb708a14d3102828a2b1d1a80284db1cdcbdb1e04d502a9a6f7a4068f74dfd7', NULL, '0003_m2.sql'),
  ('gate.decided', 1, 'episode', 'decision',
   '{"type":"object","required":["gate","decision","reason_class","subject","episode_id"]}',
   'sha256:6e86bb8518904bd3def2ad28db6a948fea52cab7d0bb140e5103b1628aa7217b', NULL, '0003_m2.sql'),
  ('episode.closed', 1, 'episode', 'evidentiary',
   '{"type":"object","required":["status","reason_class","raw_output_sha","tool_call_shas","tokens_in","tokens_out","tool_calls","consumed_cu"]}',
   'sha256:72a43e4531365b7ca4881090e5e31a52b952c48bc73c20c46d14f46786a32a96', NULL, '0003_m2.sql'),
  ('allowance.opened', 1, 'allowance', 'evidentiary',
   '{"type":"object","required":["owner_id","idempotency_key"]}',
   'sha256:ca11a2750bcf7275b35c5bb802de87a5587d87782c44a133958d86dabfc8de12', NULL, '0003_m2.sql'),
  ('allowance.granted', 1, 'allowance', 'evidentiary',
   '{"type":"object","required":["owner_id","kind","amount_cu","grant_policy_version","idempotency_key"]}',
   'sha256:ef3a7c13f795838f3c1ae426677a88f33a54cc3b906b532395d06bd88139dfe6', NULL, '0003_m2.sql'),
  ('allowance.hold_placed', 1, 'allowance', 'evidentiary',
   '{"type":"object","required":["owner_id","episode_id","hold_id","amount_cu","workspace_id","idempotency_key"]}',
   'sha256:e774fd8c0ae5c1571f1ec28757450919f252033fdccd155e40c2f10604aef1a5', NULL, '0003_m2.sql'),
  ('allowance.consumed', 1, 'allowance', 'evidentiary',
   '{"type":"object","required":["owner_id","episode_id","hold_id","held_cu","priced_cu","consumed_cu","released_cu","split","usage","price_version","idempotency_key"]}',
   'sha256:f3ea2db089f44fc41958ac61e4e1dffb7c511ba2873512d9d55cbdb9547e151a', NULL, '0003_m2.sql'),
  ('allowance.hold_released', 1, 'allowance', 'evidentiary',
   '{"type":"object","required":["owner_id","episode_id","hold_id","amount_cu","reason_class","idempotency_key"]}',
   'sha256:93515404722c6cbab45d49919c72bf8dfd8496dc67531dafa0a86f75153718b6', NULL, '0003_m2.sql');

-- §3.1.7, ADR-0008 §1 item 7: grant and price policies are policy versions. The row is
-- written by the policy.adjusted projector in the append transaction.
ALTER TABLE ledger.policy_versions DROP CONSTRAINT policy_versions_policy_kind_check;
ALTER TABLE ledger.policy_versions ADD CONSTRAINT policy_versions_policy_kind_check
  CHECK (policy_kind IN ('lattice','gate','evaluator','grant','price'));
CREATE INDEX policy_versions_resolution ON ledger.policy_versions
  (policy_kind, tenant_id, event_pos DESC);
GRANT INSERT ON ledger.policy_versions TO abw_app;

-- §4.2: agent versions (immutable) and heads.
CREATE TABLE proj.agent_versions (
  agent_id       text NOT NULL,
  tenant_id      text NOT NULL,
  version_number int NOT NULL CHECK (version_number >= 1),
  card_sha       text NOT NULL REFERENCES ledger.artifacts ON DELETE RESTRICT,
  prompt_sha     text NOT NULL CHECK (prompt_sha ~ '^sha256:[0-9a-f]{64}$'),
  reason         text NOT NULL CHECK (reason IN ('seed','owner_edit','adopt_file','revert')),
  actor          text NOT NULL,
  event_pos      bigint NOT NULL UNIQUE REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  PRIMARY KEY (agent_id, version_number));
CREATE INDEX agent_versions_by_sha ON proj.agent_versions (card_sha);
CREATE TRIGGER t_agent_versions_immutable BEFORE UPDATE OR DELETE ON proj.agent_versions
  FOR EACH ROW EXECUTE FUNCTION ledger.forbid_mutation();
CREATE TRIGGER t_agent_versions_no_truncate BEFORE TRUNCATE ON proj.agent_versions
  FOR EACH STATEMENT EXECUTE FUNCTION ledger.forbid_mutation();
CREATE CONSTRAINT TRIGGER t_agent_versions_justified AFTER INSERT ON proj.agent_versions
  DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
  EXECUTE FUNCTION proj.row_justified_by_event('event_pos');

CREATE TABLE proj.agent_heads (
  agent_id       text PRIMARY KEY,
  tenant_id      text NOT NULL,
  version_number int NOT NULL,
  card_sha       text NOT NULL,
  retired        boolean NOT NULL,
  applied_pos    bigint NOT NULL REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  FOREIGN KEY (agent_id, version_number) REFERENCES proj.agent_versions ON DELETE RESTRICT);
CREATE INDEX agent_heads_tenant ON proj.agent_heads (tenant_id, agent_id);
CREATE CONSTRAINT TRIGGER t_agent_heads_justified AFTER INSERT OR UPDATE OR DELETE
  ON proj.agent_heads DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
  EXECUTE FUNCTION proj.row_justified_by_event('applied_pos');

-- §4.6: episodes, plus the owner (ADR-0008 §3) that allowance holds are checked against.
CREATE TABLE proj.episodes (
  episode_id     uuid PRIMARY KEY,
  stream_id      text NOT NULL UNIQUE COLLATE "C" REFERENCES ledger.streams ON DELETE RESTRICT,
  tenant_id      text NOT NULL,
  owner_id       text NOT NULL,
  agent_id       text NOT NULL,
  version_number int NOT NULL CHECK (version_number >= 1),
  card_sha       text NOT NULL REFERENCES ledger.artifacts ON DELETE RESTRICT,
  workspace_id   uuid,
  hold_id        uuid NOT NULL,
  hold_cu        bigint NOT NULL CHECK (hold_cu > 0),
  consumed_cu    bigint CHECK (consumed_cu >= 0),
  status         text NOT NULL CHECK (status IN ('running','success','partial','failure')),
  reason_class   text CHECK (reason_class IN ('budget_exhausted','iterations_exhausted',
                   'provider_error','ledger_error','abandoned_timeout','abandoned_dropped')),
  input_sha      text NOT NULL REFERENCES ledger.artifacts ON DELETE RESTRICT,
  raw_output_sha text REFERENCES ledger.artifacts ON DELETE RESTRICT,
  tool_call_shas text[] NOT NULL DEFAULT '{}',
  opened_at      timestamptz NOT NULL,
  opened_pos     bigint NOT NULL REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  closed_pos     bigint REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  applied_pos    bigint NOT NULL REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  UNIQUE (episode_id, owner_id));
CREATE INDEX episodes_agent_recent ON proj.episodes (agent_id, opened_pos DESC);
CREATE INDEX episodes_tenant_recent ON proj.episodes (tenant_id, opened_pos DESC);
CREATE INDEX episodes_running ON proj.episodes (opened_at) WHERE status = 'running';
CREATE CONSTRAINT TRIGGER t_episodes_justified AFTER INSERT OR UPDATE OR DELETE
  ON proj.episodes DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
  EXECUTE FUNCTION proj.row_justified_by_event('applied_pos');

-- economy.md §3: one row per owner, three buckets and lifetime totals. ECO-7: every amount
-- is non-negative and available is exactly the buckets less what is held.
CREATE TABLE proj.allowances (
  owner_id                text PRIMARY KEY,
  stream_id               text NOT NULL UNIQUE COLLATE "C",
  tenant_id               text NOT NULL,
  free_tier               bigint NOT NULL CHECK (free_tier >= 0),
  granted                 bigint NOT NULL CHECK (granted >= 0),
  purchased               bigint NOT NULL CHECK (purchased >= 0),
  held                    bigint NOT NULL CHECK (held >= 0),
  available               bigint NOT NULL CHECK (available >= 0),
  total_granted_free_tier bigint NOT NULL CHECK (total_granted_free_tier >= 0),
  total_granted           bigint NOT NULL CHECK (total_granted >= 0),
  total_purchased         bigint NOT NULL CHECK (total_purchased >= 0),
  total_consumed          bigint NOT NULL CHECK (total_consumed >= 0),
  applied_pos             bigint NOT NULL REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  CHECK (stream_id = 'allowance:' || owner_id),
  CHECK (available = free_tier + granted + purchased - held));
CREATE CONSTRAINT TRIGGER t_allowances_justified AFTER INSERT OR UPDATE OR DELETE
  ON proj.allowances DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
  EXECUTE FUNCTION proj.row_justified_by_event('applied_pos');

-- ECO-4: a hold cites an episode owned by the allowance's owner. The episode row is
-- written after the hold in TX-A (event order), so the check runs at COMMIT.
CREATE TABLE proj.allowance_holds (
  hold_id     uuid PRIMARY KEY,
  owner_id    text NOT NULL REFERENCES proj.allowances ON DELETE RESTRICT,
  episode_id  uuid NOT NULL UNIQUE,
  tenant_id   text NOT NULL,
  amount_cu   bigint NOT NULL CHECK (amount_cu > 0),
  status      text NOT NULL CHECK (status IN ('open','consumed','released')),
  applied_pos bigint NOT NULL REFERENCES ledger.events (global_pos) ON DELETE RESTRICT,
  FOREIGN KEY (episode_id, owner_id) REFERENCES proj.episodes (episode_id, owner_id)
    DEFERRABLE INITIALLY DEFERRED);
CREATE CONSTRAINT TRIGGER t_allowance_holds_justified AFTER INSERT OR UPDATE OR DELETE
  ON proj.allowance_holds DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
  EXECUTE FUNCTION proj.row_justified_by_event('applied_pos');

-- §13.2: abw_app INSERT, UPDATE on the projections (guarded by the justification triggers
-- and the CHECKs above); INSERT only on the immutable version rows. abw_read SELECT.
GRANT SELECT ON proj.agent_versions, proj.agent_heads, proj.episodes, proj.allowances,
  proj.allowance_holds TO abw_app, abw_read;
GRANT INSERT ON proj.agent_versions TO abw_app;
GRANT INSERT, UPDATE ON proj.agent_heads, proj.episodes, proj.allowances,
  proj.allowance_holds TO abw_app;

-- §14.5 with core-delta §2.4 (schema crud, not public): the CRUD domain. abw_ledger::crud
-- writes it as abw_crud (SET LOCAL ROLE); abw_read reads it. API keys are stored only as
-- the sha256: of the secret (ADR-0008 §1 item 3).
CREATE SCHEMA crud;
REVOKE ALL ON SCHEMA crud FROM PUBLIC;
CREATE TABLE crud.api_keys (
  key_id     uuid PRIMARY KEY,
  tenant_id  text NOT NULL,
  owner_id   text NOT NULL,
  actor_id   text NOT NULL,
  key_sha    text NOT NULL UNIQUE CHECK (key_sha ~ '^sha256:[0-9a-f]{64}$'),
  label      text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz);
GRANT USAGE ON SCHEMA crud TO abw_crud, abw_read;
GRANT SELECT, INSERT, UPDATE (revoked_at) ON crud.api_keys TO abw_crud;
GRANT SELECT ON crud.api_keys TO abw_read;
