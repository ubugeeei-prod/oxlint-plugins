// Replays the upstream eslint-plugin-functional test suite (captured verbatim
// into test/fixtures/*.json by `pnpm run port:tests:functional`) against this
// plugin, so behavior stays faithful to upstream as the submodule is bumped.
//
// Each fixture case carries a `typeAware` flag derived from the upstream config
// it ran under (the TypeScript `projectService` config vs the syntax-only babel
// config). The Rust port is syntax-only, so type-aware cases cannot be replayed:
// they are skipped and counted (logged in the parity ledger — no silent
// truncation). Syntactic cases are checked against the rule's parity level:
//
//   - FULL_PARITY: valid -> no reports; invalid -> reported messageIds (and
//     count) match the upstream-declared `errors`. A rule joins this set in its
//     own porting PR once the Rust core emits the upstream messageIds.
//   - otherwise (pending): the case is smoke-run — it must not throw and must
//     return an array — which exercises the whole upstream syntactic corpus
//     through the NAPI binding before per-rule parity assertions land.

import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const FIXTURES_DIR = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');

// Rules whose syntax-only port has reached full upstream parity. Populated one
// entry per porting PR. See the file header for what "full parity" asserts.
const FULL_PARITY = new Set([
  'no-loop-statements',
  'no-this-expressions',
  'no-try-statements',
  'no-promise-reject',
]);

function runRule(ruleName, testCase) {
  const sourceText = testCase.code;
  const filename = testCase.filename ?? 'fixture.ts';
  const options = testCase.options ?? [];
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return sourceText;
    },
  };
  const visitor = plugin.rules[ruleName].createOnce({
    filename,
    options,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.before?.();
  visitor.Program?.({ type: 'Program', range: [0, sourceText.length] });
  visitor.after?.();
  return reports;
}

// The upstream messageId for a report. Rules at full parity emit it directly;
// the pending baseline flattens every diagnostic to `unexpected`.
function reportMessageId(report) {
  return report.messageId ?? 'unexpected';
}

// Compare reported errors against the upstream-declared expectations: a list of
// `{ messageId }` objects (from inline `errors` or the upstream snapshot), a
// count (number), or `undefined` when the upstream `invalid()` call declared no
// expectations and only asserted "at least one error". messageIds are compared
// as a sorted multiset so report ordering does not matter.
function assertErrors(reports, expectedErrors) {
  if (expectedErrors === undefined) {
    expect(reports.length).toBeGreaterThanOrEqual(1);
    return;
  }
  if (typeof expectedErrors === 'number') {
    expect(reports.length).toBe(expectedErrors);
    return;
  }
  const expectedIds = expectedErrors.map((error) => error.messageId).sort();
  const actualIds = reports.map(reportMessageId).sort();
  expect(actualIds).toEqual(expectedIds);
}

function label(testCase, index) {
  const code = JSON.stringify(testCase.code);
  const truncated = code.length > 70 ? `${code.slice(0, 70)}…"` : code;
  const options =
    testCase.options && testCase.options.length > 0
      ? ` options=${JSON.stringify(testCase.options)}`
      : '';
  return `#${index} ${truncated}${options}`;
}

const fixtureFiles = readdirSync(FIXTURES_DIR)
  .filter((name) => name.endsWith('.json'))
  .sort();

describe('eslint-plugin-functional upstream parity', () => {
  it('has fixtures for every ported rule', () => {
    expect(fixtureFiles.length).toBe(plugin.implementedFunctionalRuleNames.length);
    const ruleNames = fixtureFiles.map((name) => name.replace(/\.json$/, ''));
    expect(ruleNames).toEqual([...plugin.implementedFunctionalRuleNames].sort());
  });

  const ledger = [];

  for (const file of fixtureFiles) {
    const ruleName = file.replace(/\.json$/, '');
    const fixture = JSON.parse(readFileSync(join(FIXTURES_DIR, file), 'utf8'));
    const validSyntactic = fixture.valid.filter((testCase) => !testCase.typeAware);
    const invalidSyntactic = fixture.invalid.filter((testCase) => !testCase.typeAware);
    const typeAwareCount =
      fixture.valid.length +
      fixture.invalid.length -
      validSyntactic.length -
      invalidSyntactic.length;
    const full = FULL_PARITY.has(ruleName);

    ledger.push({
      ruleName,
      parity: fixture.__generated.noUpstreamTests ? 'none' : full ? 'full' : 'smoke',
      syntactic: validSyntactic.length + invalidSyntactic.length,
      typeAware: typeAwareCount,
    });

    describe(ruleName, () => {
      it('carries upstream provenance', () => {
        expect(fixture.__generated.source).toBe('eslint-plugin-functional');
        expect(fixture.__generated.ref).toBeTruthy();
      });

      // Guard against empty suites: rules whose upstream cases are entirely
      // type-aware have no syntactic cases to register (Vitest fails on an
      // empty describe block).
      if (validSyntactic.length > 0) {
        describe('valid', () => {
          validSyntactic.forEach((testCase, index) => {
            it(label(testCase, index), () => {
              const reports = runRule(ruleName, testCase);
              if (full) {
                expect(reports).toEqual([]);
              } else {
                expect(Array.isArray(reports)).toBe(true);
              }
            });
          });
        });
      }

      if (invalidSyntactic.length > 0) {
        describe('invalid', () => {
          invalidSyntactic.forEach((testCase, index) => {
            it(label(testCase, index), () => {
              const reports = runRule(ruleName, testCase);
              if (full) {
                assertErrors(reports, testCase.errors);
              } else {
                expect(Array.isArray(reports)).toBe(true);
              }
            });
          });
        });
      }
    });
  }

  it('parity ledger (type-aware cases captured but skipped)', () => {
    for (const entry of ledger) {
      console.log(
        `[functional] ${entry.ruleName}: parity=${entry.parity} ` +
          `syntactic=${entry.syntactic} type-aware-skipped=${entry.typeAware}`,
      );
    }
    expect(ledger).toHaveLength(plugin.implementedFunctionalRuleNames.length);
  });
});
