import { describe, expect, it } from 'vitest';

import { implementedPostgresqlRuleNames, scanPostgresql } from '../api.js';

describe('postgresql native api', () => {
  it('exposes the implemented rule set', () => {
    expect(implementedPostgresqlRuleNames()).toEqual([
      'consistent-identity-over-serial',
      'consistent-jsonb-over-json',
      'consistent-text-over-varchar',
      'consistent-timestamptz',
      'no-char-type',
      'no-cluster',
      'no-create-role',
      'no-cross-join',
      'no-drop-database',
      'no-drop-schema-cascade',
      'no-drop-table-cascade',
      'no-equality-with-null',
      'no-grant-all',
      'no-grant-to-public',
      'no-money-type',
      'no-natural-join',
      'no-not-in-subquery',
      'no-select-into',
      'no-select-star',
      'no-set-search-path',
      'no-temporary-table',
      'no-time-type',
      'no-truncate-cascade',
      'no-unlogged-table',
      'no-vacuum-full',
      'prefer-cast-operator',
      'prefer-current-timestamp-over-now',
      'prefer-not-equals-operator',
      'require-trailing-semicolon',
      'require-where-in-delete',
      'require-where-in-update',
    ]);
  });

  it('runs high-risk SQL rules in native code', () => {
    const diagnostics = scanPostgresql(
      [
        'DROP DATABASE archive_2023;',
        'SELECT * FROM users;',
        'UPDATE users SET active = false;',
        'DELETE FROM sessions;',
        'VACUUM FULL users;',
        'GRANT ALL ON t TO u;',
      ].join('\n'),
      'migration.sql',
    );

    expect(diagnostics.map((diagnostic) => diagnostic.messageId)).toEqual([
      'noDropDatabase',
      'noSelectStar',
      'missingWhere',
      'missingWhere',
      'noVacuumFull',
      'noGrantAll',
    ]);
  });

  it('keeps comments and strings out of the token stream', () => {
    expect(
      scanPostgresql(
        [
          '-- DROP DATABASE archive;',
          "SELECT 'VACUUM FULL users';",
          'SELECT count(*) FROM users;',
        ].join('\n'),
        'safe.sql',
      ),
    ).toEqual([]);
  });

  it('supports upstream style options', () => {
    expect(
      scanPostgresql('CREATE TABLE t (payload jsonb, name text);', 'schema.sql', {
        jsonbStyle: 'never',
        textStyle: 'never',
      }).map((diagnostic) => diagnostic.messageId),
    ).toEqual(['unexpectedJsonb', 'unexpectedText']);

    expect(
      scanPostgresql('SELECT a <> b, c::int;', 'query.sql', {
        notEqualsOperator: '!=',
        castForm: 'function',
      }).map((diagnostic) => diagnostic.messageId),
    ).toEqual(['preferBang', 'preferFunction']);
  });

  it('returns LSP-shaped source locations and message data', () => {
    const [diagnostic] = scanPostgresql('SELECT id FROM users WHERE name != NULL;', 'query.sql');

    expect(diagnostic).toMatchObject({
      ruleName: 'no-equality-with-null',
      messageId: 'useIsNull',
      data: { op: '!=' },
      loc: {
        startLine: 1,
        startColumn: 32,
        endLine: 1,
        endColumn: 34,
      },
    });
  });
});
