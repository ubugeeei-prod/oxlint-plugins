// Generic parity replay runner.
//
// Loads a committed corpus and drives a ported oxlint rule through `RuleTester` from
// `oxlint/plugins-dev`. The caller injects `RuleTester` (so it resolves from the port
// package that depends on oxlint) and the rule object under test.
//
// Comparison policy:
//   - Expected diagnostics assert `message` (the rendered upstream text, which embeds all
//     interpolated data and is the most discriminating single key) plus `line`/`endLine`/
//     `endColumn`. We assert `column` too unless the divergences ledger suppresses it.
//   - `fixOutput` (when present) is fed as RuleTester's `output`; RuleTester applies the
//     port's fixer and asserts the resulting source string.
//   - `outOfScope` cases (non-JS language / custom parser) are skipped and counted.

import { readFileSync } from 'node:fs';

export function loadCorpus(corpusPath) {
  return JSON.parse(readFileSync(corpusPath, 'utf8'));
}

export function loadLedgerEntry(ledgerPath, pluginId, ruleName) {
  let ledger;
  try {
    ledger = JSON.parse(readFileSync(ledgerPath, 'utf8'));
  } catch {
    return null;
  }
  return ledger[`${pluginId}/${ruleName}`] ?? null;
}

function buildExpectedError(e, suppress) {
  // `message` and `messageId` are mutually exclusive in RuleTester; prefer the rendered
  // message because it embeds the interpolated `{{data}}` and is strictly more discriminating.
  const out = { message: e.message };
  if (!suppress.has('column') && typeof e.column === 'number') out.column = e.column;
  if (!suppress.has('line') && typeof e.line === 'number') out.line = e.line;
  if (!suppress.has('endLine') && typeof e.endLine === 'number') out.endLine = e.endLine;
  if (!suppress.has('endColumn') && typeof e.endColumn === 'number') out.endColumn = e.endColumn;
  if (Array.isArray(e.suggestions)) {
    out.suggestions = e.suggestions.map((s) => ({
      ...(s.messageId ? { messageId: s.messageId } : { desc: s.desc }),
      output: s.output,
    }));
  }
  return out;
}

function carryCaseConfig(target, c) {
  if (c.filename) target.filename = c.filename;
  if (c.settings) target.settings = c.settings;
  if (c.languageOptions) target.languageOptions = c.languageOptions;
  if (Array.isArray(c.options) && c.options.length > 0) target.options = c.options;
}

// Matches an inline ESLint disable directive that can suppress a report.
const DISABLE_DIRECTIVE = /\beslint-disable(?:-next-line|-line)?\b/u;

export function buildTestCases(corpus, ledgerEntry) {
  const suppress = new Set(ledgerEntry?.suppressFields ?? []);
  const skipDirectiveValid = ledgerEntry?.skipValidWithDisableDirective === true;
  const valid = [];
  const invalid = [];
  let skipped = 0;

  for (const c of corpus.cases) {
    if (c.outOfScope) {
      skipped++;
      continue;
    }
    // oxlint's plugin RuleTester does not apply inline `eslint-disable*` directives, so a valid
    // case that is valid only because a directive suppresses the report cannot be replayed.
    if (c.kind === 'valid' && skipDirectiveValid && DISABLE_DIRECTIVE.test(c.code)) {
      skipped++;
      continue;
    }
    if (c.kind === 'valid') {
      const tc = { code: c.code };
      carryCaseConfig(tc, c);
      valid.push(tc);
    } else {
      const tc = {
        code: c.code,
        errors: c.expectedErrors.map((e) => buildExpectedError(e, suppress)),
      };
      carryCaseConfig(tc, c);
      if (c.fixOutput != null) tc.output = c.fixOutput;
      // The oracle captures the multi-pass fixpoint (verifyAndFix). RuleTester applies a single
      // fix pass by default, so honor a per-case `recursive` to reproduce convergent fixers.
      if (c.recursive != null) tc.recursive = c.recursive;
      invalid.push(tc);
    }
  }
  return { valid, invalid, skipped };
}

/**
 * Run a corpus against a ported rule. Throws (via RuleTester) on the first mismatch.
 * Returns counts on success.
 */
export function runRuleParity({
  RuleTester,
  rule,
  ruleName,
  corpus,
  ledgerEntry,
  eslintCompat = true,
}) {
  // Make RuleTester execute synchronously instead of registering nested test-runner hooks,
  // so a single outer test owns the assertion and a failure surfaces as a thrown AssertionError.
  RuleTester.describe = (_name, fn) => fn();
  RuleTester.it = (_name, fn) => fn();
  RuleTester.itOnly = (_name, fn) => fn();

  const { valid, invalid, skipped } = buildTestCases(corpus, ledgerEntry);
  const tester = new RuleTester({ eslintCompat });
  tester.run(ruleName, rule, { valid, invalid });
  return { valid: valid.length, invalid: invalid.length, skipped };
}
