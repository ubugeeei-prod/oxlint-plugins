'use strict';

// Oxlint plugin port of eslint-plugin-postgresql (MIT).
// SQL is parsed (libpg_query, PostgreSQL 17) and every rule runs in Rust through
// NAPI-RS; this JavaScript layer is only the Oxlint/ESLint-compat adapter.
// Oxlint does not route .sql files to jsPlugins, so this package exposes the
// adapter and native API while keeping CLI integration disabled in status
// metadata (mirroring the @eslint/json and @eslint/markdown ports).

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedPostgresqlRuleNames, scanPostgresql } = require('./api.js');

const PLUGIN_NAME = 'postgresql';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/postgresql';
const diagnosticsCache = new WeakMap();

// Per-rule ESLint `meta` (description, messages, fixable, schema), keyed by rule
// name. Entries are added as each upstream rule is ported.
const ruleMeta = Object.freeze({
  'consistent-as-for-column-alias': {
    type: 'layout',
    description:
      'Enforce a consistent stance on the `AS` keyword before column aliases in `SELECT` (either always require it, or always forbid it)',
    recommended: false,
    fixable: 'code',
    schema: [
      {
        type: 'object',
        properties: {
          style: { enum: ['always', 'never'] },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      preferAs: 'Use `AS` before the column alias `{{alias}}`.',
      unexpectedAs: 'Omit `AS` before the column alias `{{alias}}`.',
    },
  },
  'consistent-between-over-and': {
    type: 'suggestion',
    description:
      'Enforce a consistent stance on `x BETWEEN a AND b` vs `x >= a AND x <= b` for closed-interval range checks',
    recommended: false,
    fixable: 'code',
    schema: [
      {
        type: 'object',
        properties: {
          style: { enum: ['always', 'never'] },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      preferBetween: 'Use `{{lhs}} BETWEEN {{lower}} AND {{upper}}` instead of `>= ... AND <=`.',
      unexpectedBetween:
        'Use `{{lhs}} >= {{lower}} AND {{lhs}} <= {{upper}}` instead of `BETWEEN`. Some teams prefer explicit comparisons so the inclusive bounds are obvious to readers.',
    },
  },
  'consistent-create-or-replace': {
    type: 'suggestion',
    description:
      'Enforce a consistent stance on `CREATE OR REPLACE` for `FUNCTION` / `PROCEDURE` / `VIEW` (either always require it, or always forbid it)',
    recommended: false,
    fixable: undefined,
    schema: [
      {
        type: 'object',
        properties: {
          style: { enum: ['always', 'never'] },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      preferOrReplace:
        'Use `CREATE OR REPLACE {{kind}}` so re-running this migration does not abort with `relation already exists`.',
      unexpectedOrReplace:
        'Avoid `CREATE OR REPLACE {{kind}}`; drop and re-create the object explicitly so unintended overwrites are surfaced.',
    },
  },
  'consistent-explicit-inner-join': {
    type: 'layout',
    description:
      'Enforce a consistent stance on the explicit `INNER` keyword in `INNER JOIN` (either always require it, or always forbid it)',
    recommended: false,
    fixable: 'code',
    schema: [
      {
        type: 'object',
        properties: {
          style: { enum: ['always', 'never'] },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      preferInnerJoin: 'Write `INNER JOIN` explicitly instead of bare `JOIN`.',
      unexpectedInnerJoin: 'Omit the redundant `INNER`; use bare `JOIN` for inner joins.',
    },
  },
  'consistent-identity-over-serial': {
    type: 'suggestion',
    description:
      'Enforce a consistent stance on `GENERATED ... AS IDENTITY` vs `SERIAL` / `BIGSERIAL` / `SMALLSERIAL`',
    recommended: true,
    fixable: undefined,
    schema: [
      {
        type: 'object',
        properties: {
          style: { enum: ['always', 'never'] },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      preferIdentity:
        'Use `GENERATED ALWAYS AS IDENTITY` (SQL standard) instead of `{{type}}`. The serial pseudo-types create a separately-owned sequence that breaks under pg_dump round-trips and does not honor column privileges.',
      unexpectedIdentity:
        'Use a serial pseudo-type (e.g. `bigserial`) instead of `GENERATED ... AS IDENTITY`. Useful for projects that need to keep compatibility with tooling that does not understand identity columns.',
    },
  },
  'consistent-reindex-concurrently': {
    type: 'problem',
    description:
      'Enforce a consistent stance on `CONCURRENTLY` for `REINDEX` (either always require it, or always forbid it)',
    recommended: true,
    fixable: undefined,
    schema: [
      {
        type: 'object',
        properties: {
          style: { enum: ['always', 'never'] },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      preferReindexConcurrently:
        '`REINDEX` without `CONCURRENTLY` takes a `SHARE` lock (table) or `ACCESS EXCLUSIVE` (index), blocking writers for the rebuild. Use `REINDEX (TABLE|INDEX) CONCURRENTLY ...` (PG ≥ 12) so writers keep working.',
      unexpectedReindexConcurrently:
        'Avoid `REINDEX CONCURRENTLY`. Concurrent reindex cannot run inside a transaction; use plain `REINDEX` when the migration tool wraps each step in BEGIN/COMMIT.',
    },
  },
  'no-add-check-constraint-without-not-valid': {
    type: 'problem',
    description:
      'Disallow `ALTER TABLE ... ADD CONSTRAINT ... CHECK (...)` without `NOT VALID`; the synchronous form holds `ACCESS EXCLUSIVE` on the table for the entire validating scan',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      checkNotValid:
        'Add this CHECK constraint with `NOT VALID` and run `VALIDATE CONSTRAINT` separately, so the validating scan does not block writers under `ACCESS EXCLUSIVE`.',
    },
  },
  'no-add-column-not-null-without-default': {
    type: 'problem',
    description:
      'Disallow `ALTER TABLE ADD COLUMN ... NOT NULL` without a `DEFAULT` because the migration fails outright on any non-empty table',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noAddColumnNotNullWithoutDefault:
        '`ADD COLUMN ... NOT NULL` without a `DEFAULT` aborts the migration on any table that already has rows. Either supply a `DEFAULT`, or add the column nullable first, backfill, and then `ALTER COLUMN ... SET NOT NULL` in a follow-up.',
    },
  },
  'no-add-unique-constraint-directly': {
    type: 'problem',
    description:
      'Disallow `ALTER TABLE ... ADD CONSTRAINT ... UNIQUE (...)` directly; build the index with `CREATE UNIQUE INDEX CONCURRENTLY` first, then promote it',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      useIndexFirst:
        "Build this UNIQUE constraint's index with `CREATE UNIQUE INDEX CONCURRENTLY` first, then promote it via `ALTER TABLE ... ADD CONSTRAINT ... UNIQUE USING INDEX <name>`. The inline form blocks writers under `ACCESS EXCLUSIVE` for the entire index build.",
    },
  },
  'no-alter-column-type': {
    type: 'problem',
    description:
      'Disallow `ALTER TABLE ... ALTER COLUMN ... TYPE ...` because it can rewrite the table under an ACCESS EXCLUSIVE lock',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noAlterColumnType:
        '`ALTER COLUMN ... TYPE` can rewrite the entire table under an ACCESS EXCLUSIVE lock. For non-trivial tables, add a new column, dual-write, backfill, and swap — or use `USING` only for known-safe conversions in a separate migration.',
    },
  },
  'no-cluster': {
    type: 'problem',
    description:
      'Disallow the `CLUSTER` statement: it takes ACCESS EXCLUSIVE, rewrites the table, and is not maintained afterwards',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noCluster:
        '`CLUSTER` takes `ACCESS EXCLUSIVE` and rewrites the entire table, just like `VACUUM FULL` — and PostgreSQL does not keep the rows clustered as you continue to write. Use `pg_repack --order-by` for online clustering, or build an index in the order you actually want to read.',
    },
  },
  'no-composite-primary-key': {
    type: 'problem',
    description: 'Disallow composite PRIMARY KEY constraints (more than one column)',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      noCompositePk:
        'Composite PRIMARY KEY is not allowed. Use a single-column surrogate key (e.g. `id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY`) and enforce the natural key with a `UNIQUE` constraint. Composite primary keys complicate joins, ORM mapping, and foreign-key references.',
    },
  },
  'no-create-role': {
    type: 'suggestion',
    description:
      'Disallow `CREATE ROLE` / `CREATE USER` in application migrations; manage roles in a separate operator workflow',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noCreateRole:
        '`CREATE ROLE` / `CREATE USER` belongs in an operator-managed bootstrap (Terraform, Pulumi, a runbook), not in application migrations. Migration files run with whichever role the deploy uses and are not the right place to manage permissions.',
    },
  },
  'no-cross-join': {
    type: 'suggestion',
    description: 'Disallow `CROSS JOIN` (unqualified cartesian product)',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noCrossJoin:
        'Avoid `CROSS JOIN`. Cartesian products are almost always a mistake; use an explicit `JOIN ... ON` with a join condition, or `JOIN ... ON true` if you really do want one.',
    },
  },
  'no-distinct-on-without-order-by': {
    type: 'problem',
    description:
      'Disallow `SELECT DISTINCT ON (...)` without an `ORDER BY`; the surviving row in each group is otherwise non-deterministic',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noDistinctOnWithoutOrderBy:
        '`DISTINCT ON (...)` keeps an arbitrary row from each group unless `ORDER BY` is specified. Add an `ORDER BY` whose leading columns match the `DISTINCT ON` expressions.',
    },
  },
  'no-drop-column': {
    type: 'problem',
    description:
      'Disallow `ALTER TABLE ... DROP COLUMN` — every reader of the dropped column breaks at deploy time',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noDropColumn:
        '`DROP COLUMN` breaks every running app that still references the column. Roll it out as a two-step migration: stop reading the column in the application, deploy, then drop it in a follow-up release.',
    },
  },
  'no-drop-database': {
    type: 'problem',
    description:
      'Disallow `DROP DATABASE`; it is catastrophic if run by accident and should not live in versioned SQL',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noDropDatabase:
        '`DROP DATABASE` is catastrophic and irreversible. Database creation/deletion belongs in an explicit operator workflow, not in versioned SQL applied automatically by a migration tool.',
    },
  },
  'no-drop-not-null': {
    type: 'suggestion',
    description:
      'Disallow `ALTER COLUMN ... DROP NOT NULL` — relaxing a NOT NULL constraint surprises every consumer that already assumes the column is non-null',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noDropNotNull:
        '`DROP NOT NULL` lets the column store NULLs again — every consumer that already assumes the column is non-null (joins, COALESCE coverage, app-level types) silently breaks. If a row genuinely needs no value, model it with a sentinel or a separate optional table.',
    },
  },
  'no-drop-schema-cascade': {
    type: 'problem',
    description:
      'Disallow `DROP SCHEMA ... CASCADE`; it silently removes every object in the schema',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noDropSchemaCascade:
        "`DROP SCHEMA ... CASCADE` removes every table, view, function, and sequence in the schema with no preview. List the objects you actually want to drop instead, or drop the schema only when it's already empty.",
    },
  },
  'no-drop-table-cascade': {
    type: 'problem',
    description: 'Disallow `DROP TABLE ... CASCADE` because it silently removes dependent objects',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noCascade:
        'Avoid `DROP TABLE ... CASCADE`. CASCADE silently removes dependent objects (views, foreign keys, sequences); list them explicitly so reviewers can see the blast radius.',
    },
  },
  'no-equality-with-null': {
    type: 'problem',
    description:
      "Disallow `x = NULL` / `x <> NULL`; PostgreSQL's three-valued logic makes both expressions evaluate to NULL (i.e. neither true nor false), which silently filters away rows the author probably wanted",
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      useIsNull:
        '`{{op}} NULL` always evaluates to NULL (treated as false). Use `IS NULL` / `IS NOT NULL` instead.',
    },
  },
  'no-grant-all': {
    type: 'problem',
    description:
      'Disallow `GRANT ALL` / `GRANT ALL PRIVILEGES`; enumerate the privileges actually needed so the grant is auditable and a future PG release adding a new privilege does not silently extend it',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      noGrantAll:
        '`GRANT ALL` (or `GRANT ALL PRIVILEGES`) is opaque — list the privileges you actually need (e.g. `SELECT, INSERT, UPDATE`) so the grant is auditable and adding a new PostgreSQL privilege in a future release does not silently extend it.',
    },
  },
  'no-grant-to-public': {
    type: 'problem',
    description: 'Disallow GRANT statements that target the `PUBLIC` pseudo-role',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noPublic:
        'Avoid `GRANT ... TO PUBLIC`. The PUBLIC role covers every current and future role in the database, including ones added later for unrelated services. Name the role(s) you actually want to grant to.',
    },
  },
  'no-group-by-ordinal': {
    type: 'suggestion',
    description:
      'Disallow `GROUP BY <position>` (positional/ordinal references); use the column name or expression instead',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noGroupByOrdinal:
        '`GROUP BY <position>` silently breaks when the SELECT list changes. Use the column name or the expression itself.',
    },
  },
  'no-having-without-group-by': {
    type: 'problem',
    description:
      'Disallow `HAVING` without `GROUP BY` — the query aggregates the entire result set, which is almost never the intended shape',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noHavingWithoutGroupBy:
        '`HAVING` without `GROUP BY` collapses the query to one aggregate row over the whole table. If that is intended, put the predicate in `WHERE`. Otherwise, add a `GROUP BY`.',
    },
  },
  'no-select-star': {
    type: 'suggestion',
    description:
      'Disallow `SELECT *` in queries to keep result schemas stable when the underlying table changes',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      noSelectStar:
        'Avoid `SELECT *`; list the columns you need so the result schema does not silently change when the table does.',
    },
  },
  'no-identifier-too-long': {
    type: 'problem',
    description:
      "Disallow identifiers longer than PostgreSQL's `NAMEDATALEN - 1` limit (default 63 bytes)",
    recommended: true,
    fixable: undefined,
    schema: [
      {
        type: 'object',
        properties: {
          max: { type: 'integer', minimum: 1 },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      identifierTooLong:
        'Identifier `{{name}}` is {{length}} bytes, which exceeds the {{max}}-byte limit. PostgreSQL silently truncates over-length identifiers at parse time, so the object will be created (or looked up) under a different name than written — every later `DROP` / `ALTER` / `\\d` that uses the original name will then fail with `does not exist`.',
    },
  },
  'no-implicit-join': {
    type: 'suggestion',
    description:
      'Disallow comma-separated FROM clauses (implicit cross joins); use explicit JOIN syntax',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noImplicitJoin:
        'Comma-separated tables in `FROM` are an implicit cross join. Use explicit `JOIN ... ON ...` so the join condition lives next to the join.',
    },
  },
  'no-leading-wildcard-like': {
    type: 'suggestion',
    description:
      'Disallow `LIKE`/`ILIKE` patterns that begin with `%` because they cannot use a B-tree index and force a full scan',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noLeadingWildcardLike:
        '`LIKE`/`ILIKE` patterns that begin with `%` cannot use a B-tree index and force a sequential scan. If you need substring search, use a `pg_trgm` GIN index, full-text search, or rework the schema so the prefix is indexable.',
    },
  },
  'no-natural-join': {
    type: 'problem',
    description: 'Disallow `NATURAL JOIN`',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noNaturalJoin:
        'Avoid `NATURAL JOIN`. The join columns are implicit — any future column with a matching name on both sides silently changes the result. Use `JOIN ... USING (...)` or `JOIN ... ON ...` and name the columns.',
    },
  },
  'no-not-in-subquery': {
    type: 'problem',
    description:
      'Disallow `NOT IN (subquery)` because it returns no rows when the subquery yields any NULL',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noNotInSubquery:
        '`NOT IN (subquery)` returns no rows if the subquery yields any NULL — almost certainly not what you want. Use `NOT EXISTS (SELECT 1 FROM ... WHERE ...)` instead; it handles NULL correctly.',
    },
  },
  'no-on-delete-cascade': {
    type: 'problem',
    description:
      'Disallow `ON DELETE CASCADE` on foreign keys; cascading deletes are easy to write but can wipe out far more rows than the author intended',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      noCascade:
        'Avoid `ON DELETE CASCADE`. The deletion will silently propagate through every dependent row; prefer an explicit `RESTRICT` or `SET NULL` action and handle the cleanup in application code.',
    },
  },
  'no-order-by-ordinal': {
    type: 'suggestion',
    description:
      'Disallow `ORDER BY <position>` (positional/ordinal references); use the column name or alias instead',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noOrderByOrdinal:
        '`ORDER BY <position>` silently breaks when the SELECT list changes. Use the column name or alias instead.',
    },
  },
  'no-rename-column': {
    type: 'problem',
    description:
      'Disallow `ALTER TABLE ... RENAME COLUMN` — every deployed reader of the old name breaks at deploy time',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noRenameColumn:
        '`RENAME COLUMN` breaks every running app that still selects/inserts by the old name. The safer pattern is to add a new column, dual-write, backfill, and drop the old one across separate deploys.',
    },
  },
  'no-rename-table': {
    type: 'problem',
    description:
      'Disallow `ALTER TABLE ... RENAME TO` — every deployed reader of the old name breaks at deploy time',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noRenameTable:
        "`RENAME TO` breaks every running app that still queries the old name. The safer pattern is `CREATE VIEW old AS SELECT * FROM new` so old callers keep working until they're migrated, then drop the view in a separate deploy.",
    },
  },
  'no-rule': {
    type: 'problem',
    description:
      "Disallow `CREATE RULE`; PostgreSQL's rule system is a known foot-gun and is effectively deprecated in favor of triggers and views",
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      noRule:
        "Avoid `CREATE RULE`. PostgreSQL's rule system has surprising semantics around row counts, RETURNING, and updatable views; use a trigger or an updatable view instead.",
    },
  },
  'no-security-definer-without-search-path': {
    type: 'problem',
    description:
      'Require `SECURITY DEFINER` functions to also `SET search_path = ...` to prevent search-path injection attacks',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      missingSearchPath:
        "`SECURITY DEFINER` function must `SET search_path = ...` (e.g. `pg_catalog, pg_temp`) so an attacker-controlled schema in the caller's `search_path` cannot shadow built-in objects called from inside the function body.",
    },
  },
  'no-select-into': {
    type: 'suggestion',
    description:
      'Disallow `SELECT ... INTO target FROM ...` (creates a new table); use `CREATE TABLE AS SELECT ...` instead',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noSelectInto:
        "`SELECT ... INTO target FROM ...` creates a new table whose semantics differ from a regular `SELECT` and conflict with PL/pgSQL's `SELECT INTO variable`. Use `CREATE TABLE target AS SELECT ...` so the intent is explicit.",
    },
  },
  'no-set-not-null': {
    type: 'problem',
    description:
      'Disallow `ALTER COLUMN ... SET NOT NULL` because it scans the whole table under ACCESS EXCLUSIVE',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noSetNotNull:
        '`SET NOT NULL` scans the whole table for nulls under an `ACCESS EXCLUSIVE` lock. The safe pattern in production is to add a `CHECK (col IS NOT NULL) NOT VALID` constraint, `VALIDATE CONSTRAINT` separately, then `SET NOT NULL` (PG ≥ 12 reuses the validated CHECK and skips the scan).',
    },
  },
  'no-set-search-path': {
    type: 'suggestion',
    description:
      'Disallow `SET search_path = ...` in versioned SQL; qualify identifiers with their schema instead',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noSetSearchPath:
        '`SET search_path` makes name resolution depend on session state and is a known foot-gun for security-definer functions and CREATE statements. Qualify identifiers with their schema (`audit.events`, `public.users`) instead.',
    },
  },
  'no-temporary-table': {
    type: 'suggestion',
    description:
      'Disallow `CREATE TEMPORARY TABLE` in versioned SQL — temp tables exist for the session only and rarely belong in migration files',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noTemporaryTable:
        '`TEMPORARY` tables exist only for the current session, so they almost never belong in versioned SQL. If you need session-scoped scratch storage, build it from application code; if you mean a persistent table, drop the `TEMP/TEMPORARY` qualifier.',
    },
  },
  'no-truncate-cascade': {
    type: 'problem',
    description:
      'Disallow `TRUNCATE ... CASCADE` because it transitively empties referencing tables',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noCascade:
        '`TRUNCATE ... CASCADE` also truncates every table that has a foreign key referencing this one. List the dependent tables explicitly so reviewers can see what gets emptied.',
    },
  },
  'no-unlogged-table': {
    type: 'problem',
    description:
      'Disallow `CREATE UNLOGGED TABLE` because unlogged tables are truncated on crash and not replicated',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noUnloggedTable:
        '`UNLOGGED` tables skip WAL: they are truncated on crash, not replicated to standbys, and not restored from base backups. If a cache table is what you want, document it explicitly and disable this rule for that file.',
    },
  },
  'no-update-primary-key': {
    type: 'problem',
    description: 'Disallow `UPDATE ... SET <pk> = ...` for columns the rule treats as primary keys',
    recommended: false,
    fixable: undefined,
    schema: [
      {
        type: 'object',
        properties: {
          pkColumnNames: {
            type: 'array',
            items: { type: 'string' },
            uniqueItems: true,
          },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      noUpdatePk:
        'Avoid `UPDATE` on the primary-key column `{{name}}`. Primary keys are intended to be immutable; FK references and external systems can hold the old value.',
    },
  },
  'no-update-without-from-binding': {
    type: 'problem',
    description:
      'Disallow `UPDATE ... FROM` without a `WHERE` clause (Cartesian product with the target table)',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      missingJoin:
        '`UPDATE ... FROM` without a `WHERE` clause produces a Cartesian product with the target table; add a `WHERE t.x = other.x` condition to bind the rows.',
    },
  },
  'no-vacuum-full': {
    type: 'problem',
    description:
      'Disallow `VACUUM FULL` because it takes ACCESS EXCLUSIVE and rewrites the entire table',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      noVacuumFull:
        '`VACUUM FULL` takes `ACCESS EXCLUSIVE` and rewrites the whole table; the table is unavailable for the duration. For shrinking a bloated table on a live database, use `pg_repack` or `pg_squeeze`. A plain `VACUUM` (no `FULL`) is fine.',
    },
  },
  'no-volatile-default-on-add-column': {
    type: 'problem',
    description:
      'Disallow `ALTER TABLE ... ADD COLUMN ... DEFAULT <volatile>()`; volatile defaults force a full table rewrite under `ACCESS EXCLUSIVE` because the stable-default short-cut cannot be used',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      noVolatileDefault:
        '`{{fn}}()` is VOLATILE — using it as a column DEFAULT on `ADD COLUMN` forces PostgreSQL to rewrite the entire table under `ACCESS EXCLUSIVE`. Add the column without a default, then `UPDATE` rows in batches.',
    },
  },
  'no-with-recursive-without-limit': {
    type: 'problem',
    description:
      "Disallow `WITH RECURSIVE` queries that have no `LIMIT` on the outer SELECT, which can run unboundedly if the recursion's termination condition is wrong",
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      noLimit:
        'Add a `LIMIT` to a `WITH RECURSIVE` query so a buggy or accidentally-non-terminating recursion cannot run unboundedly.',
    },
  },
  'prefer-coalesce-over-case': {
    type: 'suggestion',
    description:
      'Prefer `COALESCE(x, y)` over `CASE WHEN x IS NULL THEN y ELSE x END` (and its IS NOT NULL mirror)',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      preferCoalesceOverCase:
        '`CASE WHEN ... IS NULL THEN ... ELSE ... END` is a verbose `COALESCE`. Use `COALESCE(x, fallback)` instead.',
    },
  },
  'prefer-current-timestamp-over-now': {
    type: 'layout',
    description:
      "Prefer SQL-standard `CURRENT_TIMESTAMP` / `CURRENT_TIME` over PostgreSQL's `now()` and the timezone-naive `LOCALTIMESTAMP` / `LOCALTIME`",
    recommended: false,
    fixable: 'code',
    schema: [],
    messages: {
      preferCurrentTimestamp: 'Use the SQL-standard `CURRENT_TIMESTAMP` instead of `now()`.',
      preferCurrentTimestampOverLocal:
        'Use `CURRENT_TIMESTAMP` instead of `LOCALTIMESTAMP`. `LOCALTIMESTAMP` returns a timezone-naive `timestamp`; `CURRENT_TIMESTAMP` returns `timestamptz`, which is what most apps actually want.',
      preferCurrentTimeOverLocal:
        'Use `CURRENT_TIME` instead of `LOCALTIME`. `LOCALTIME` returns a timezone-naive `time`; `CURRENT_TIME` returns `timetz`.',
    },
  },
  'prefer-exists-over-in-subquery': {
    type: 'suggestion',
    description: 'Prefer `EXISTS (...)` over `IN (subquery)` to avoid NULL-related surprises',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      preferExists:
        'Use `EXISTS (...)` instead of `IN (subquery)`. `IN` returns NULL when the subquery has any NULL row, which silently turns the row into a no-match; `EXISTS` is unambiguously boolean.',
    },
  },
  'prefer-explicit-null-ordering': {
    type: 'suggestion',
    description:
      'When `ORDER BY` specifies an explicit direction, require an explicit `NULLS FIRST` / `NULLS LAST` so null ordering is not implicit',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      preferExplicitNullOrdering:
        "`ORDER BY ... ASC|DESC` without `NULLS FIRST` / `NULLS LAST` relies on PostgreSQL's implicit ordering (NULLS LAST for ASC, NULLS FIRST for DESC), which trips up cross-database readers. Add an explicit `NULLS FIRST` / `NULLS LAST`.",
    },
  },
  'prefer-in-list-over-or': {
    type: 'suggestion',
    description: 'Prefer `x IN (a, b, c)` over a chain of `x = a OR x = b OR x = c`',
    recommended: false,
    fixable: 'code',
    schema: [],
    messages: {
      preferIn: 'Combine these `=` checks on `{{lhs}}` into a single `IN (...)` clause.',
    },
  },
  'prefer-not-equals-operator': {
    type: 'layout',
    description: 'Enforce a single style for the not-equal operator (`<>` or `!=`)',
    recommended: false,
    fixable: 'code',
    schema: [
      {
        type: 'object',
        properties: {
          operator: { enum: ['<>', '!='] },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      preferAngle: 'Use `<>` instead of `!=`.',
      preferBang: 'Use `!=` instead of `<>`.',
    },
  },
  'require-if-exists': {
    type: 'suggestion',
    description:
      'Require `IF EXISTS` on every `DROP` statement so re-running a migration on a database that already lost the object does not error',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      missingIfExists:
        'Add `IF EXISTS` to this `DROP` so re-running the migration on a database that already lost the object does not abort.',
    },
  },
  'require-limit': {
    type: 'suggestion',
    description: 'Require LIMIT clause in SELECT statements',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      missingLimit:
        'SELECT statement should include a LIMIT clause to prevent excessive data retrieval',
    },
  },
  'require-named-constraint': {
    type: 'suggestion',
    description:
      'Require an explicit `CONSTRAINT <name>` on table-level CHECK / UNIQUE / FOREIGN KEY / EXCLUSION constraints',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      requireNamedConstraint:
        'Table-level CHECK / UNIQUE / FOREIGN KEY / EXCLUSION constraints should be named with `CONSTRAINT <name>`. Auto-generated names are unpredictable across environments and make later `DROP CONSTRAINT` / `ALTER CONSTRAINT` migrations brittle.',
    },
  },
  'require-on-delete-action': {
    type: 'suggestion',
    description: 'Require an explicit `ON DELETE` clause on every foreign-key constraint',
    recommended: false,
    fixable: undefined,
    schema: [
      {
        type: 'object',
        properties: {
          allowed: {
            type: 'array',
            items: { enum: ['CASCADE', 'RESTRICT', 'NO ACTION', 'SET NULL', 'SET DEFAULT'] },
            uniqueItems: true,
          },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      missingOnDelete:
        'Foreign-key constraint is missing an explicit `ON DELETE` clause; the implicit default is `NO ACTION`. Make the choice explicit so reviewers can see what happens to dependent rows.',
      disallowedAction:
        "`ON DELETE {{action}}` is not in the `allowed` list ({{allowedList}}). Either change the action or extend the rule's `allowed` option.",
    },
  },
  'require-primary-key': {
    type: 'suggestion',
    description: 'Require every `CREATE TABLE` to have a PRIMARY KEY',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      missingPrimaryKey:
        'Table `{{table}}` has no PRIMARY KEY. Tables without one cannot be replicated cleanly, cannot be sharded predictably, and break almost every ORM. Add one as either a column constraint or a table-level constraint.',
    },
  },
  'require-schema-qualified-table': {
    type: 'suggestion',
    description:
      'Require `CREATE TABLE` to specify an explicit schema to avoid `search_path` dependence',
    recommended: false,
    fixable: undefined,
    schema: [],
    messages: {
      requireSchemaQualifiedTable:
        '`CREATE TABLE` should specify a schema (e.g. `audit.events`). Without one, the target depends on `search_path` and may land in an unintended schema. The rule is off by default in `recommended` because many projects intentionally use the `public` schema.',
    },
  },
  'require-trailing-semicolon': {
    type: 'layout',
    description: 'Require a trailing `;` at the end of the SQL file',
    recommended: false,
    fixable: 'code',
    schema: [],
    messages: {
      missingSemicolon: 'Missing trailing `;` at the end of the file.',
    },
  },
  'require-where-in-delete': {
    type: 'problem',
    description: 'Require a WHERE clause in DELETE statements',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      missingWhere:
        'DELETE without WHERE removes every row in the table. Add a WHERE clause, or use TRUNCATE if you really mean to empty the table.',
    },
  },
  'require-where-in-update': {
    type: 'problem',
    description: 'Require a WHERE clause in UPDATE statements',
    recommended: true,
    fixable: undefined,
    schema: [],
    messages: {
      missingWhere:
        'UPDATE without WHERE rewrites every row in the table. Add a WHERE clause to scope the change.',
    },
  },
  'snake-case-table-name': {
    type: 'suggestion',
    description: 'Require table names to be snake_case',
    recommended: true,
    fixable: undefined,
    schema: [
      {
        type: 'object',
        properties: {
          allow: {
            type: 'array',
            items: { type: 'string' },
            uniqueItems: true,
          },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      notSnakeCase:
        'Table name `{{name}}` is not snake_case. PostgreSQL folds unquoted identifiers to lower case but preserves the case of quoted identifiers; mixing the two leads to `relation "BadName" does not exist` errors that are confusing to debug.',
    },
  },
});

