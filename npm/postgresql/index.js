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
