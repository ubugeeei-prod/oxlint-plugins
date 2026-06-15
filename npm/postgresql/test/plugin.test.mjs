import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

function runRule(ruleName, code, options) {
  const reports = [];
  const context = {
    filename: 'fixture.sql',
    options: options === undefined ? [] : [options],
    sourceCode: {
      getText: () => code,
    },
    report(descriptor) {
      reports.push(descriptor);
    },
  };

  plugin.rules[ruleName].createOnce(context).Program();
  return reports;
}

describe('postgresql plugin shape', () => {
  it('exposes implemented rules and configs', () => {
    expect(plugin.meta?.name).toBe('postgresql');
    expect(Object.keys(plugin.rules)).toContain('no-drop-database');
    expect(Object.keys(plugin.rules)).toContain('require-where-in-update');
    expect(plugin.configs.recommended.files).toEqual(['**/*.sql']);
    expect(plugin.configs.recommended.rules['postgresql/no-drop-database']).toBe('warn');
    expect(plugin.configs.all.rules['postgresql/no-drop-database']).toBe('error');
  });
});

describe('postgresql rule adapter', () => {
  it('reports a single rule from the shared native scan', () => {
    expect(runRule('no-drop-database', 'DROP DATABASE archive;')).toMatchObject([
      {
        messageId: 'noDropDatabase',
        loc: {
          start: { line: 1, column: 0 },
          end: { line: 1, column: 21 },
        },
      },
    ]);
  });

  it('passes rule options to native code', () => {
    expect(
      runRule('consistent-text-over-varchar', 'CREATE TABLE t (name text);', {
        style: 'never',
      }),
    ).toMatchObject([{ messageId: 'unexpectedText' }]);

    expect(
      runRule('prefer-not-equals-operator', 'SELECT a <> b;', {
        operator: '!=',
      }),
    ).toMatchObject([{ messageId: 'preferBang' }]);
  });

  it('renders upstream-style message data', () => {
    expect(
      runRule('no-equality-with-null', 'SELECT id FROM users WHERE name = NULL;'),
    ).toMatchObject([
      {
        messageId: 'useIsNull',
        data: { op: '=' },
      },
    ]);
  });
});
