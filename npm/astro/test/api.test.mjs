import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { implementedAstroRuleNames, scanAstro } from '../api.js';

const testRoot = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(readFileSync(join(testRoot, 'fixtures', 'astro-v3.0.1.json'), 'utf8'));
const expectedRuleNames = [
  'no-deprecated-astro-canonicalurl',
  'no-deprecated-astro-fetchcontent',
  'no-deprecated-astro-resolve',
  'no-deprecated-getentrybyslug',
  'no-set-html-directive',
  'no-set-text-directive',
  'prefer-class-list-directive',
];
const messageIds = {
  'no-deprecated-astro-canonicalurl': 'deprecated',
  'no-deprecated-astro-fetchcontent': 'deprecated',
  'no-deprecated-astro-resolve': 'deprecated',
  'no-deprecated-getentrybyslug': 'deprecated',
  'no-set-html-directive': 'unexpected',
  'no-set-text-directive': 'disallow',
  'prefer-class-list-directive': 'unexpected',
};

function applyNativeFixes(sourceText, diagnostics) {
  return diagnostics
    .flatMap((diagnostic) => (diagnostic.fix ? [diagnostic.fix] : []))
    .sort((left, right) => right.start - left.start)
    .reduce((output, fix) => {
      const bytes = Buffer.from(output);
      return Buffer.concat([
        bytes.subarray(0, fix.start),
        Buffer.from(fix.replacement),
        bytes.subarray(fix.end),
      ]).toString();
    }, sourceText);
}

describe('astro native API', () => {
  it('exposes both implemented eslint-plugin-astro slices', () => {
    expect(implementedAstroRuleNames()).toEqual(expectedRuleNames);
  });

  it.each(
    Object.entries(fixture.rules).flatMap(([ruleName, cases]) =>
      cases.valid.map((testCase) => [ruleName, testCase]),
    ),
  )('accepts authored upstream valid fixture for %s', (ruleName, testCase) => {
    expect(scanAstro(testCase.code, testCase.filename, { ruleNames: [ruleName] })).toEqual([]);
  });

  it.each(
    Object.entries(fixture.rules).flatMap(([ruleName, cases]) =>
      cases.invalid.map((testCase) => [ruleName, testCase]),
    ),
  )('replays authored upstream invalid fixture for %s', (ruleName, testCase) => {
    const diagnostics = scanAstro(testCase.code, testCase.filename, {
      ruleNames: [ruleName],
    });
    expect(diagnostics).toHaveLength(testCase.errors.length);
    expect(
      diagnostics.map((diagnostic) => ({
        messageId: diagnostic.messageId,
        line: diagnostic.loc.startLine,
        column: diagnostic.loc.startColumn + 1,
      })),
    ).toEqual(
      testCase.errors.map((error) => ({
        messageId: messageIds[ruleName],
        line: error.line,
        column: error.column,
      })),
    );
  });

  it.each(
    Object.entries(fixture.rules).flatMap(([ruleName, cases]) =>
      cases.invalid
        .filter((testCase) => testCase.output !== undefined)
        .map((testCase) => [ruleName, testCase]),
    ),
  )('replays authored upstream fixed output for %s', (ruleName, testCase) => {
    const diagnostics = scanAstro(testCase.code, testCase.filename, {
      ruleNames: [ruleName],
    });
    expect(applyNativeFixes(testCase.code, diagnostics)).toBe(testCase.output);
  });

  it.each(
    Object.entries(fixture.rules).flatMap(([ruleName, cases]) =>
      cases.invalid
        .filter((testCase) => testCase.output !== undefined && testCase.output !== testCase.code)
        .map((testCase) => [ruleName, testCase]),
    ),
  )('is stable after applying authored %s output once', (ruleName, testCase) => {
    const diagnostics = scanAstro(testCase.output, testCase.filename, {
      ruleNames: [ruleName],
    });
    expect(applyNativeFixes(testCase.output, diagnostics)).toBe(testCase.output);
  });

  it('returns native UTF-8 byte fix ranges', () => {
    const source = '---\nconst emoji = "😀"; Astro.fetchContent("*.md")\n---\n';
    const [diagnostic] = scanAstro(source);
    const propertyStart = source.indexOf('fetchContent');
    expect(diagnostic.fix).toEqual({
      start: Buffer.byteLength(source.slice(0, propertyStart)),
      end: Buffer.byteLength(source.slice(0, propertyStart + 'fetchContent'.length)),
      replacement: 'glob',
    });
  });

  it('is clean and stable after applying the native fix once', () => {
    const source = '---\nconst emoji = "😀"; Astro.fetchContent("*.md")\n---\n';
    const [diagnostic] = scanAstro(source);
    const bytes = Buffer.from(source);
    const fixed = Buffer.concat([
      bytes.subarray(0, diagnostic.fix.start),
      Buffer.from(diagnostic.fix.replacement),
      bytes.subarray(diagnostic.fix.end),
    ]).toString();
    expect(fixed).toBe('---\nconst emoji = "😀"; Astro.glob("*.md")\n---\n');
    expect(scanAstro(fixed)).toEqual([]);
  });

  it('isolates one requested rule', () => {
    const source = '---\nimport { getEntryBySlug } from "astro:content"\nAstro.canonicalURL\n---\n';
    expect(
      scanAstro(source, 'fixture.astro', {
        ruleNames: ['no-deprecated-getentrybyslug'],
      }).map((diagnostic) => diagnostic.ruleName),
    ).toEqual(['no-deprecated-getentrybyslug']);
  });

  it('enables every implemented rule when ruleNames is omitted', () => {
    const source = '---\nimport { getEntryBySlug } from "astro:content"\nAstro.canonicalURL\n---\n';
    expect(scanAstro(source).map((diagnostic) => diagnostic.ruleName)).toEqual([
      'no-deprecated-getentrybyslug',
      'no-deprecated-astro-canonicalurl',
    ]);
  });

  it('returns no diagnostics for malformed frontmatter', () => {
    expect(scanAstro('---\nconst = Astro.canonicalURL\n---\n')).toEqual([]);
  });

  it('preserves source order for multiple boundary diagnostics', () => {
    const diagnostics = scanAstro(
      '---\nAstro.fetchContent("*.md"); Astro.canonicalURL\nAstro.fetchContent("*.md")\n---\n',
    );
    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'no-deprecated-astro-fetchcontent',
      'no-deprecated-astro-canonicalurl',
      'no-deprecated-astro-fetchcontent',
    ]);
  });

  it('returns no diagnostics for another extension', () => {
    expect(scanAstro('---\nAstro.canonicalURL\n---\n', 'fixture.ts')).toEqual([]);
  });

  it('validates sourceText and filename', () => {
    expect(() => scanAstro(null)).toThrow('sourceText must be a string');
    expect(() => scanAstro('', null)).toThrow('filename must be a string');
  });
});
