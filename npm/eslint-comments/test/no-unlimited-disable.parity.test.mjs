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
const rule = require('../src/no-unlimited-disable.js');

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(HERE, '..', '..', '..');
const PLUGIN_ID = 'eslint-plugin-eslint-comments';
const RULE_NAME = 'no-unlimited-disable';

const corpus = loadCorpus(
  path.join(REPO_ROOT, 'tools', 'parity', 'corpora', PLUGIN_ID, `${RULE_NAME}.json`),
);
const ledgerEntry = loadLedgerEntry(
  path.join(REPO_ROOT, 'tools', 'parity', 'divergences.json'),
  PLUGIN_ID,
  RULE_NAME,
);

describe(`parity: ${PLUGIN_ID}/${RULE_NAME}`, () => {
  it('reproduces the upstream corpus exactly (minus ledgered divergences)', () => {
    const counts = runRuleParity({ RuleTester, rule, ruleName: RULE_NAME, corpus, ledgerEntry });
    // Sanity: the corpus actually exercised both branches.
    expect(counts.valid).toBeGreaterThan(0);
    expect(counts.invalid).toBeGreaterThan(0);
  });
});
