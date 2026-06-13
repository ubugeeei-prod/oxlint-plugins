// Replays the upstream eslint-plugin-regexp test suite (captured verbatim into
// test/fixtures/*.json by `pnpm run port:tests:regexp`) against this plugin,
// so behavior stays faithful to upstream as the submodule is bumped.
//
// Design note — diagnostics-only port, ratcheted parity:
//   The native `scanRegexp(sourceText, filename)` API returns raw diagnostics
//   from the Rust/oxc backend. Per-rule enforcement is driven by parity.json
//   (see its `_doc`). Levels:
//     - 'off'   — quarantined: the port diverges from upstream defaults, so the
//                 rule's valid AND invalid cases are skipped until it is fixed.
//     - 'valid' — the default for any rule not listed: every upstream VALID case
//                 must emit zero diagnostics for the rule (soundness).
//     - 'full'  — 'valid' plus invalid-case COUNT parity: the number of
//                 diagnostics per case must match upstream.
//   Message text and span are intentionally NOT asserted: the port rewords
//   messages and may flag a different span (e.g. the whole literal) than
//   upstream by design. The fixtures still record upstream locations as
//   reference data for a possible future strict-location pass.

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { scanRegexp } from '../api.js';

const FIXTURES_DIR = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');
const PARITY_FILE = join(dirname(fileURLToPath(import.meta.url)), 'parity.json');

const fixtureFiles = readdirSync(FIXTURES_DIR).filter((name) => name.endsWith('.json'));
const parityRules = JSON.parse(readFileSync(PARITY_FILE, 'utf8')).rules ?? {};

// Resolve the enforcement level for a rule (default 'valid' when unlisted).
function levelFor(rule) {
  return parityRules[rule]?.level ?? 'valid';
}

function diagnosticsFor(testCase, rule) {
  return scanRegexp(testCase.code, testCase.filename ?? 'file.js').filter(
    (d) => d.ruleName === rule,
  );
}

// Summary logged at startup so CI output makes coverage visible.
const counts = { off: 0, valid: 0, full: 0 };
for (const file of fixtureFiles) {
  counts[levelFor(file.replace(/\.json$/, ''))]++;
}
console.log(
  `regexp upstream parity: ${counts.full} full (count) | ${counts.valid} valid-soundness | ` +
    `${counts.off} quarantined (off)`,
);

for (const file of fixtureFiles) {
  const rule = file.replace(/\.json$/, '');
  const fixture = JSON.parse(readFileSync(join(FIXTURES_DIR, file), 'utf8'));
  const level = levelFor(rule);

  describe(rule, () => {
    describe('valid', () => {
      fixture.valid.forEach((testCase, index) => {
        const label = `#${index} ${JSON.stringify(testCase.code)}`;
        // 'off' rules are quarantined; show the gap as a skip rather than hide it.
        const run = level === 'off' ? it.skip : it;
        run(label, () => {
          expect(diagnosticsFor(testCase, rule)).toEqual([]);
        });
      });
    });

    describe('invalid', () => {
      fixture.invalid.forEach((testCase, index) => {
        const label = `#${index} ${JSON.stringify(testCase.code)}`;
        // Invalid cases are only enforced (count parity) at level 'full'.
        // Everything else is registered as skipped so coverage stays visible.
        if (level !== 'full') {
          it.skip(label, () => {});
          return;
        }
        it(label, () => {
          expect(diagnosticsFor(testCase, rule).length).toBe(testCase.errors.length);
        });
      });
    });
  });
}
