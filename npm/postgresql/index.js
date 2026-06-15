'use strict';

// Oxlint plugin port of eslint-plugin-postgresql (MIT).
// SQL tokenization and implemented rule checks run in Rust through NAPI.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedPostgresqlRuleNames, scanPostgresql } = require('./api.js');

const PLUGIN_NAME = 'postgresql';
const DOCS_BASE = 'https://github.com/baseballyama/eslint-plugin-postgresql/blob/main/docs/rules';
const diagnosticsCache = new WeakMap();

const messages = {
  'consistent-identity-over-serial': {
    preferIdentity:
      'Use `GENERATED ALWAYS AS IDENTITY` (SQL standard) instead of `{{typeName}}`. The serial pseudo-types create a separately-owned sequence that breaks under pg_dump round-trips and does not honor column privileges.',
    unexpectedIdentity:
      'Use a serial pseudo-type (e.g. `bigserial`) instead of `GENERATED ... AS IDENTITY`. Useful for projects that need to keep compatibility with tooling that does not understand identity columns.',
  },
  'consistent-jsonb-over-json': {
    preferJsonb:
      'Use `jsonb` instead of `json`. `jsonb` stores the parsed representation, supports indexing, and is what almost every application actually wants.',
    unexpectedJsonb:
      "Use `json` instead of `jsonb`. Useful when the project intentionally relies on `json`'s preservation of key order, whitespace, and duplicate keys.",
  },
  'consistent-text-over-varchar': {
    preferText:
      'Use `text` instead of `varchar(n)`. PostgreSQL stores both the same way; the length cap is enforced by a constraint that you cannot relax without a full table rewrite. Move the limit into a CHECK constraint.',
    unexpectedText:
      "Use `varchar(n)` (or another bounded string type) instead of `text`. Useful for projects that intentionally cap every string column's length at the type level.",
  },
  'consistent-timestamptz': {
    preferTimestamptz:
      'Use `timestamptz` (or `TIMESTAMP WITH TIME ZONE`) instead of `timestamp`. `timestamp` is timezone-naive: it stores the wall-clock value you handed in and assumes every reader and writer share the same convention, so two clients on different `TimeZone` settings will disagree on which instant the row represents.',
    unexpectedTimestamptz:
      "Use `timestamp` instead of `timestamptz` (or `TIMESTAMP WITH TIME ZONE`). When the project treats every timestamp as UTC at the application layer, `timestamp` avoids the implicit conversions `timestamptz` performs against each session's `TimeZone` setting.",
  },
  'no-char-type': {
    noChar:
      'Avoid `char(n)`. PostgreSQL pads stored values to `n` with trailing spaces and trims on read, which surprises every comparison and round-trip. Use `text` instead.',
  },
  'no-cluster': {
    noCluster:
      '`CLUSTER` takes `ACCESS EXCLUSIVE` and rewrites the entire table, just like `VACUUM FULL` - and PostgreSQL does not keep the rows clustered as you continue to write. Use `pg_repack --order-by` for online clustering, or build an index in the order you actually want to read.',
  },
  'no-create-role': {
    noCreateRole:
      '`CREATE ROLE` / `CREATE USER` belongs in an operator-managed bootstrap (Terraform, Pulumi, a runbook), not in application migrations. Migration files run with whichever role the deploy uses and are not the right place to manage permissions.',
  },
  'no-cross-join': {
    noCrossJoin:
      'Avoid `CROSS JOIN`. Cartesian products are almost always a mistake; use an explicit `JOIN ... ON` with a join condition, or `JOIN ... ON true` if you really do want one.',
  },
  'no-drop-database': {
    noDropDatabase:
      '`DROP DATABASE` is catastrophic and irreversible. Database creation/deletion belongs in an explicit operator workflow, not in versioned SQL applied automatically by a migration tool.',
  },
  'no-drop-schema-cascade': {
    noDropSchemaCascade:
      "`DROP SCHEMA ... CASCADE` removes every table, view, function, and sequence in the schema with no preview. List the objects you actually want to drop instead, or drop the schema only when it's already empty.",
  },
  'no-drop-table-cascade': {
    noCascade:
      'Avoid `DROP TABLE ... CASCADE`. CASCADE silently removes dependent objects (views, foreign keys, sequences); list them explicitly so reviewers can see the blast radius.',
  },
  'no-equality-with-null': {
    useIsNull:
      '`{{op}} NULL` always evaluates to NULL (treated as false). Use `IS NULL` / `IS NOT NULL` instead.',
  },
  'no-grant-all': {
    noGrantAll:
      '`GRANT ALL` (or `GRANT ALL PRIVILEGES`) is opaque - list the privileges you actually need (e.g. `SELECT, INSERT, UPDATE`) so the grant is auditable and adding a new PostgreSQL privilege in a future release does not silently extend it.',
  },
  'no-grant-to-public': {
    noPublic:
      'Avoid `GRANT ... TO PUBLIC`. The PUBLIC role covers every current and future role in the database, including ones added later for unrelated services. Name the role(s) you actually want to grant to.',
  },
  'no-money-type': {
    noMoney:
      'Avoid `money`. Its output format and precision depend on `lc_monetary`, so the same row looks different on different servers and round-trips badly. Store amounts as `numeric` and keep the currency in a separate column.',
  },
  'no-natural-join': {
    noNaturalJoin:
      'Avoid `NATURAL JOIN`. The join columns are implicit - any future column with a matching name on both sides silently changes the result. Use `JOIN ... USING (...)` or `JOIN ... ON ...` and name the columns.',
  },
  'no-not-in-subquery': {
    noNotInSubquery:
      '`NOT IN (subquery)` returns no rows if the subquery yields any NULL - almost certainly not what you want. Use `NOT EXISTS (SELECT 1 FROM ... WHERE ...)` instead; it handles NULL correctly.',
  },
  'no-select-into': {
    noSelectInto:
      "`SELECT ... INTO target FROM ...` creates a new table whose semantics differ from a regular `SELECT` and conflict with PL/pgSQL's `SELECT INTO variable`. Use `CREATE TABLE target AS SELECT ...` so the intent is explicit.",
  },
  'no-select-star': {
    noSelectStar:
      'Avoid `SELECT *`; list the columns you need so the result schema does not silently change when the table does.',
  },
  'no-set-search-path': {
    noSetSearchPath:
      '`SET search_path` makes name resolution depend on session state and is a known foot-gun for security-definer functions and CREATE statements. Qualify identifiers with their schema (`audit.events`, `public.users`) instead.',
  },
  'no-temporary-table': {
    noTemporaryTable:
      '`TEMPORARY` tables exist only for the current session, so they almost never belong in versioned SQL. If you need session-scoped scratch storage, build it from application code; if you mean a persistent table, drop the `TEMP/TEMPORARY` qualifier.',
  },
  'no-time-type': {
    noTimeType:
      '`TIME` and `TIME WITH TIME ZONE` rarely model anything correctly: `time` has no date so cannot disambiguate around DST, and `timetz` stores an offset that is meaningless without a date. Use `timestamptz` for points in time, `interval` for durations, or store an opaque `text` if all you need is a display value.',
  },
  'no-truncate-cascade': {
    noCascade:
      '`TRUNCATE ... CASCADE` also truncates every table that has a foreign key referencing this one. List the dependent tables explicitly so reviewers can see what gets emptied.',
  },
  'no-unlogged-table': {
    noUnloggedTable:
      '`UNLOGGED` tables skip WAL: they are truncated on crash, not replicated to standbys, and not restored from base backups. If a cache table is what you want, document it explicitly and disable this rule for that file.',
  },
  'no-vacuum-full': {
    noVacuumFull:
      '`VACUUM FULL` takes `ACCESS EXCLUSIVE` and rewrites the whole table; the table is unavailable for the duration. For shrinking a bloated table on a live database, use `pg_repack` or `pg_squeeze`. A plain `VACUUM` (no `FULL`) is fine.',
  },
  'prefer-cast-operator': {
    preferOperator: 'Use `<expr>::type` operator form instead of `CAST(...)`.',
    preferFunction: 'Use `CAST(<expr> AS type)` instead of the `::` operator.',
  },
  'prefer-current-timestamp-over-now': {
    preferCurrentTimestamp: 'Use the SQL-standard `CURRENT_TIMESTAMP` instead of `now()`.',
    preferCurrentTimestampOverLocal:
      'Use `CURRENT_TIMESTAMP` instead of `LOCALTIMESTAMP`. `LOCALTIMESTAMP` returns a timezone-naive `timestamp`; `CURRENT_TIMESTAMP` returns `timestamptz`, which is what most apps actually want.',
    preferCurrentTimeOverLocal:
      'Use `CURRENT_TIME` instead of `LOCALTIME`. `LOCALTIME` returns a timezone-naive `time`; `CURRENT_TIME` returns `timetz`.',
  },
  'prefer-not-equals-operator': {
    preferAngle: 'Use `<>` instead of `!=`.',
    preferBang: 'Use `!=` instead of `<>`.',
  },
  'require-trailing-semicolon': {
    missingSemicolon: 'Missing trailing `;` at the end of the file.',
  },
  'require-where-in-delete': {
    missingWhere:
      'DELETE without WHERE removes every row in the table. Add a WHERE clause, or use TRUNCATE if you really mean to empty the table.',
  },
  'require-where-in-update': {
    missingWhere:
      'UPDATE without WHERE rewrites every row in the table. Add a WHERE clause to scope the change.',
  },
};

