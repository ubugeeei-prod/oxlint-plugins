import { describe, expect, it } from 'vitest';

import { isForbiddenIdentifierName, scanForbiddenIdentifiers } from '../api.js';

describe('scanForbiddenIdentifiers', () => {
  it('exposes NAPI-backed scanning of default forbidden identifiers', () => {
    expect(scanForbiddenIdentifiers('const event = data.error;')).toEqual([
      'event',
      'error',
      'data',
    ]);
  });

  it('returns an empty array when no forbidden identifiers are present', () => {
    expect(scanForbiddenIdentifiers('const value = input;')).toEqual([]);
  });

  it('returns an empty array for an empty source text', () => {
    expect(scanForbiddenIdentifiers('')).toEqual([]);
  });

  it('returns each match at most once even when it appears multiple times', () => {
    const matches = scanForbiddenIdentifiers('event(); event(); event();');
    expect(matches.filter((name) => name === 'event')).toEqual(['event']);
  });

  it('does not match identifiers embedded inside other identifiers', () => {
    expect(scanForbiddenIdentifiers('const eventBus = 1;')).toEqual([]);
    expect(scanForbiddenIdentifiers('const myEvent = 1;')).toEqual([]);
    expect(scanForbiddenIdentifiers('const event_handler = 1;')).toEqual([]);
    expect(scanForbiddenIdentifiers('const $event = 1;')).toEqual([]);
    expect(scanForbiddenIdentifiers('const event$ = 1;')).toEqual([]);
    expect(scanForbiddenIdentifiers('const event0 = 1;')).toEqual([]);
    expect(scanForbiddenIdentifiers('const _event = 1;')).toEqual([]);
  });

  it('respects identifier boundaries with surrounding punctuation', () => {
    expect(scanForbiddenIdentifiers('call(event)')).toEqual(['event']);
    expect(scanForbiddenIdentifiers('call(event);')).toEqual(['event']);
    expect(scanForbiddenIdentifiers('a.event')).toEqual(['event']);
    expect(scanForbiddenIdentifiers('a[event]')).toEqual(['event']);
    expect(scanForbiddenIdentifiers('event\n')).toEqual(['event']);
    expect(scanForbiddenIdentifiers('event\t')).toEqual(['event']);
  });

  it('finds an identifier at the very start or very end of the source', () => {
    expect(scanForbiddenIdentifiers('event')).toEqual(['event']);
    expect(scanForbiddenIdentifiers(' event ')).toEqual(['event']);
    expect(scanForbiddenIdentifiers('foo;event')).toEqual(['event']);
  });

  it('augments defaults with custom names when provided', () => {
    expect(
      scanForbiddenIdentifiers('function run(ctx) { return payload + event; }', {
        names: ['ctx', 'payload'],
      }),
    ).toEqual(expect.arrayContaining(['ctx', 'payload', 'event']));
  });

  it('keeps defaults active when custom names are supplied', () => {
    const matches = scanForbiddenIdentifiers('const ctx = error;', { names: ['ctx'] });
    expect(matches).toEqual(expect.arrayContaining(['ctx', 'error']));
  });

  it('ignores empty strings in custom names', () => {
    expect(scanForbiddenIdentifiers('const x = 1;', { names: [''] })).toEqual([]);
  });

  it('ignores non-string entries in custom names', () => {
    // The JS wrapper filters non-strings before calling native.
    expect(
      scanForbiddenIdentifiers('const ctx = 1;', { names: [123, null, undefined, true, 'ctx'] }),
    ).toEqual(['ctx']);
  });

  it('treats options without a names array as no custom names', () => {
    expect(scanForbiddenIdentifiers('const ctx = 1;', {})).toEqual([]);
    expect(scanForbiddenIdentifiers('const ctx = 1;', { names: null })).toEqual([]);
    expect(scanForbiddenIdentifiers('const ctx = 1;', { names: 'ctx' })).toEqual([]);
  });

  it('treats nullish options as no custom names', () => {
    expect(scanForbiddenIdentifiers('const event = 1;', null)).toEqual(['event']);
    expect(scanForbiddenIdentifiers('const event = 1;', undefined)).toEqual(['event']);
  });

  it('does not match comments outside of identifier-shaped tokens', () => {
    // The pre-scan is identifier-aware, so the word "event" inside a comment
    // is still an identifier-like substring with proper boundaries; the
    // visitor in the rule itself is what filters by AST. Confirm the scan
    // surfaces the substring so the rule can see it.
    expect(scanForbiddenIdentifiers('// event\n')).toEqual(['event']);
  });

  it('matches identifiers separated by unicode whitespace boundaries', () => {
    expect(scanForbiddenIdentifiers('const a = event\u00a0;')).toEqual(['event']);
  });

  it('handles non-ASCII source text without panicking', () => {
    expect(scanForbiddenIdentifiers('// 日本語\nconst event = 1;\n')).toEqual(['event']);
  });

  it('throws a TypeError when sourceText is not a string', () => {
    expect(() => scanForbiddenIdentifiers(123)).toThrow(TypeError);
    expect(() => scanForbiddenIdentifiers(null)).toThrow(TypeError);
    expect(() => scanForbiddenIdentifiers(undefined)).toThrow(TypeError);
    expect(() => scanForbiddenIdentifiers({})).toThrow(TypeError);
  });

  it('reports custom name and default in stable order', () => {
    // Custom names come first followed by defaults that match.
    const matches = scanForbiddenIdentifiers('const event = ctx;', { names: ['ctx'] });
    expect(matches[0]).toBe('ctx');
    expect(matches).toContain('event');
  });
});

