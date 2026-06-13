import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const RULE_NAME = 'no-forbidden-identifiers';

function runRule(sourceText, identifiers, options) {
  const reports = [];
  const rule = plugin.rules[RULE_NAME];
  const visitor = rule.createOnce({
    options: options ? [options] : [],
    sourceCode: { text: sourceText },
    report(descriptor) {
      reports.push({
        messageId: descriptor.messageId,
        data: descriptor.data,
        node: {
          type: descriptor.node.type,
          name: descriptor.node.name,
        },
      });
    },
  });

  if (visitor.before?.() === false) {
    return { reports, skipped: true, visitor };
  }

  for (const name of identifiers) {
    visitor.Identifier({ type: 'Identifier', name });
  }

  visitor.after?.();
  return { reports, skipped: false, visitor };
}

describe('no-forbidden-identifiers rule', () => {
  it('reports default names with a Rust pre-scan', () => {
    expect(runRule('const event = data.error;', ['event', 'data', 'error']).reports)
      .toMatchInlineSnapshot(`
      [
        {
          "data": {
            "name": "event",
          },
          "messageId": "forbiddenIdentifier",
          "node": {
            "name": "event",
            "type": "Identifier",
          },
        },
        {
          "data": {
            "name": "data",
          },
          "messageId": "forbiddenIdentifier",
          "node": {
            "name": "data",
            "type": "Identifier",
          },
        },
        {
          "data": {
            "name": "error",
          },
          "messageId": "forbiddenIdentifier",
          "node": {
            "name": "error",
            "type": "Identifier",
          },
        },
      ]
    `);
  });

  it('skips visitor work when the Rust pre-scan finds nothing', () => {
    const { reports, skipped } = runRule('const value = input;', ['value', 'input']);
    expect(skipped).toBe(true);
    expect(reports).toEqual([]);
  });

  it('skips visitor work when the source text is empty', () => {
    const { reports, skipped } = runRule('', ['event']);
    expect(skipped).toBe(true);
    expect(reports).toEqual([]);
  });

  it('skips visitor work when the source text is missing entirely', () => {
    const reports = [];
    const rule = plugin.rules[RULE_NAME];
    const visitor = rule.createOnce({
      options: [],
      sourceCode: {},
      report: (d) => reports.push(d),
    });
    expect(visitor.before?.()).toBe(false);
  });

  it('skips visitor work when sourceCode is missing entirely', () => {
    const reports = [];
    const rule = plugin.rules[RULE_NAME];
    const visitor = rule.createOnce({
      options: [],
      report: (d) => reports.push(d),
    });
    expect(visitor.before?.()).toBe(false);
  });

  it('supports custom names alongside defaults', () => {
    const { reports } = runRule('const ctx = event;', ['ctx', 'event'], { names: ['ctx'] });
    expect(reports.map((report) => report.data.name)).toEqual(['ctx', 'event']);
  });

  it('only reports identifiers visited by the AST, not every occurrence in source', () => {
    // The pre-scan finds `event`, but the visitor only sees `error` and
    // therefore should only emit a report for `error`.
    const { reports } = runRule('const event = error;', ['error']);
    expect(reports.map((report) => report.data.name)).toEqual(['error']);
  });

  it('does not report identifiers that were not flagged by the pre-scan', () => {
    const { reports } = runRule('const event = 1;', ['event', 'value', 'foo']);
    expect(reports.map((report) => report.data.name)).toEqual(['event']);
  });

  it('does not crash when the visitor receives a non-Identifier-shaped node', () => {
    const reports = [];
    const rule = plugin.rules[RULE_NAME];
    const visitor = rule.createOnce({
      options: [],
      sourceCode: { text: 'const event = 1;' },
      report: (d) => reports.push(d),
    });
    visitor.before?.();
    // Missing `name` should be safely ignored.
    visitor.Identifier({ type: 'Identifier' });
    visitor.Identifier({ type: 'Identifier', name: 123 });
    visitor.Identifier({ type: 'Identifier', name: null });
    visitor.after?.();
    expect(reports).toEqual([]);
  });

  it('clears the active name set in after() so a stale set never carries over', () => {
    const rule = plugin.rules[RULE_NAME];
    const reports = [];
    const visitor = rule.createOnce({
      options: [],
      sourceCode: { text: 'const event = 1;' },
      report: (d) => reports.push(d),
    });

    visitor.before?.();
    visitor.Identifier({ type: 'Identifier', name: 'event' });
    visitor.after?.();

    // After cleanup, calling Identifier again must not emit reports.
    visitor.Identifier({ type: 'Identifier', name: 'event' });
    expect(reports.map((d) => d.data.name)).toEqual(['event']);
  });

  it('treats options=[] (no rule options) as no custom names', () => {
    const rule = plugin.rules[RULE_NAME];
    const reports = [];
    const visitor = rule.createOnce({
      options: [],
      sourceCode: { text: 'const ctx = 1;' },
      report: (d) => reports.push(d),
    });
    expect(visitor.before?.()).toBe(false);
    expect(reports).toEqual([]);
  });

  it('reuses the rule across consecutive files without state leaking', () => {
    const rule = plugin.rules[RULE_NAME];
    const reports = [];
    const context = {
      options: [],
      sourceCode: { text: 'const event = 1;' },
      report: (d) => reports.push(d),
    };

    rule.createOnce(context).before?.();
    rule.createOnce(context).Identifier({ type: 'Identifier', name: 'event' });
    // The second createOnce instance has not run its own `before()`, so the
    // visitor must not report on stale state from the first instance.
    expect(reports).toEqual([]);
  });

  it('emits reports with the expected messageId and data payload shape', () => {
    const { reports } = runRule('const event = 1;', ['event']);
    expect(reports[0].messageId).toBe('forbiddenIdentifier');
    expect(reports[0].data).toEqual({ name: 'event' });
  });

  it('respects empty custom names array (falls back to defaults only)', () => {
    const { reports } = runRule('const event = 1;', ['event'], { names: [] });
    expect(reports.map((d) => d.data.name)).toEqual(['event']);
  });
});