const descriptions = Object.freeze(
  Object.fromEntries(
    Object.keys(messages).map((ruleName) => [
      ruleName,
      `port of eslint-plugin-postgresql/${ruleName}`,
    ]),
  ),
);

const implementedRuleNames = Object.freeze(implementedPostgresqlRuleNames());
const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createPostgresqlRule(ruleName)]),
  ),
);

const recommendedRules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'warn']),
  ),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  configs: {
    recommended: {
      name: `${PLUGIN_NAME}/recommended`,
      files: ['**/*.sql'],
      plugins: [PLUGIN_NAME],
      rules: recommendedRules,
    },
    all: {
      name: `${PLUGIN_NAME}/all`,
      files: ['**/*.sql'],
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
      ),
    },
  },
});

plugin.implementedPostgresqlRuleNames = implementedRuleNames;
plugin.scanPostgresql = scanPostgresql;

function createPostgresqlRule(ruleName) {
  return {
    meta: {
      type: ruleName.startsWith('prefer-') ? 'layout' : 'suggestion',
      docs: {
        description: descriptions[ruleName],
        recommended: true,
        url: `${DOCS_BASE}/${ruleName}.md`,
      },
      messages: messages[ruleName],
      schema: optionSchemaFor(ruleName),
    },
    createOnce(context) {
      return {
        Program() {
          for (const diagnostic of diagnosticsForContext(context, ruleName)) {
            if (diagnostic.ruleName !== ruleName) {
              continue;
            }
            context.report({
              messageId: diagnostic.messageId,
              data: compactData(diagnostic.data),
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
            });
          }
        },
      };
    },
  };
}

