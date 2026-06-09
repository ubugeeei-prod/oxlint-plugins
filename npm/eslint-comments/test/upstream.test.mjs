// Replays the upstream eslint-comments test suite (captured verbatim into
// test/fixtures/*.json by `pnpm run port:tests:eslint-comments`) against this
// plugin, so behavior stays faithful to upstream as the submodule is bumped.

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { matchError, runRule } from './harness.mjs';

const FIXTURES_DIR = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');
const fixtureFiles = readdirSync(FIXTURES_DIR).filter((name) => name.endsWith('.json'));

function label(testCase, index) {
  const code = JSON.stringify(testCase.code);
  const options =
    testCase.options && testCase.options.length > 0
      ? ` options=${JSON.stringify(testCase.options)}`
      : '';
  return `#${index} ${code}${options}`;
}

// Match a list of actual reports against declared expectations. `errors` may be
// a count (number) or a list of string/object expectations.
function assertErrors(actual, expectedErrors) {
  if (typeof expectedErrors === 'number') {
    expect(actual.length).toBe(expectedErrors);
    return;
  }

  expect(actual.length).toBe(expectedErrors.length);
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

describe('eslint-comments upstream parity', () => {
  expect(fixtureFiles.length).toBeGreaterThan(0);

  for (const file of fixtureFiles) {
    const ruleName = file.replace(/\.json$/, '');
    const fixture = JSON.parse(readFileSync(join(FIXTURES_DIR, file), 'utf8'));

    describe(ruleName, () => {
      describe('valid', () => {
        fixture.valid.forEach((testCase, index) => {
          it(label(testCase, index), () => {
            expect(runRule(ruleName, testCase)).toEqual([]);
          });
        });
      });

      describe('invalid', () => {
        fixture.invalid.forEach((testCase, index) => {
          it(label(testCase, index), () => {
            assertErrors(runRule(ruleName, testCase), testCase.errors);
          });
        });
      });
    });
  }
});
