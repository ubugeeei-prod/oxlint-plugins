// Replays the upstream eslint-plugin-functional test suite (captured verbatim
// into test/fixtures/*.json by `pnpm run port:tests:functional`) against this
// plugin, so behavior stays faithful to upstream as the submodule is bumped.
//
// Each fixture case carries a `typeAware` flag derived from the upstream config
// it ran under (the TypeScript `projectService` config vs the syntax-only babel
// config). Behavior depends on the rule's parity level:
//
//   - FULL_PARITY: every upstream case is asserted (valid -> no reports;
//     invalid -> reported messageIds and count match the upstream-declared
//     `errors`). This includes cases that ran under the TypeScript config but
//     need no type information (e.g. TS-only syntax). A rule joins this set in
//     its own porting PR, once the syntax-only port handles ALL of its cases.
//   - otherwise (pending): only the syntactic (non-`typeAware`) cases are
//     smoke-run (must not throw, must return an array) to exercise the upstream
//     corpus through the NAPI binding; the `typeAware` cases — which need type
//     information the syntax-only port lacks — are skipped and counted in the
//     parity ledger (no silent truncation).

import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const FIXTURES_DIR = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');

// Rules whose syntax-only port has reached full upstream parity. Populated one
// entry per porting PR. See the file header for what "full parity" asserts.
const FULL_PARITY = new Set([
  'functional-parameters',
  'no-class-inheritance',
  'no-classes',
  'no-let',
  'no-loop-statements',
  'no-promise-reject',
  'no-this-expressions',
  'no-try-statements',
  'prefer-property-signatures',
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

    // Full-parity rules assert EVERY upstream case, including TypeScript-syntax
    // cases that need no type information (the syntax-only port handles them). A
    // rule only joins FULL_PARITY once it handles all of its cases. Pending rules
    // only smoke-run their syntactic cases; their type-aware cases (which need
    // type information the syntax-only port lacks) are skipped and counted.
    const validCases = full ? fixture.valid : validSyntactic;
    const invalidCases = full ? fixture.invalid : invalidSyntactic;

    ledger.push({
      ruleName,
      parity: fixture.__generated.noUpstreamTests ? 'none' : full ? 'full' : 'smoke',
      asserted: validCases.length + invalidCases.length,
      skipped: full ? 0 : typeAwareCount,
    });

    describe(ruleName, () => {
      it('carries upstream provenance', () => {
        expect(fixture.__generated.source).toBe('eslint-plugin-functional');
        expect(fixture.__generated.ref).toBeTruthy();
      });

      // Guard against empty suites (Vitest fails on a describe block with no
      // tests) — e.g. a pending rule whose upstream cases are all type-aware.
      if (validCases.length > 0) {
        describe('valid', () => {
          validCases.forEach((testCase, index) => {
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

      if (invalidCases.length > 0) {
        describe('invalid', () => {
          invalidCases.forEach((testCase, index) => {
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
          `asserted=${entry.asserted} type-aware-skipped=${entry.skipped}`,
      );
    }
    expect(ledger).toHaveLength(plugin.implementedFunctionalRuleNames.length);
  });
});
