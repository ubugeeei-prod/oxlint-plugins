// `no-unused-disable` needs the file's lint problems, which exist only at
// runtime via `sourceCode.getDisableDirectives()`. The espree replay harness
// cannot provide those, so this rule's mechanism is exercised here with a mock
// `sourceCode` that supplies synthetic problems.

import * as espree from 'espree';
import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const rule = plugin.rules['no-unused-disable'];

function run(code, problems) {
  const ast = espree.parse(code, { comment: true, loc: true, range: true, ecmaVersion: 'latest' });
  const sourceCode = {
    text: code,
    ast,
    getAllComments: () => ast.comments ?? [],
    getDisableDirectives: () => ({ problems, directives: [] }),
  };

  const reports = [];
  const context = { options: [], sourceCode, report: (d) => reports.push(d) };
  const visitor = rule.createOnce(context);
  visitor.Program?.(ast);

  return reports.map((d) => ({
    messageId: d.messageId,
    ruleId: d.data?.ruleId,
    line: d.loc.start.line,
    column: d.loc.start.column + 1,
  }));
}

function problem(ruleId, line, column) {
  return { ruleId, loc: { start: { line, column }, end: { line, column } } };
}

describe('no-unused-disable (approximation)', () => {
  it('is marked deprecated', () => {
    expect(rule.meta.deprecated).toBe(true);
  });

  it('reports a disable that suppressed no matching problem', () => {
    expect(run('/* eslint-disable no-undef */\nvar a = 1\n', [])).toEqual([
      { messageId: 'unusedRule', ruleId: 'no-undef', line: 1, column: 19 },
    ]);
  });

  it('does not report a disable that suppressed a matching problem', () => {
    expect(run('/* eslint-disable no-undef */\nb()\n', [problem('no-undef', 2, 0)])).toEqual([]);
  });

  it('reports a bare disable only when nothing was reported', () => {
    expect(run('/* eslint-disable */\nvar a = 1\n', [])).toEqual([
      { messageId: 'unused', ruleId: undefined, line: 1, column: 0 },
    ]);
    expect(run('/* eslint-disable */\nb()\n', [problem('no-undef', 2, 0)])).toEqual([]);
  });
});
