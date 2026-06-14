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
