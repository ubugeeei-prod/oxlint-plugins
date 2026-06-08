import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

function runRule(sourceText, identifiers, options) {
  const reports = [];
  const rule = plugin.rules['no-forbidden-identifiers'];
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
    return reports;
  }

  for (const name of identifiers) {
    visitor.Identifier({ type: 'Identifier', name });
  }

  visitor.after?.();
  return reports;
}

describe('no-forbidden-identifiers', () => {
  it('reports default names with a Rust pre-scan', () => {
    expect(runRule('const event = data.error;', ['event', 'data', 'error'])).toMatchInlineSnapshot(`
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
    expect(runRule('const value = input;', ['value', 'input'])).toEqual([]);
  });

  it('supports custom names', () => {
    expect(runRule('const ctx = event;', ['ctx', 'event'], { names: ['ctx'] }))
      .toMatchInlineSnapshot(`
      [
        {
          "data": {
            "name": "ctx",
          },
          "messageId": "forbiddenIdentifier",
          "node": {
            "name": "ctx",
            "type": "Identifier",
          },
        },
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
      ]
    `);
  });
});
