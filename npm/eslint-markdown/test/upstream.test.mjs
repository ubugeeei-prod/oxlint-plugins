// Replays the upstream @eslint/markdown test suite (captured verbatim into
// test/fixtures/*.json by `pnpm run port:tests:eslint-markdown`) against this
// plugin, so behavior stays faithful to upstream as the submodule is bumped.
//
// Design note — exact parity, ratcheted per rule:
//   The harness drives each case through the ported rule's createOnce/Program
//   adapter and asserts EXACT upstream parity: every reported error's
//   messageId, data, and 1-indexed location must match, and cases that declare
//   `output` must autofix to the same text. Enforcement is per rule via
//   parity.json (see its `_doc`). Levels:
//     - 'full' (the default for any rule not listed) — exact parity is enforced
//       for every valid and invalid case.
//     - 'off' — quarantined: the rule still diverges from upstream, so its
//       cases are registered as skipped (coverage stays visible) until the Rust
//       core is fixed and the rule is promoted to 'full'. The end state is zero
//       'off' entries.

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { matchError, runRule } from './harness.mjs';

const FIXTURES_DIR = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');
const PARITY_FILE = join(dirname(fileURLToPath(import.meta.url)), 'parity.json');

const fixtureFiles = readdirSync(FIXTURES_DIR)
  .filter((name) => name.endsWith('.json'))
  .sort();
const parityRules = JSON.parse(readFileSync(PARITY_FILE, 'utf8')).rules ?? {};

// Resolve the enforcement level for a rule (default 'full' when unlisted).
function levelFor(rule) {
  return parityRules[rule]?.level ?? 'full';
}

// Defensive: a `code` carrying a lone surrogate is not well-formed UTF-8 and
// cannot survive the plugin's UTF-8 source-text boundary (it becomes U+FFFD).
// Such a string also cannot occur in a real on-disk Markdown file. None are
// expected in the markdown suite, but skip-and-count keeps any gap visible.
function isReplayable(testCase) {
  return typeof testCase.code !== 'string' || testCase.code.isWellFormed();
}

function label(testCase, index) {
  const code = JSON.stringify(testCase.code);
  const options =
    testCase.options && testCase.options.length > 0
      ? ` options=${JSON.stringify(testCase.options)}`
      : '';
  const languageOptions = testCase.languageOptions
    ? ` languageOptions=${JSON.stringify(testCase.languageOptions)}`
    : '';
  return `#${index} ${code}${options}${languageOptions}`;
}

// Match a list of actual reports against declared expectations. `errors` may be
// a count (number) or a list of string/object expectations.
function assertErrors(actual, expectedErrors) {
  if (typeof expectedErrors === 'number') {
    expect(actual.length).toBe(expectedErrors);
    return;
  }

  expect(
    actual.length,
    `error count mismatch\n  actual:   ${JSON.stringify(actual)}\n  expected: ${JSON.stringify(
      expectedErrors,
    )}`,
  ).toBe(expectedErrors.length);

  expectedErrors.forEach((expected, index) => {
    const result = matchError(actual[index], expected);
    expect(
      result.ok,
      `error #${index} mismatch on "${result.field}"\n  actual:   ${JSON.stringify(
        actual[index],
      )}\n  expected: ${JSON.stringify(expected)}`,
    ).toBe(true);
  });
}

// Summary logged at startup so CI output makes coverage visible.
const counts = { full: 0, off: 0 };
let nonUtf8 = 0;
for (const file of fixtureFiles) {
  counts[levelFor(file.replace(/\.json$/, '')) === 'off' ? 'off' : 'full']++;
  const fixture = JSON.parse(readFileSync(join(FIXTURES_DIR, file), 'utf8'));
  for (const testCase of [...fixture.valid, ...fixture.invalid]) {
    if (!isReplayable(testCase)) {
      nonUtf8 += 1;
    }
  }
}
console.log(
  `eslint-markdown upstream parity: ${counts.full} full (exact) | ${counts.off} quarantined (off)` +
    `${nonUtf8 > 0 ? ` | ${nonUtf8} non-UTF-8 case(s) skipped` : ''}`,
);

describe('eslint-markdown upstream parity', () => {
  expect(fixtureFiles.length).toBeGreaterThan(0);

  for (const file of fixtureFiles) {
    const ruleName = file.replace(/\.json$/, '');
    const fixture = JSON.parse(readFileSync(join(FIXTURES_DIR, file), 'utf8'));
    const quarantined = levelFor(ruleName) === 'off';
    // The dialect for a case is its own `language` override, falling back to the
    // suite-level language captured at sync time.
    const suiteLanguage = fixture.__generated?.language ?? 'markdown/gfm';
    const dialectOf = (testCase) => testCase.language ?? suiteLanguage;

    describe(ruleName, () => {
      describe('valid', () => {
        fixture.valid.forEach((testCase, index) => {
          const run = quarantined || !isReplayable(testCase) ? it.skip : it;
          run(label(testCase, index), () => {
            expect(runRule(ruleName, testCase, dialectOf(testCase)).reports).toEqual([]);
          });
        });
      });

      describe('invalid', () => {
        fixture.invalid.forEach((testCase, index) => {
          if (quarantined || !isReplayable(testCase)) {
            it.skip(label(testCase, index), () => {});
            return;
          }
          it(label(testCase, index), () => {
            const { reports, output } = runRule(ruleName, testCase, dialectOf(testCase));
            assertErrors(reports, testCase.errors);
            if ('output' in testCase) {
              const expectedOutput = testCase.output === null ? testCase.code : testCase.output;
              expect(output).toBe(expectedOutput);
            }
          });
        });
      });
    });
  }
});
