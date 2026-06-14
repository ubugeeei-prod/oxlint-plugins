// Ported from upstream src/processor.test.ts.

import { describe, expect, it } from 'vitest';

import { createPlProcessor } from '../processor.js';

describe('createPlProcessor', () => {
  const sql = `CREATE FUNCTION f() RETURNS int AS $$
  return 42;
$$ LANGUAGE plv8;
CREATE FUNCTION g() RETURNS int AS $$ return 1; $$ LANGUAGE plpython3u;
`;

  it('emits one virtual file per body with the configured extension', () => {
    const processor = createPlProcessor({
      languages: { plv8: '.js', plpython3u: '.py' },
    });

    const blocks = processor.preprocess(sql, 'queries.sql');
    expect(blocks).toEqual([
      { text: '\n  return 42;\n', filename: '0.js' },
      { text: ' return 1; ', filename: '1.py' },
    ]);
  });

  it('skips bodies whose language is not mapped (default)', () => {
    const processor = createPlProcessor({ languages: { plv8: '.js' } });
    const blocks = processor.preprocess(sql, 'queries.sql');
    expect(blocks).toEqual([{ text: '\n  return 42;\n', filename: '0.js' }]);
  });

  it("throws on unknown languages when unknown: 'error'", () => {
    const processor = createPlProcessor({
      languages: { plv8: '.js' },
      unknown: 'error',
    });
    expect(() => processor.preprocess(sql, 'queries.sql')).toThrow(/plpython3u/);
  });

  it('translates message line/column back to the SQL coordinate system', () => {
    const processor = createPlProcessor({
      languages: { plv8: '.js', plpython3u: '.py' },
    });
    processor.preprocess(sql, 'queries.sql');

    const messages = processor.postprocess(
      [[{ ruleId: 'no-magic-numbers', line: 2, column: 3 }], []],
      'queries.sql',
    );

    expect(messages).toEqual([{ ruleId: 'no-magic-numbers', line: 2, column: 3 }]);
  });

  it("translates first-line columns by adding the body's start column", () => {
    const oneLine = `CREATE FUNCTION x() RETURNS int AS $$ return 1; $$ LANGUAGE plv8;`;
    const processor = createPlProcessor({ languages: { plv8: '.js' } });
    processor.preprocess(oneLine, 'q.sql');

    const out = processor.postprocess([[{ ruleId: 'r', line: 1, column: 2 }]], 'q.sql');
    expect(out[0]).toMatchObject({ line: 1, column: 39 });
  });

  it("offsets dollar-quote fix ranges by the body's absolute start", () => {
    const oneLine = `CREATE FUNCTION x() RETURNS int AS $$ return 1; $$ LANGUAGE plv8;`;
    const processor = createPlProcessor({ languages: { plv8: '.js' } });
    processor.preprocess(oneLine, 'q.sql');

    const out = processor.postprocess(
      [[{ ruleId: 'r', line: 1, column: 1, fix: { range: [1, 7], text: 'RETURN' } }]],
      'q.sql',
    );
    expect(out[0]?.fix).toEqual({ range: [38, 44], text: 'RETURN' });
  });

  it('drops fixes for single-quoted bodies to avoid wrong positions', () => {
    const sqlSingle = `CREATE FUNCTION p() RETURNS void AS '
  RAISE NOTICE ''hi'';
' LANGUAGE plpgsql;`;
    const processor = createPlProcessor({ languages: { plpgsql: '.plpgsql' } });
    processor.preprocess(sqlSingle, 'p.sql');
    const out = processor.postprocess(
      [[{ ruleId: 'r', line: 1, column: 1, fix: { range: [0, 1], text: 'X' } }]],
      'p.sql',
    );
    expect(out[0]?.fix).toBeUndefined();
  });

  it('declares supportsAutofix and meta name/version', () => {
    const processor = createPlProcessor({ languages: { plv8: '.js' } });
    expect(processor.supportsAutofix).toBe(true);
    expect(processor.meta.name).toBe('postgresql-eslint-parser/processor');
    expect(processor.meta.version).toMatch(/\d+/);
  });
});
