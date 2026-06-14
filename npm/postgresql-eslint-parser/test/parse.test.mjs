// Ported from upstream src/parse.test.ts — source-location accuracy across
// multi-byte / surrogate-pair comments, and token classification.

import { describe, expect, it } from 'vitest';

import { parseForESLint } from '../api.js';

const findFirst = (body, type) => {
  const visit = (node) => {
    if (Array.isArray(node)) {
      for (const child of node) {
        const found = visit(child);
        if (found) return found;
      }
      return null;
    }
    if (node && typeof node === 'object') {
      if (node.type === type) return node;
      for (const [key, value] of Object.entries(node)) {
        if (key === 'parent' || key === 'loc' || key === 'range') continue;
        const found = visit(value);
        if (found) return found;
      }
    }
    return null;
  };
  return visit(body);
};

const requireFirst = (body, type) => {
  const node = findFirst(body, type);
  if (!node) throw new Error(`Could not find a ${type} node`);
  return node;
};

describe('parseForESLint - source location accuracy', () => {
  it('reports correct line/column when multi-byte comments precede a statement', () => {
    const code = '-- 日本語\nSELECT 1';
    const { ast } = parseForESLint(code);

    const constNode = requireFirst(ast.body, 'A_Const');
    expect(constNode.loc).toEqual({
      start: { line: 2, column: 7 },
      end: { line: 2, column: 8 },
    });
    expect(constNode.range).toEqual([14, 15]);
  });

  it('does not push child loc onto wrong lines after multi-line, multi-byte comments', () => {
    const code = [
      '-- これはテーブル定義です',
      '-- 列の説明: ID は主キーです',
      'SELECT id FROM users WHERE id = 1',
    ].join('\n');
    const { ast } = parseForESLint(code);

    const constNode = requireFirst(ast.body, 'A_Const');
    expect(constNode.loc.start.line).toBe(3);
  });

  it('handles surrogate pairs (emoji) before a statement', () => {
    const code = '-- 😀\nSELECT 1';
    const { ast } = parseForESLint(code);
    const constNode = requireFirst(ast.body, 'A_Const');
    expect(constNode.loc).toEqual({
      start: { line: 2, column: 7 },
      end: { line: 2, column: 8 },
    });
  });
});

describe('parseForESLint - token classification', () => {
  const tokenValuesByType = (code) => {
    const { ast } = parseForESLint(code);
    const grouped = {};
    for (const token of ast.tokens ?? []) {
      (grouped[token.type] ??= []).push(token.value);
    }
    return grouped;
  };

  it('classifies well-formed integers, decimals, and exponents as Numeric', () => {
    const tokens = tokenValuesByType('SELECT 1, 1.5, 2e10, 3.5E-2');
    expect(tokens.Numeric).toEqual(['1', '1.5', '2e10', '3.5E-2']);
  });

  it('falls back to Identifier for malformed numeric-looking lexemes', () => {
    const tokens = tokenValuesByType('1..2 1e');
    expect(tokens.Numeric ?? []).toEqual([]);
    expect(tokens.Identifier).toEqual(['1..2', '1e']);
  });
});
