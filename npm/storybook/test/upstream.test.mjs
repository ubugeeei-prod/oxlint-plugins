// Replays the upstream eslint-plugin-storybook test suite (captured verbatim into
// test/fixtures/*.json by `pnpm run port:tests:storybook`) against this plugin's
// native scanner, so behaviour stays faithful to upstream as the submodule is
// bumped. This is the guarantee that the port reproduces eslint-plugin-storybook's
// own tests.
//
// For every upstream case:
//   - valid   -> the port must report zero diagnostics (soundness).
//   - invalid -> the multiset of reported messageIds must equal the upstream
//                `errors[].messageId`; every upstream `data` placeholder must be
//                reproduced; and where the upstream case is autofixable (a
//                top-level `output`) or carries a single suggestion, applying the
//                port's fixes must reproduce that output.
//
// Known divergences are quarantined per-case in test/parity.json (with a reason);
// each is a tracked bug whose fix PR removes the entry and enforces the case.
//
// no-uninstalled-addons resolves installed addons from package.json on disk; the
// upstream suite mocks `fs` with a fixed manifest, so we supply the same installed
// set via scan options and ignore the environment-dependent `packageJsonPath`
// placeholder (the addon name — what the rule is about — is still asserted).

import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { implementedStorybookRuleNames, scanStorybook } from '../api.js';

const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = join(TEST_DIR, 'fixtures');
const parity = JSON.parse(readFileSync(join(TEST_DIR, 'parity.json'), 'utf8'));

// devDependencies the upstream no-uninstalled-addons test mocks `fs.readFileSync`
// to return (code/lib/eslint-plugin/src/rules/no-uninstalled-addons.test.ts).
const MOCKED_INSTALLED_ADDONS = Object.freeze([
  '@storybook/addon-essentials',
  '@storybook/addon-interactions',
  '@storybook/preset-create-react-app',
  '@storybook/addon-links',
  'storybook-addon-valid-addon',
  'addon-without-the-prefix',
]);

function scanOptionsFor(ruleName, testCase) {
  const options = { ruleNames: [ruleName] };
  if (ruleName === 'no-uninstalled-addons') {
    options.installedAddons = [...MOCKED_INSTALLED_ADDONS];
    options.packageJsonPath = 'package.json';
    const ruleOptions = Array.isArray(testCase.options) ? testCase.options[0] : null;
    if (ruleOptions && Array.isArray(ruleOptions.ignore)) {
      options.ignoredAddons = ruleOptions.ignore;
    }
  }
  return options;
}

function scan(ruleName, testCase) {
  return scanStorybook(testCase.code, testCase.filename, scanOptionsFor(ruleName, testCase));
}

function applyFixes(code, diagnostics) {
  const fixes = [];
  for (const diagnostic of diagnostics) {
    for (const fix of diagnostic.fixes ?? []) {
      fixes.push(fix);
    }
  }
  fixes.sort((a, b) => b.start - a.start);
  return fixes.reduce(
    (text, fix) => text.slice(0, fix.start) + fix.replacement + text.slice(fix.end),
    code,
  );
}

// The upstream output a faithful autofix should reproduce: the top-level `output`
// when the rule auto-fixes, else the lone suggestion's output when the case has a
// single error with a single suggestion (suggestion-only upstream rules that the
// port implements as an autofix). Otherwise there is nothing single-valued to
// assert.
function expectedFixOutput(testCase) {
  if (typeof testCase.output === 'string' && testCase.output !== testCase.code) {
    return testCase.output;
  }
  if (Array.isArray(testCase.errors) && testCase.errors.length === 1) {
    const suggestions = testCase.errors[0].suggestions;
    if (
      Array.isArray(suggestions) &&
      suggestions.length === 1 &&
      typeof suggestions[0].output === 'string'
    ) {
      return suggestions[0].output;
    }
  }
  return undefined;
}

function dataMatches(ruleName, expectedData, actualData) {
  const data = actualData ?? {};
  return Object.entries(expectedData).every(([key, value]) => {
    // packageJsonPath is the absolute path the rule was pointed at; the upstream
    // suite asserts a mocked cwd-derived path we cannot reproduce here. The addon
    // name (the rule's subject) is still asserted.
    if (ruleName === 'no-uninstalled-addons' && key === 'packageJsonPath') {
      return true;
    }
    return data[key] === value;
  });
}

function label(testCase, index) {
  const code = JSON.stringify(testCase.code);
  const truncated = code.length > 70 ? `${code.slice(0, 70)}…"` : code;
  return `#${index} ${truncated}`;
}

function skipList(ruleName, kind) {
  const entry = parity.skip[ruleName];
  return new Set(entry && Array.isArray(entry[kind]) ? entry[kind] : []);
}

const fixtureFiles = readdirSync(FIXTURES_DIR)
  .filter((name) => name.endsWith('.json'))
  .sort();

describe('eslint-plugin-storybook upstream parity', () => {
  it('has a fixture for every implemented rule', () => {
    const ruleNames = fixtureFiles.map((name) => name.replace(/\.json$/, ''));
    expect(ruleNames).toEqual([...implementedStorybookRuleNames()].sort());
  });

  for (const file of fixtureFiles) {
    const ruleName = file.replace(/\.json$/, '');
    const fixture = JSON.parse(readFileSync(join(FIXTURES_DIR, file), 'utf8'));
    const skipValid = skipList(ruleName, 'valid');
    const skipInvalid = skipList(ruleName, 'invalid');
    const skipFixOutput = skipList(ruleName, 'fixOutput');

    describe(ruleName, () => {
      it('carries upstream provenance', () => {
        expect(fixture.__generated.source).toBe('eslint-plugin-storybook');
        expect(fixture.__generated.ref).toBeTruthy();
      });

      if (fixture.valid.length > 0) {
        describe('valid', () => {
          fixture.valid.forEach((testCase, index) => {
            const test = skipValid.has(index) ? it.skip : it;
            test(label(testCase, index), () => {
              expect(scan(ruleName, testCase)).toEqual([]);
            });
          });
        });
      }

      if (fixture.invalid.length > 0) {
        describe('invalid', () => {
          fixture.invalid.forEach((testCase, index) => {
            const test = skipInvalid.has(index) ? it.skip : it;
            test(label(testCase, index), () => {
              const diagnostics = scan(ruleName, testCase);
              const expected = testCase.errors ?? [];

              const actualIds = diagnostics.map((d) => d.messageId).sort();
              const expectedIds = expected.map((e) => e.messageId).sort();
              expect(actualIds).toEqual(expectedIds);

              for (const error of expected) {
                if (!error.data) {
                  continue;
                }
                const match = diagnostics.some(
                  (d) =>
                    d.messageId === error.messageId && dataMatches(ruleName, error.data, d.data),
                );
                expect(
                  match,
                  `data ${JSON.stringify(error.data)} for ${error.messageId} not reproduced`,
                ).toBe(true);
              }

              const expectedOutput = expectedFixOutput(testCase);
              if (expectedOutput !== undefined && !skipFixOutput.has(index)) {
                expect(applyFixes(testCase.code, diagnostics)).toBe(expectedOutput);
              }
            });
          });
        });
      }
    });
  }
});
