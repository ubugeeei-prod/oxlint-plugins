import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

import { RuleTester } from 'oxlint/plugins-dev';
import { describe, expect, it } from 'vitest';

import {
  loadCorpus,
  loadLedgerEntry,
  runRuleParity,
} from '../../../tools/parity/replay/runner.mjs';

const require = createRequire(import.meta.url);

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(HERE, '..', '..', '..');
const PLUGIN_ID = 'eslint-plugin-simple-import-sort';

const RULES = {
  imports: require('../src/imports.js'),
  exports: require('../src/exports.js'),
};

for (const [ruleName, rule] of Object.entries(RULES)) {
  const corpus = loadCorpus(
    path.join(REPO_ROOT, 'tools', 'parity', 'corpora', PLUGIN_ID, `${ruleName}.json`),
  );
  const ledgerEntry = loadLedgerEntry(
    path.join(REPO_ROOT, 'tools', 'parity', 'divergences.json'),
    PLUGIN_ID,
    ruleName,
  );

  describe(`parity: ${PLUGIN_ID}/${ruleName}`, () => {
    it('reproduces the upstream corpus exactly (minus ledgered divergences)', () => {
      const counts = runRuleParity({ RuleTester, rule, ruleName, corpus, ledgerEntry });
      expect(counts.valid).toBeGreaterThan(0);
      expect(counts.invalid).toBeGreaterThan(0);
    });
  });
}
