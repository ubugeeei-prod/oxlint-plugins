// Replays the upstream eslint-plugin-regexp test suite (captured verbatim into
// test/fixtures/*.json by `pnpm run port:tests:regexp`) against this plugin,
// so behavior stays faithful to upstream as the submodule is bumped.
//
// Design note — diagnostics-only port:
//   The native `scanRegexp(sourceText, filename)` API returns raw diagnostics
//   from the Rust/oxc backend. Messages are NOT asserted here because the port
//   intentionally rewords them (Oxlint message style may differ from ESLint).
//   We assert:
//     - VALID soundness (always): every valid upstream case must emit zero
//       diagnostics for the relevant rule.
//     - COUNT + LOCATION parity (only when parity.json[rule] === 'full'):
//       the number of diagnostics and their start/end line/column must match.
//       Location mapping: native loc uses 0-based UTF-16 columns; the snapshot
//       uses 1-based columns. So `native.startColumn + 1 === fixture.column`
//       and `native.endColumn + 1 === fixture.endColumn`. If CI shows an
//       off-by-one, tune the mapping here — the comment marks the assumption.
//   When parity.json[rule] is absent or not 'full', invalid cases are
//   registered with `it.skip` so coverage gaps are visible in CI output.

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { scanRegexp } from '../api.js';

const FIXTURES_DIR = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');
const PARITY_FILE = join(dirname(fileURLToPath(import.meta.url)), 'parity.json');

const fixtureFiles = readdirSync(FIXTURES_DIR).filter((name) => name.endsWith('.json'));
const parity = JSON.parse(readFileSync(PARITY_FILE, 'utf8'));

// Summary logged at startup so CI output makes coverage visible.
const fullRules = fixtureFiles.filter((f) => parity[f.replace(/\.json$/, '')] === 'full').length;
const validOnlyRules = fixtureFiles.length - fullRules;
console.log(
  `regexp upstream parity: ${fullRules} rule(s) with full (count+location) parity, ` +
    `${validOnlyRules} rule(s) with valid-soundness-only parity`,
);

for (const file of fixtureFiles) {
  const rule = file.replace(/\.json$/, '');
  const fixture = JSON.parse(readFileSync(join(FIXTURES_DIR, file), 'utf8'));
  const isFullParity = parity[rule] === 'full';

  describe(rule, () => {
    describe('valid', () => {
      fixture.valid.forEach((testCase, index) => {
        it(`#${index} ${JSON.stringify(testCase.code)}`, () => {
          const diagnostics = scanRegexp(testCase.code, testCase.filename ?? 'file.js').filter(
            (d) => d.ruleName === rule,
          );
          expect(diagnostics).toEqual([]);
        });
      });
    });

    describe('invalid', () => {
      fixture.invalid.forEach((testCase, index) => {
        const label = `#${index} ${JSON.stringify(testCase.code)}`;

        if (!isFullParity) {
          // Not yet promoted to full parity — register as skipped so the gap
          // is visible without failing CI. Add the rule to parity.json once
          // the native locations are verified.
          it.skip(label, () => {});
          return;
        }

        it(label, () => {
          const actual = scanRegexp(testCase.code, testCase.filename ?? 'file.js').filter(
            (d) => d.ruleName === rule,
          );

          // Count parity
          expect(actual.length).toBe(testCase.errors.length);

          // Location parity per error
          // Mapping assumption: native loc uses 0-based UTF-16 columns;
          // fixture uses 1-based columns from the .eslintsnap marker lines.
          testCase.errors.forEach((expected, i) => {
            const loc = actual[i]?.loc;
            expect(loc?.startLine, `error[${i}].line`).toBe(expected.line);
            // +1 maps 0-based native column to 1-based snapshot column
            expect((loc?.startColumn ?? -1) + 1, `error[${i}].column`).toBe(expected.column);
            expect((loc?.endColumn ?? -1) + 1, `error[${i}].endColumn`).toBe(expected.endColumn);
          });
        });
      });
    });
  });
}
