// Replays the upstream eslint-plugin-postgresql fixture suite (captured verbatim
// into test/fixtures/*.json by `pnpm run port:tests:postgresql`) against this
// plugin, so behaviour stays faithful to upstream as the submodule is bumped.
//
// Design note — exact parity, ratcheted per rule:
//   The harness drives each case through the ported rule's createOnce/Program
//   adapter and asserts EXACT upstream parity: every reported error's messageId
//   and 1-indexed location must match, and cases that declare `output` must
//   autofix to the same text. Enforcement is per rule via parity.json:
//     - 'full' (default for any implemented rule not listed) — exact parity is
//       enforced for every valid and invalid case.
//     - 'off' — quarantined: the rule diverges from upstream, so its cases are
//       registered as skipped (coverage stays visible) until the Rust core is
//       fixed and the rule is promoted to 'full'. The end state is zero 'off'.
//   Rules not yet implemented in Rust are registered as skipped automatically.

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';
import { matchError, runRule } from './harness.mjs';

const FIXTURES_DIR = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');
const PARITY_FILE = join(dirname(fileURLToPath(import.meta.url)), 'parity.json');

const fixtureFiles = readdirSync(FIXTURES_DIR)
  .filter((name) => name.endsWith('.json'))
  .sort();
const parityRules = JSON.parse(readFileSync(PARITY_FILE, 'utf8')).rules ?? {};
const implemented = new Set(plugin.implementedPostgresqlRuleNames);

function levelFor(rule) {
  if (!implemented.has(rule)) {
    return 'pending';
  }
  return parityRules[rule]?.level ?? 'full';
}

function label(testCase, index) {
  return `#${index} ${testCase.name ?? JSON.stringify(testCase.code)}`;
}

// Match a list of actual reports against declared expectations.
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

  for (let i = 0; i < expectedErrors.length; i++) {
    const result = matchError(actual[i], expectedErrors[i]);
    expect(
      result.ok,
      `error #${i} field "${result.field}" mismatch\n  actual:   ${JSON.stringify(
        actual[i],
      )}\n  expected: ${JSON.stringify(expectedErrors[i])}`,
    ).toBe(true);
  }
}

for (const file of fixtureFiles) {
  const rule = file.slice(0, -'.json'.length);
  const data = JSON.parse(readFileSync(join(FIXTURES_DIR, file), 'utf8'));
  const valid = data.valid ?? [];
  const invalid = data.invalid ?? [];

  describe(rule, () => {
    const level = levelFor(rule);

    if (level !== 'full') {
      const reason =
        level === 'pending' ? 'not yet ported' : 'quarantined (diverges from upstream)';
      it.skip(`${rule}: ${reason} — ${valid.length} valid / ${invalid.length} invalid`, () => {});
      return;
    }

    for (const [index, testCase] of valid.entries()) {
      it(`valid ${label(testCase, index)}`, () => {
        const { reports } = runRule(rule, testCase);
        expect(reports, `unexpected diagnostics: ${JSON.stringify(reports)}`).toEqual([]);
      });
    }

    for (const [index, testCase] of invalid.entries()) {
      it(`invalid ${label(testCase, index)}`, () => {
        const { reports, output } = runRule(rule, testCase);
        assertErrors(reports, testCase.errors ?? []);
        if (testCase.output !== null && testCase.output !== undefined) {
          expect(output).toBe(testCase.output);
        }
      });
    }
  });
}