const implementedRuleNames = Object.freeze(implementedPostgresqlRuleNames());
const recommendedRuleNames = Object.freeze(
  implementedRuleNames.filter((name) => ruleMeta[name]?.recommended === true),
);
const recommendedRuleConfig = Object.freeze(
  Object.fromEntries(recommendedRuleNames.map((name) => [`${PLUGIN_NAME}/${name}`, 'error'])),
);
const allRuleConfig = Object.freeze(
  Object.fromEntries(implementedRuleNames.map((name) => [`${PLUGIN_NAME}/${name}`, 'error'])),
);

const rules = Object.freeze(
  Object.fromEntries(implementedRuleNames.map((name) => [name, createPostgresqlRule(name)])),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: 'eslint-plugin-postgresql',
    version: '0.22.1',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((name) => [name, 0])),
  configs: {
    recommended: configFromRuleConfig('recommended', recommendedRuleConfig),
    all: configFromRuleConfig('all', allRuleConfig),
  },
});

plugin.implementedPostgresqlRuleNames = implementedRuleNames;
plugin.scanPostgresql = scanPostgresql;

function configFromRuleConfig(name, ruleConfig) {
  return {
    name: `${PLUGIN_NAME}/${name}`,
    plugins: [PLUGIN_NAME],
    rules: { ...ruleConfig },
  };
}