describe('isForbiddenIdentifierName', () => {
  it('returns true for default forbidden names', () => {
    expect(isForbiddenIdentifierName('event')).toBe(true);
    expect(isForbiddenIdentifierName('error')).toBe(true);
    expect(isForbiddenIdentifierName('data')).toBe(true);
  });

  it('returns false for unrelated names', () => {
    expect(isForbiddenIdentifierName('value')).toBe(false);
    expect(isForbiddenIdentifierName('foo')).toBe(false);
    expect(isForbiddenIdentifierName('')).toBe(false);
  });

  it('returns true for custom names', () => {
    expect(isForbiddenIdentifierName('ctx', { names: ['ctx'] })).toBe(true);
  });

  it('returns false for names that are not in the configured list', () => {
    expect(isForbiddenIdentifierName('value', { names: ['ctx'] })).toBe(false);
  });

  it('keeps defaults forbidden even when custom names are supplied', () => {
    expect(isForbiddenIdentifierName('event', { names: ['ctx'] })).toBe(true);
  });

  it('is case-sensitive', () => {
    expect(isForbiddenIdentifierName('Event')).toBe(false);
    expect(isForbiddenIdentifierName('EVENT')).toBe(false);
    expect(isForbiddenIdentifierName('Data')).toBe(false);
  });

  it('ignores empty strings inside custom names', () => {
    expect(isForbiddenIdentifierName('foo', { names: [''] })).toBe(false);
  });

  it('accepts options without a names property', () => {
    expect(isForbiddenIdentifierName('event', {})).toBe(true);
    expect(isForbiddenIdentifierName('foo', {})).toBe(false);
  });

  it('throws a TypeError when name is not a string', () => {
    expect(() => isForbiddenIdentifierName(123)).toThrow(TypeError);
    expect(() => isForbiddenIdentifierName(null)).toThrow(TypeError);
    expect(() => isForbiddenIdentifierName(undefined)).toThrow(TypeError);
    expect(() => isForbiddenIdentifierName({})).toThrow(TypeError);
  });

  it('treats nullish options as defaults-only mode', () => {
    expect(isForbiddenIdentifierName('event', null)).toBe(true);
    expect(isForbiddenIdentifierName('event', undefined)).toBe(true);
    expect(isForbiddenIdentifierName('ctx', null)).toBe(false);
  });
});
