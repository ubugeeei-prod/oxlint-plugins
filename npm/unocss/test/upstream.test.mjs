// Replays the upstream @unocss/eslint-plugin test suite (captured verbatim into
// test/fixtures/*.json by `pnpm run port:tests:unocss`) against this plugin's
// native scanner, so behaviour stays faithful to upstream as the submodule is
// bumped. This is the guarantee that the port reproduces @unocss/eslint-plugin's
// own tests.
//
// Fixture structure: each JSON file has a top-level `blocks` object whose keys
// are the `name` passed to the upstream `run()` call (e.g. 'order', 'order-jsx',
// 'order-unoFunctions'). Each block carries:
//   `parser`  — 'vue' | 'svelte' | 'jsx' | 'js'
//   `valid`   — array of cases: { code, options?, filename?, parser }
//   `invalid` — array of cases: { code, options?, filename?, output?, errors?, parser }
//
// Parser policy:
//   vue / svelte → always quarantined (Oxlint / the Rust port operates on
//     JS/TS only; Vue and Svelte template scanning is not yet wired through
//     the same NAPI interface).
//   jsx / js → asserted, unless listed in parity.json.
//
// For every asserted case:
//   valid   → the port must report zero diagnostics (soundness).
//   invalid → the multiset of reported messageIds must equal the upstream
//             `errors[].messageId`; where `output` is a non-null string,
//             applying the port's fixes must reproduce it exactly.
//
// Known divergences are quarantined per-case in test/parity.json (with a
// reason); each is a tracked bug whose fix PR removes the entry.

import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = join(TEST_DIR, 'fixtures');
const parity = JSON.parse(readFileSync(join(TEST_DIR, 'parity.json'), 'utf8'));

// Rule name derived from the block name (strip leading 'order-*' qualifier
// suffixes that are not part of the rule name, e.g. 'order-jsx' → 'order').
function ruleNameForBlock(blockName) {
  if (blockName === 'order' || blockName.startsWith('order-')) {
    return 'order';
  }
  return blockName;
}

function runRule(ruleName, testCase) {
  const sourceText = testCase.code;
  const filename = testCase.filename ?? (testCase.parser === 'jsx' ? 'fixture.jsx' : 'fixture.js');
  const options = testCase.options ?? [];
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return sourceText;
    },
  };
  const rule = plugin.rules[ruleName];
  const visitor = rule.createOnce({
    filename,
    options,
    settings: {},
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

function applyFixes(code, reports) {
  const fixes = [];
  for (const report of reports) {
    if (typeof report.fix === 'function') {
      report.fix({
        replaceTextRange(range, replacement) {
          fixes.push({ start: range[0], end: range[1], replacement });
        },
      });
    }
  }
  fixes.sort((a, b) => b.start - a.start);
  return fixes.reduce(
    (text, fix) => text.slice(0, fix.start) + fix.replacement + text.slice(fix.end),
    code,
  );
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

// Returns a Set of indices to skip from parity.json for a given block and kind.
function skipSet(blockName, kind) {
  const entry = parity.quarantine[blockName];
  if (!entry || !Array.isArray(entry[kind])) {
    return new Set();
  }
  return new Set(entry[kind].map((q) => q.index));
}

const fixtureFiles = readdirSync(FIXTURES_DIR)
  .filter((name) => name.endsWith('.json'))
  .sort();

describe('@unocss/eslint-plugin upstream parity', () => {
  for (const file of fixtureFiles) {
    const ruleName = file.replace(/\.json$/, '');
    const fixture = JSON.parse(readFileSync(join(FIXTURES_DIR, file), 'utf8'));

    describe(ruleName, () => {
      it('carries upstream provenance', () => {
        expect(fixture.__generated.source).toBe('@unocss/eslint-plugin');
        expect(fixture.__generated.ref).toBeTruthy();
      });

      if (fixture.__generated.noUpstreamTests) {
        return;
      }

      for (const [blockName, block] of Object.entries(fixture.blocks)) {
        const blockRuleName = ruleNameForBlock(blockName);
        const parser = block.parser;
        // Vue and Svelte cases are always quarantined — the Rust port does not
        // process template-based languages through the same NAPI interface.
        const parserQuarantined = parser === 'vue' || parser === 'svelte';

        const skipValid = skipSet(blockName, 'valid');
        const skipInvalid = skipSet(blockName, 'invalid');

        describe(blockName, () => {
          if (block.valid.length > 0) {
            describe('valid', () => {
              block.valid.forEach((testCase, index) => {
                const quarantined = parserQuarantined || skipValid.has(index);
                const run = quarantined ? it.skip : it;
                run(label(testCase, index), () => {
                  const reports = runRule(blockRuleName, testCase);
                  expect(reports).toEqual([]);
                });
              });
            });
          }

          if (block.invalid.length > 0) {
            describe('invalid', () => {
              block.invalid.forEach((testCase, index) => {
                const quarantined = parserQuarantined || skipInvalid.has(index);
                const run = quarantined ? it.skip : it;
                run(label(testCase, index), () => {
                  const reports = runRule(blockRuleName, testCase);

                  const expectedErrors = testCase.errors ?? [];
                  const actualIds = reports.map((r) => r.messageId).sort();
                  const expectedIds = expectedErrors.map((e) => e.messageId).sort();
                  expect(actualIds).toEqual(expectedIds);

                  if (typeof testCase.output === 'string') {
                    expect(applyFixes(testCase.code, reports)).toBe(testCase.output);
                  } else if (testCase.output === null) {
                    // Upstream asserts no fix; we assert no fix was applied.
                    expect(applyFixes(testCase.code, reports)).toBe(testCase.code);
                  }
                });
              });
            });
          }
        });
      }
    });
  }
});
