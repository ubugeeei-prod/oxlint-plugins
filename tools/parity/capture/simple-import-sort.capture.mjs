// Vitest-hosted capture for eslint-plugin-simple-import-sort.
//
// Its tests are ESM, use `require.resolve(parser)`, vitest snapshot serializers, and
// function-valued `output:` callbacks — none of which load under plain `node require()`.
// Running under vitest lets vite transform the files and provides the real vitest globals
// the upstream helpers need. We monkeypatch `RuleTester.prototype.run` before importing,
// harvest the JavaScript-parser run, and materialize ground truth via the shared oracle.
//
//   pnpm exec vitest run --config tools/parity/capture/vitest.config.ts

import { execFileSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

import { Linter, RuleTester } from 'eslint';
import { expect, test } from 'vitest';

import core from './core.cjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(HERE, '..', '..', '..');

const PLUGIN_ID = 'eslint-plugin-simple-import-sort';
const PINNED_REF = 'v13.0.0';
const DEFAULT_LANGUAGE_OPTIONS = { ecmaVersion: 2020, sourceType: 'module' };

// The JavaScript RuleTester run is labelled "JavaScript"; the Flow/TypeScript runs use
// custom parsers and are out of scope for the oxc replay, so we ignore them here.
const JS_RUN_LABEL = 'JavaScript';

const TEST_FILES = [
  { file: 'upstream/eslint-plugin-simple-import-sort/test/imports.test.js', rule: 'imports' },
  { file: 'upstream/eslint-plugin-simple-import-sort/test/exports.test.js', rule: 'exports' },
];

// Install the run() sink before any upstream test module evaluates.
const RUNS = [];
RuleTester.prototype.run = function (label, rule, cases) {
  RUNS.push({ label, rule, valid: cases.valid || [], invalid: cases.invalid || [] });
};

function submoduleSha() {
  return execFileSync(
    'git',
    [
      '-C',
      path.join(REPO_ROOT, 'upstream', 'eslint-plugin-simple-import-sort'),
      'rev-parse',
      'HEAD',
    ],
    { encoding: 'utf8' },
  ).trim();
}

const sha = submoduleSha();
const eslintVersion = (await import('eslint/package.json', { with: { type: 'json' } })).default
  .version;

test('capture simple-import-sort (imports, exports)', async () => {
  const outRoot = path.join(REPO_ROOT, 'tools', 'parity', 'corpora', PLUGIN_ID);
  mkdirSync(outRoot, { recursive: true });

  for (const { file, rule: ruleName } of TEST_FILES) {
    RUNS.length = 0;
    await import(pathToFileURL(path.join(REPO_ROOT, file)).href);

    const jsRun = RUNS.find((r) => r.label === JS_RUN_LABEL);
    expect(jsRun, `no "${JS_RUN_LABEL}" run captured from ${file}`).toBeTruthy();

    const failures = [];
    const cases = [];
    let fixCount = 0;
    let countOnly = 0;

    for (const [list, kind] of [
      [jsRun.valid, 'valid'],
      [jsRun.invalid, 'invalid'],
    ]) {
      list.forEach((raw, idx) => {
        const c = core.normalizeCase(raw, kind);
        if (c.outOfScope) {
          cases.push(core.buildCorpusCase(c, null));
          return;
        }
        let oracle;
        try {
          oracle = core.runOracle(Linter, jsRun.rule, c, DEFAULT_LANGUAGE_OPTIONS);
        } catch (err) {
          failures.push(`${ruleName} ${kind} #${idx}: oracle error: ${err.message}`);
          return;
        }
        const problem = core.selfValidate(ruleName, idx, c, oracle);
        if (problem) failures.push(problem);
        if (kind === 'invalid' && typeof raw === 'object' && typeof raw.errors === 'number')
          countOnly++;
        if (oracle.fixOutput != null) fixCount++;
        cases.push(core.buildCorpusCase(c, oracle));
      });
    }

    expect(failures, failures.join('\n')).toEqual([]);
    // These two capabilities are exactly what this plugin proves: fixable-rule fix capture
    // and count-only assertions backfilled by the oracle.
    expect(fixCount, 'expected at least one captured autofix output').toBeGreaterThan(0);
    expect(countOnly, 'expected count-only upstream assertions').toBeGreaterThan(0);

    const corpus = {
      corpusVersion: core.CORPUS_VERSION,
      provenance: {
        plugin: PLUGIN_ID,
        rule: ruleName,
        pinnedRef: PINNED_REF,
        submoduleSha: sha,
        eslintVersion,
        license: 'MIT',
        copyright: 'Simon Lydell',
        note: 'Test fixtures converted/derived from upstream; see tools/parity/corpora/NOTICE.',
      },
      cases,
    };
    writeFileSync(path.join(outRoot, `${ruleName}.json`), core.stableStringify(corpus));
  }
});
