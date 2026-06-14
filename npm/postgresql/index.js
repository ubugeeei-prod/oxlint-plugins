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