describe('no-forbidden-identifiers plugin meta', () => {
  const rule = plugin.rules[RULE_NAME];

  it('declares problem type and is not on by default', () => {
    expect(rule.meta.type).toBe('problem');
    expect(rule.meta.docs.recommended).toBe(false);
  });

  it('exposes a documentation URL pointing at the repository', () => {
    expect(rule.meta.docs.url).toMatch(/github\.com\/ubugeeei-prod\/oxlint-plugins/);
  });

  it('declares a forbiddenIdentifier message with a {{name}} interpolation', () => {
    expect(rule.meta.messages.forbiddenIdentifier).toContain('{{name}}');
  });

  it('declares a JSON schema that rejects unknown properties', () => {
    const schema = rule.meta.schema;
    expect(Array.isArray(schema)).toBe(true);
    expect(schema[0].type).toBe('object');
    expect(schema[0].additionalProperties).toBe(false);
    expect(schema[0].properties.names.type).toBe('array');
    expect(schema[0].properties.names.items.type).toBe('string');
  });
});

describe('no-forbidden-identifiers plugin shape', () => {
  it('exposes the no-forbidden-identifiers rule', () => {
    expect(plugin.rules).toBeDefined();
    expect(plugin.rules[RULE_NAME]).toBeDefined();
  });

  it('exposes a recommended config that enables the rule as error', () => {
    expect(plugin.configs.recommended.plugins).toContain('no-forbidden-identifiers');
    expect(
      plugin.configs.recommended.rules['no-forbidden-identifiers/no-forbidden-identifiers'],
    ).toBe('error');
  });

  it('exports both default and CommonJS surfaces equivalently', () => {
    expect(plugin.default).toBe(plugin);
  });
});
