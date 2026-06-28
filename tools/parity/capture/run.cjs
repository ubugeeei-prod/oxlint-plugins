#!/usr/bin/env node
'use strict';

/**
 * Parity capture + oracle harness (CJS upstreams, plain `require()`-loadable tests).
 *
 *   node tools/parity/capture/run.cjs <plugin-id> [--check]
 *
 * Layer A (capture): monkeypatch `RuleTester.prototype.run` and import each upstream rule
 * test file. By the time `run()` is called every dynamic case (semver gates, spreads,
 * builders) is already resolved, so we harvest the exact `{valid, invalid}` set.
 *
 * Layer B (oracle): execute the real upstream rule through ESLint's `Linter` to materialize
 * authoritative diagnostics, then self-validate against the upstream test's own assertion.
 *
 * Output: one deterministic JSON corpus per rule under tools/parity/corpora/<plugin-id>/.
 * `--check` writes to a scratch dir and diffs against the committed copies (CI drift gate).
 *
 * ESM/transform-required upstreams (simple-import-sort, functional, …) cannot be loaded by
 * plain require(); they are captured under vitest instead — see capture/*.capture.mjs and
 * the shared primitives in capture/core.cjs.
 */

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const Module = require('node:module');
const { execFileSync } = require('node:child_process');

const REPO_ROOT = path.resolve(__dirname, '..', '..', '..');
const { PLUGINS } = require('./plugins.cjs');
const {
  normalizeCase,
  runOracle,
  selfValidate,
  buildCorpusCase,
  stableStringify,
  CORPUS_VERSION,
} = require('./core.cjs');

function fail(msg) {
  console.error(`\n✖ ${msg}`);
  process.exit(1);
}

function installModuleStubs(ids) {
  const stubbed = new Set(ids);
  const origLoad = Module._load;
  Module._load = function (request, parent, isMain) {
    if (stubbed.has(request)) return { default: { meta: { name: `parity-stub:${request}` } } };
    return origLoad.call(this, request, parent, isMain);
  };
}

function captureFile(absTestFile, RuleTester) {
  const runs = [];
  const original = RuleTester.prototype.run;
  RuleTester.prototype.run = function (ruleName, rule, cases) {
    runs.push({ ruleName, rule, valid: cases.valid || [], invalid: cases.invalid || [] });
  };
  try {
    require(absTestFile);
  } finally {
    RuleTester.prototype.run = original;
  }
  return runs;
}

function submoduleSha(submodule) {
  return execFileSync('git', ['-C', path.join(REPO_ROOT, submodule), 'rev-parse', 'HEAD'], {
    encoding: 'utf8',
  }).trim();
}

function listTestFiles(testGlob) {
  const dir = path.join(REPO_ROOT, path.dirname(testGlob));
  const ext = path.extname(testGlob);
  return fs
    .readdirSync(dir)
    .filter((f) => f.endsWith(ext))
    .sort()
    .map((f) => path.join(dir, f));
}

function main() {
  const pluginId = process.argv[2];
  const check = process.argv.includes('--check');
  if (!pluginId || !PLUGINS[pluginId]) {
    fail(
      `usage: node tools/parity/capture/run.cjs <plugin-id> [--check]\n  known: ${Object.keys(PLUGINS).join(', ')}`,
    );
  }
  const cfg = PLUGINS[pluginId];

  installModuleStubs(cfg.stubModules || []);
  const { RuleTester, Linter } = require('eslint');
  const eslintVersion = require('eslint/package.json').version;
  const sha = submoduleSha(cfg.submodule);

  const outRoot = check
    ? fs.mkdtempSync(path.join(os.tmpdir(), 'parity-check-'))
    : path.join(REPO_ROOT, 'tools', 'parity', 'corpora', pluginId);
  fs.mkdirSync(outRoot, { recursive: true });

  const excluded = new Set(cfg.excludeFiles || []);
  const files = listTestFiles(cfg.testGlob);
  const failures = [];
  const written = [];
  let totalCases = 0;
  let outOfScope = 0;

  for (const file of files) {
    if (excluded.has(path.basename(file))) continue;
    let runs;
    try {
      runs = captureFile(file, RuleTester);
    } catch (err) {
      failures.push(`capture import failed for ${path.basename(file)}: ${err.message}`);
      continue;
    }
    for (const run of runs) {
      const cases = [];
      for (const [list, kind] of [
        [run.valid, 'valid'],
        [run.invalid, 'invalid'],
      ]) {
        list.forEach((raw, idx) => {
          const c = normalizeCase(raw, kind);
          if (c.outOfScope) {
            outOfScope++;
            cases.push(buildCorpusCase(c, null));
            return;
          }
          let oracle;
          try {
            oracle = runOracle(Linter, run.rule, c);
          } catch (err) {
            failures.push(`${run.ruleName} ${kind} #${idx}: oracle error: ${err.message}`);
            return;
          }
          const problem = selfValidate(run.ruleName, idx, c, oracle);
          if (problem) failures.push(problem);
          cases.push(buildCorpusCase(c, oracle));
          totalCases++;
        });
      }

      const corpus = {
        corpusVersion: CORPUS_VERSION,
        provenance: {
          plugin: pluginId,
          rule: run.ruleName,
          pinnedRef: cfg.pinnedRef,
          submoduleSha: sha,
          eslintVersion,
          license: cfg.license,
          copyright: cfg.copyright,
          note: 'Test fixtures converted/derived from upstream; see tools/parity/corpora/NOTICE.',
        },
        cases,
      };
      const outFile = path.join(outRoot, `${run.ruleName}.json`);
      fs.writeFileSync(outFile, stableStringify(corpus));
      written.push({ rule: run.ruleName, file: outFile, cases: cases.length });
    }
  }

  if (failures.length > 0) {
    console.error(`\n✖ self-validation failed (${failures.length}):`);
    for (const f of failures) console.error(`  - ${f}`);
    fail('aborting: refusing to freeze an unverified corpus');
  }

  if (check) {
    const committedRoot = path.join(REPO_ROOT, 'tools', 'parity', 'corpora', pluginId);
    const diffs = [];
    for (const w of written) {
      const committed = path.join(committedRoot, `${w.rule}.json`);
      const fresh = fs.readFileSync(w.file, 'utf8');
      if (!fs.existsSync(committed)) diffs.push(`missing committed corpus: ${w.rule}`);
      else if (fs.readFileSync(committed, 'utf8') !== fresh) diffs.push(`corpus drift: ${w.rule}`);
    }
    if (diffs.length) {
      console.error('\n✖ corpus --check drift:');
      for (const d of diffs) console.error(`  - ${d}`);
      fail('committed corpora are stale; regenerate with the capture harness');
    }
    console.log(`✔ ${pluginId}: ${written.length} corpora match committed (check mode)`);
    return;
  }

  console.log(`✔ ${pluginId} captured`);
  console.log(`  rules:        ${written.length}`);
  console.log(`  cases:        ${totalCases} (+${outOfScope} out-of-scope, skipped)`);
  console.log(`  eslint:       ${eslintVersion}`);
  console.log(`  submodule:    ${cfg.pinnedRef} (${sha.slice(0, 10)})`);
  console.log(`  written to:   tools/parity/corpora/${pluginId}/`);
}

main();
