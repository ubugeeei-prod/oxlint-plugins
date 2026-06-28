'use strict';

// Shared capture/oracle primitives, used by both the CJS CLI (run.cjs) and the
// vitest-hosted capture used for ESM/transform-required upstreams (e.g. simple-import-sort).

const JS_LANGS = new Set(['js', 'jsx', 'ts', 'tsx', 'dts', undefined, null]);

/** Turn a raw RuleTester case into a plain, serializable shape; flag non-replayable cases. */
function normalizeCase(raw, kind) {
  if (typeof raw === 'string') {
    return { kind, code: raw, options: [] };
  }
  const out = {
    kind,
    code: raw.code,
    options: Array.isArray(raw.options) ? raw.options : [],
  };
  if (raw.filename) out.filename = raw.filename;
  if (raw.settings) out.settings = raw.settings;
  if (raw.languageOptions) out.languageOptions = raw.languageOptions;
  if (raw.parserOptions) out.parserOptions = raw.parserOptions;
  if (kind === 'invalid') out._errors = raw.errors;

  const langId = typeof raw.language === 'string' ? raw.language.split('/')[0] : raw.language;
  if (raw.language && !JS_LANGS.has(langId)) {
    out.outOfScope = { reason: `non-JS language: ${raw.language}` };
  } else if (raw.parser) {
    out.outOfScope = { reason: 'custom parser' };
  } else if (raw.plugins && Object.keys(raw.plugins).length > 0) {
    out.outOfScope = { reason: 'requires extra eslint plugins/languages' };
  }
  return out;
}

function applyFix(code, fix) {
  return code.slice(0, fix.range[0]) + fix.text + code.slice(fix.range[1]);
}

function makeConfig(rule, c, defaultLanguageOptions) {
  return [
    {
      linterOptions: { reportUnusedDisableDirectives: 'off' },
      languageOptions: c.languageOptions ||
        defaultLanguageOptions || { ecmaVersion: 'latest', sourceType: 'script' },
      plugins: { __p: { rules: { __r: rule } } },
      rules: { '__p/__r': ['error', ...c.options] },
    },
  ];
}

/** Run the real upstream rule and return filtered, materialized diagnostics + applied fix. */
function runOracle(Linter, rule, c, defaultLanguageOptions) {
  const linter = new Linter({ configType: 'flat' });
  const config = makeConfig(rule, c, defaultLanguageOptions);
  const filename = c.filename || 'file.js';
  const messages = linter.verify(c.code, config, { filename });

  const fatal = messages.find((m) => m.fatal);
  if (fatal) throw new Error(`parse error in case: ${fatal.message}`);

  const mine = messages.filter((m) => m.ruleId === '__p/__r');
  const expectedErrors = mine.map((m) => {
    const e = {
      messageId: m.messageId ?? null,
      message: m.message,
      line: m.line,
      column: m.column,
      endLine: m.endLine ?? null,
      endColumn: m.endColumn ?? null,
    };
    if (Array.isArray(m.suggestions) && m.suggestions.length > 0) {
      e.suggestions = m.suggestions.map((s) => ({
        messageId: s.messageId ?? null,
        desc: s.desc ?? null,
        output: applyFix(c.code, s.fix),
      }));
    }
    return e;
  });

  let fixOutput = null;
  if (c.kind === 'invalid' && rule.meta && rule.meta.fixable) {
    const fixed = linter.verifyAndFix(c.code, config, { filename });
    if (fixed.fixed) fixOutput = fixed.output;
  }

  return { expectedErrors, fixOutput };
}

function classifyAssertion(errors) {
  if (typeof errors === 'number') return 'count';
  if (!Array.isArray(errors)) return 'unknown';
  if (errors.every((e) => typeof e === 'string')) return 'plain-string';
  if (errors.some((e) => e && typeof e === 'object' && e.messageId)) return 'messageId';
  return 'location';
}

/** The oracle must agree with whatever the upstream test itself asserted, or we abort. */
function selfValidate(ruleName, idx, c, oracle) {
  const got = oracle.expectedErrors;
  if (c.kind === 'valid') {
    if (got.length !== 0)
      return `valid case #${idx} of ${ruleName} produced ${got.length} oracle diagnostic(s)`;
    return null;
  }
  const expected = c._errors;
  if (typeof expected === 'number') {
    if (got.length !== expected)
      return `${ruleName} invalid #${idx}: upstream asserts ${expected} error(s), oracle produced ${got.length}`;
    return null;
  }
  if (!Array.isArray(expected)) return `${ruleName} invalid #${idx}: unsupported errors assertion`;
  if (got.length !== expected.length)
    return `${ruleName} invalid #${idx}: upstream asserts ${expected.length} error(s), oracle produced ${got.length}`;
  for (let i = 0; i < expected.length; i++) {
    const exp = expected[i];
    const g = got[i];
    if (typeof exp === 'string') {
      if (g.message !== exp)
        return `${ruleName} invalid #${idx} error ${i}: message mismatch\n    upstream: ${exp}\n    oracle:   ${g.message}`;
      continue;
    }
    for (const key of ['messageId', 'message', 'line', 'column', 'endLine', 'endColumn']) {
      if (exp[key] !== undefined && exp[key] !== g[key])
        return `${ruleName} invalid #${idx} error ${i}: ${key} mismatch (upstream ${JSON.stringify(exp[key])} vs oracle ${JSON.stringify(g[key])})`;
    }
  }
  return null;
}

function buildCorpusCase(c, oracle) {
  if (c.outOfScope) {
    return { kind: c.kind, code: c.code, outOfScope: c.outOfScope };
  }
  if (c.kind === 'valid') {
    const out = {
      kind: 'valid',
      code: c.code,
      options: c.options,
      expectedErrors: [],
      fixOutput: null,
    };
    if (c.filename) out.filename = c.filename;
    if (c.settings) out.settings = c.settings;
    if (c.languageOptions) out.languageOptions = c.languageOptions;
    return out;
  }
  const out = {
    kind: 'invalid',
    code: c.code,
    options: c.options,
    expectedErrors: oracle.expectedErrors,
    fixOutput: oracle.fixOutput,
    upstreamAssertion: { style: classifyAssertion(c._errors) },
  };
  if (c.filename) out.filename = c.filename;
  if (c.settings) out.settings = c.settings;
  if (c.languageOptions) out.languageOptions = c.languageOptions;
  return out;
}

function stableStringify(obj) {
  return JSON.stringify(obj, null, 2) + '\n';
}

/**
 * Neutralize a RuleTester's test-runner hooks so it executes synchronously and never
 * registers nested describe/it/afterAll callbacks. Required for `@typescript-eslint/rule-tester`,
 * whose constructor *reads* `afterAll` and throws if it is missing. Safe for the core ESLint
 * RuleTester too. Call before constructing the tester / importing the test file.
 */
function installRuleTesterHooks(RuleTester) {
  RuleTester.afterAll = () => {};
  RuleTester.describe = (_name, fn) => (fn ? fn() : undefined);
  RuleTester.it = (_name, fn) => (fn ? fn() : undefined);
  RuleTester.itOnly = (_name, fn) => (fn ? fn() : undefined);
}

module.exports = {
  JS_LANGS,
  normalizeCase,
  applyFix,
  makeConfig,
  runOracle,
  classifyAssertion,
  selfValidate,
  buildCorpusCase,
  stableStringify,
  installRuleTesterHooks,
  CORPUS_VERSION: 1,
};