function createPostgresqlRule(ruleName) {
  const meta = ruleMeta[ruleName] ?? {};
  return {
    meta: {
      type: meta.type ?? 'suggestion',
      docs: {
        description: meta.description,
        recommended: meta.recommended === true,
        url: `${DOCS_BASE}#${ruleName}`,
      },
      fixable: meta.fixable,
      messages: meta.messages ?? {},
      schema: meta.schema ?? [],
    },
    createOnce(context) {
      return {
        Program() {
          for (const diagnostic of diagnosticsForRule(context, ruleName)) {
            reportDiagnostic(context, diagnostic);
          }
        },
      };
    },
  };
}

function diagnosticsForRule(context, ruleName) {
  const sourceCode = context.sourceCode ?? context.getSourceCode();
  let bySource = diagnosticsCache.get(sourceCode);
  if (!bySource) {
    bySource = new Map();
    diagnosticsCache.set(sourceCode, bySource);
  }

  const options = context.options ?? [];
  const cacheKey = `${ruleName}\0${JSON.stringify(options)}`;
  let diagnostics = bySource.get(cacheKey);
  if (!diagnostics) {
    diagnostics = scanPostgresql(sourceCode.text, { ruleNames: [ruleName], options }).filter(
      (diagnostic) => diagnostic.ruleName === ruleName,
    );
    bySource.set(cacheKey, diagnostics);
  }

  return diagnostics;
}

function dataForReport(data) {
  const out = {};
  for (const datum of data ?? []) {
    out[datum.key] = datum.value;
  }
  return out;
}

function reportDiagnostic(context, diagnostic) {
  const report = {
    messageId: diagnostic.messageId,
    data: dataForReport(diagnostic.data),
    loc: {
      start: {
        line: diagnostic.loc.startLine,
        column: diagnostic.loc.startColumn,
      },
      end: {
        line: diagnostic.loc.endLine,
        column: diagnostic.loc.endColumn,
      },
    },
  };

  if (diagnostic.fix) {
    report.fix = (fixer) =>
      fixer.replaceTextRange(
        [diagnostic.fix.start, diagnostic.fix.end],
        diagnostic.fix.replacement,
      );
  }

  context.report(report);
}

module.exports = plugin;
module.exports.default = plugin;
