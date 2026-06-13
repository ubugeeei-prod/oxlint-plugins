// Replays the upstream eslint-plugin-simple-import-sort test suite (captured
// verbatim into test/fixtures/*.json by `pnpm run port:tests:simple-import-sort`)
// against this plugin, so behavior stays faithful to upstream as the submodule
// is bumped.
//
// simple-import-sort is an autofix rule, so each invalid upstream case carries
// the exact fixed source. The replay drives a case's `code` through the rule's
// adapter (which calls the Rust scanner), applies the reported fixes, and
// compares the result to upstream's `output`. How strictly each rule is
// asserted is governed by test/parity.json (see its `$comment`); cases whose
// parser is out of scope (Flow) are counted and logged, never silently dropped.

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = join(TEST_DIR, 'fixtures');
const parity = JSON.parse(readFileSync(join(TEST_DIR, 'parity.json'), 'utf8'));

const RULES = ['imports', 'exports'];

// Drive one rule's adapter over `code` and return its `context.report`
// descriptors. The rule logic lives in Rust (`scanSimpleImportSort`); the JS
// layer is only an Oxlint/NAPI adapter.
function runRule(ruleName, testCase) {
  const code = testCase.code;
  const sourceCode = {
    text: code,
    getText() {
      return this.text;
    },
  };
  const reports = [];
  const context = {
    id: ruleName,
    options: testCase.options ?? [],
    sourceCode,
    // TypeScript-parser cases may use TS-only syntax (e.g. `import type`), so
    // they are scanned under a `.ts` filename; everything else is plain JS.
    filename: testCase.parser === 'ts' ? 'file.ts' : 'file.js',
    report(descriptor) {
      reports.push(descriptor);
    },
  };
  const visitor = plugin.rules[ruleName].createOnce(context);
  visitor.before?.();
  visitor.Program?.({ type: 'Program', range: [0, code.length] });
  visitor.after?.();
  return reports;
}

// Resolve each report's fix to a `{ start, end, text }` range, then apply them
// all to `source`. Fixes target disjoint chunks, so a single right-to-left pass
// reproduces ESLint's fixer for these cases.
function applyFixes(source, reports) {
  const fixes = reports
    .map((report) =>
      typeof report.fix === 'function'
        ? report.fix({ replaceTextRange: (range, text) => ({ range, text }) })
        : undefined,
    )
    .filter((fix) => fix && Array.isArray(fix.range))
    .sort((a, b) => b.range[0] - a.range[0]);

  let output = source;
  for (const fix of fixes) {
    output = output.slice(0, fix.range[0]) + fix.text + output.slice(fix.range[1]);
  }
  return output;
}

function ruleConfig(ruleName) {
  const config = parity.rules?.[ruleName] ?? {};
  return {
    level: config.level ?? 'valid',
    skipParsers: new Set(config.skipParsers ?? []),
  };
}

function loadFixture(ruleName) {
  return JSON.parse(readFileSync(join(FIXTURES_DIR, `${ruleName}.json`), 'utf8'));
}

function label(testCase, index) {
  const code = JSON.stringify(testCase.code);
  const truncated = code.length > 72 ? `${code.slice(0, 72)}…` : code;
  return `#${index} [${testCase.parser}] ${truncated}`;
}

for (const ruleName of RULES) {
  const fixture = loadFixture(ruleName);
  const { level, skipParsers } = ruleConfig(ruleName);

  describe(`simple-import-sort/${ruleName} upstream parity (level=${level})`, () => {
    const valid = fixture.valid ?? [];
    const invalid = fixture.invalid ?? [];

    let skippedByParser = 0;
    let skippedByLevel = 0;

    describe('valid', () => {
      valid.forEach((testCase, index) => {
        if (skipParsers.has(testCase.parser)) {
          skippedByParser += 1;
          it.skip(label(testCase, index), () => {});
          return;
        }
        if (level === 'off') {
          skippedByLevel += 1;
          it.skip(label(testCase, index), () => {});
          return;
        }
        it(label(testCase, index), () => {
          const reports = runRule(ruleName, testCase);
          expect(
            reports,
            `expected no ${ruleName} diagnostics for upstream-valid code:\n${testCase.code}`,
          ).toEqual([]);
        });
      });
    });

    describe('invalid', () => {
      invalid.forEach((testCase, index) => {
        if (skipParsers.has(testCase.parser)) {
          skippedByParser += 1;
          it.skip(label(testCase, index), () => {});
          return;
        }
        if (level === 'off' || level === 'valid') {
          skippedByLevel += 1;
          it.skip(label(testCase, index), () => {});
          return;
        }
        it(label(testCase, index), () => {
          const reports = runRule(ruleName, testCase);
          expect(reports.length, `diagnostic count mismatch for:\n${testCase.code}`).toBe(
            testCase.errors,
          );
          if (level === 'full') {
            expect(applyFixes(testCase.code, reports)).toBe(testCase.output);
          }
        });
      });
    });

    // Surface what was not asserted, so the ratchet level and parser scope are
    // always visible in the test output — coverage is never silently dropped.
    it(`parity summary (${valid.length} valid, ${invalid.length} invalid)`, () => {
      if (skippedByParser > 0 || skippedByLevel > 0) {
        const parts = [];
        if (skippedByParser > 0) {
          parts.push(`${skippedByParser} skipped by parser scope (${[...skipParsers].join(', ')})`);
        }
        if (skippedByLevel > 0) {
          parts.push(`${skippedByLevel} skipped at level "${level}"`);
        }
        console.info(`simple-import-sort/${ruleName}: ${parts.join('; ')}`);
      }
      expect(valid.length + invalid.length).toBeGreaterThan(0);
    });
  });
}