function diagnosticsForContext(context, ruleName) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'schema.sql';
  const options = nativeOptionsForRule(ruleName, context.options?.[0]);
  const cacheKey = JSON.stringify(options);
  const cached = diagnosticsCache.get(sourceCode);

  if (
    cached &&
    cached.sourceText === sourceText &&
    cached.filename === filename &&
    cached.cacheKey === cacheKey
  ) {
    return cached.diagnostics;
  }

  const diagnostics = scanPostgresql(sourceText, filename, options);
  diagnosticsCache.set(sourceCode, { sourceText, filename, cacheKey, diagnostics });
  return diagnostics;
}

function nativeOptionsForRule(ruleName, raw) {
  const options = raw && typeof raw === 'object' ? raw : {};
  switch (ruleName) {
    case 'consistent-identity-over-serial':
      return { identityStyle: styleOption(options.style) };
    case 'consistent-jsonb-over-json':
      return { jsonbStyle: styleOption(options.style) };
    case 'consistent-text-over-varchar':
      return { textStyle: styleOption(options.style) };
    case 'consistent-timestamptz':
      return { timestamptzStyle: styleOption(options.style) };
    case 'prefer-not-equals-operator':
      return {
        notEqualsOperator: options.operator === '!=' ? '!=' : '<>',
      };
    case 'prefer-cast-operator':
      return {
        castForm: options.form === 'function' ? 'function' : 'operator',
      };
    default:
      return {};
  }
}

function styleOption(value) {
  return value === 'never' ? 'never' : 'always';
}

function optionSchemaFor(ruleName) {
  if (
    [
      'consistent-identity-over-serial',
      'consistent-jsonb-over-json',
      'consistent-text-over-varchar',
      'consistent-timestamptz',
    ].includes(ruleName)
  ) {
    return [
      {
        type: 'object',
        properties: {
          style: { enum: ['always', 'never'] },
        },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'prefer-not-equals-operator') {
    return [
      {
        type: 'object',
        properties: {
          operator: { enum: ['<>', '!='] },
        },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'prefer-cast-operator') {
    return [
      {
        type: 'object',
        properties: {
          form: { enum: ['operator', 'function'] },
        },
        additionalProperties: false,
      },
    ];
  }
  return [];
}

function sourceTextForContext(context) {
  const sourceCode = context.sourceCode || {};
  if (typeof sourceCode.getText === 'function') {
    return sourceCode.getText();
  }
  if (typeof sourceCode.text === 'string') {
    return sourceCode.text;
  }
  return '';
}

function compactData(data) {
  const out = {};
  for (const [key, value] of Object.entries(data || {})) {
    if (value != null) {
      out[key] = value;
    }
  }
  return out;
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedPostgresqlRuleNames = implementedRuleNames;
module.exports.scanPostgresql = scanPostgresql;
