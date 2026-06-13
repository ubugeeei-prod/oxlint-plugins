// Unit tests for the LSP fixture harness primitives. The harness powers the
// scenario test in `diagnostics.test.mjs`; these tests pin its individual
// helpers so subtle regressions in offset/position math surface quickly.

import { describe, expect, it } from 'vitest';

import {
  applyTextEdits,
  createTextDocument,
  diagnosticForIdentifier,
  quickFixReplaceIdentifier,
} from './harness.mjs';

describe('createTextDocument', () => {
  it('records the URI and text verbatim', () => {
    const doc = createTextDocument('file:///a.ts', 'const x = 1;\n');
    expect(doc).toEqual({ uri: 'file:///a.ts', text: 'const x = 1;\n' });
  });
});

describe('diagnosticForIdentifier', () => {
  it('builds a diagnostic with a range matching the first occurrence', () => {
    const doc = createTextDocument('file:///a.ts', 'const event = 1;\n');
    const diagnostic = diagnosticForIdentifier(
      doc,
      'no-forbidden-identifiers/no-forbidden-identifiers',
      'event',
      "Identifier 'event' is reserved by this project policy.",
    );
    expect(diagnostic).toEqual({
      range: {
        start: { line: 0, character: 6 },
        end: { line: 0, character: 11 },
      },
      severity: 1,
      source: 'oxlint-plugins',
      code: 'no-forbidden-identifiers/no-forbidden-identifiers',
      message: "Identifier 'event' is reserved by this project policy.",
      data: {
        ruleId: 'no-forbidden-identifiers/no-forbidden-identifiers',
        identifier: 'event',
      },
    });
  });

  it('computes positions correctly across newline boundaries', () => {
    const doc = createTextDocument('file:///a.ts', 'const x = 1;\nconst event = 2;\n');
    const diagnostic = diagnosticForIdentifier(doc, 'rule', 'event', 'msg');
    expect(diagnostic.range.start).toEqual({ line: 1, character: 6 });
    expect(diagnostic.range.end).toEqual({ line: 1, character: 11 });
  });

  it('throws when the identifier is not present in the document', () => {
    const doc = createTextDocument('file:///a.ts', 'const x = 1;\n');
    expect(() => diagnosticForIdentifier(doc, 'rule', 'nope', 'msg')).toThrow(
      /not found in fixture text/,
    );
  });
});

describe('quickFixReplaceIdentifier', () => {
  it('builds a workspace edit pointing at the diagnostic range', () => {
    const doc = createTextDocument('file:///a.ts', 'const event = 1;\n');
    const diagnostic = diagnosticForIdentifier(doc, 'rule', 'event', 'msg');
    const fix = quickFixReplaceIdentifier(doc, diagnostic, 'foo');

    expect(fix).toEqual({
      title: 'Replace event with foo',
      kind: 'quickfix',
      diagnostics: [diagnostic],
      edit: {
        changes: {
          [doc.uri]: [{ range: diagnostic.range, newText: 'foo' }],
        },
      },
    });
  });
});

describe('applyTextEdits', () => {
  it('applies a single-line edit', () => {
    const text = 'const event = 1;\n';
    const result = applyTextEdits(text, [
      {
        range: {
          start: { line: 0, character: 6 },
          end: { line: 0, character: 11 },
        },
        newText: 'reservedEvent',
      },
    ]);
    expect(result).toBe('const reservedEvent = 1;\n');
  });

  it('applies multiple edits in reverse so earlier offsets stay valid', () => {
    const text = 'event;event;\n';
    const result = applyTextEdits(text, [
      {
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 5 },
        },
        newText: 'A',
      },
      {
        range: {
          start: { line: 0, character: 6 },
          end: { line: 0, character: 11 },
        },
        newText: 'B',
      },
    ]);
    expect(result).toBe('A;B;\n');
  });

  it('is a no-op when no edits are provided', () => {
    const text = 'foo;\n';
    expect(applyTextEdits(text, [])).toBe(text);
  });

  it('handles edits that span multiple lines', () => {
    const text = 'a\nb\nc\n';
    const result = applyTextEdits(text, [
      {
        range: {
          start: { line: 0, character: 0 },
          end: { line: 2, character: 0 },
        },
        newText: 'X\n',
      },
    ]);
    expect(result).toBe('X\nc\n');
  });

  it('clamps edits whose end position is past the end of the text', () => {
    const text = 'foo\n';
    const result = applyTextEdits(text, [
      {
        range: {
          start: { line: 0, character: 0 },
          end: { line: 5, character: 99 },
        },
        newText: 'bar',
      },
    ]);
    expect(result).toBe('bar');
  });
});
