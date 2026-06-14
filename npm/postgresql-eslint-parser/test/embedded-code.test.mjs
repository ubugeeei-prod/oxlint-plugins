// Ported from upstream src/embeddedCode.test.ts.

import { describe, expect, it } from 'vitest';

import { extractEmbeddedCode, parseForESLint } from '../api.js';

describe('extractEmbeddedCode', () => {
  it('returns an EmbeddedCode for every PL function body', () => {
    const sql = `
CREATE FUNCTION a() RETURNS int AS $$ return 1; $$ LANGUAGE plv8;
CREATE FUNCTION b() RETURNS int AS $$ return 2; $$ LANGUAGE plv8;
`;
    const { ast } = parseForESLint(sql);
    const bodies = extractEmbeddedCode(ast);
    expect(bodies).toHaveLength(2);
    expect(bodies[0]?.source).toBe(' return 1; ');
    expect(bodies[1]?.source).toBe(' return 2; ');
  });

  it('preserves source order', () => {
    const sql = `
CREATE FUNCTION first() RETURNS text AS $$ return "x"; $$ LANGUAGE plv8;
CREATE FUNCTION second() RETURNS text AS $$ return "y"; $$ LANGUAGE plpython3u;
CREATE FUNCTION third() RETURNS text AS $$ return "z"; $$ LANGUAGE plv8;
`;
    const { ast } = parseForESLint(sql);
    const langs = extractEmbeddedCode(ast).map((b) => b.language);
    expect(langs).toEqual(['plv8', 'plpython3u', 'plv8']);
  });

  it('reports absolute SQL ranges that slice back to the body', () => {
    const sql = 'CREATE FUNCTION f() RETURNS int AS $$  return 42;  $$ LANGUAGE plv8;';
    const { ast } = parseForESLint(sql);
    const [body] = extractEmbeddedCode(ast);
    expect(body).toBeDefined();
    const sliced = sql.slice(body.range[0], body.range[1]);
    expect(sliced).toBe(body.source);
  });

  it('skips C-style two-argument AS clauses', () => {
    const sql = `CREATE FUNCTION c() RETURNS int AS 'libname', 'symbol' LANGUAGE c;`;
    const { ast } = parseForESLint(sql);
    expect(extractEmbeddedCode(ast)).toEqual([]);
  });

  it('handles dollar-quote tags', () => {
    const sql = `CREATE FUNCTION t() RETURNS int AS $body$ return 1; $body$ LANGUAGE plv8;`;
    const { ast } = parseForESLint(sql);
    const [body] = extractEmbeddedCode(ast);
    expect(body?.quoteStyle).toBe('dollar');
    expect(body?.source).toBe(' return 1; ');
  });

  it('lower-cases the LANGUAGE clause', () => {
    const sql = `CREATE FUNCTION u() RETURNS int AS $$ return 1; $$ LANGUAGE PLV8;`;
    const { ast } = parseForESLint(sql);
    expect(extractEmbeddedCode(ast)[0]?.language).toBe('plv8');
  });
});
