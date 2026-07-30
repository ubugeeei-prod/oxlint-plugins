// Captures every authored stable @stylistic/jsx-curly-brace-presence
// v5.10.0 RuleTester case from the exact pinned upstream commit. The published
// rule is then replayed with pinned ESLint parsers so the committed fixture
// includes exact messages, ranges, first-pass fixes, and recursive output.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = { name: string; valid: RawCase[]; invalid: RawCase[] };
type NormalizedCase = {
  suite: string;
  code: string;
  options: unknown[];
  features: string[];
  [key: string]: unknown;
};

const ROOT = process.cwd();
const RULE = 'jsx-curly-brace-presence';
const VERSION = '5.10.0';
const COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '9.39.2';
const TYPESCRIPT_ESLINT_VERSION = '8.60.0';
const BABEL_ESLINT_VERSION = '7.28.6';
const BABEL_CORE_VERSION = '7.28.0';
const UPSTREAM = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const RULE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.ts`;
const FIXTURE = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures', `${RULE}-v${VERSION}.json`);
const CAPTURE_KEY = '__stylisticJsxCurlyBracePresenceCapture__';

if (!existsSync(join(UPSTREAM, '.git'))) {
  throw new Error(`Missing ${UPSTREAM}; initialize upstream/eslint-stylistic.`);
}
const actualCommit = execFileSync('git', ['-C', UPSTREAM, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== COMMIT) {
  throw new Error(`Expected eslint-stylistic ${COMMIT}, received ${actualCommit}.`);
}
const ruleSource = upstreamFile(RULE_FILE);
for (const expected of [
  "description: 'Disallow unnecessary JSX expressions when literals alone are sufficient or enforce JSX expressions on literals in JSX children or attributes'",
  "fixable: 'code'",
  "defaultOptions: [{ props: 'never', children: 'never', propElementValues: 'ignore' }]",
  "unnecessaryCurly: 'Curly braces are unnecessary here.'",
  "missingCurly: 'Need to wrap this literal in a JSX expression.'",
]) {
  if (!ruleSource.includes(expected)) {
    throw new Error(`Pinned ${RULE_FILE} no longer contains ${JSON.stringify(expected)}.`);
  }
}

registerCaptureHooks();
const captureDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-curly-brace-presence-capture-'));
const captureFile = join(captureDir, `${RULE}.test.ts`);
writeFileSync(captureFile, upstreamFile(SOURCE_FILE));
(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
try {
  await import(`${pathToFileURL(captureFile).href}?commit=${COMMIT}`);
} finally {
  rmSync(captureDir, { recursive: true, force: true });
}

const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
if (runs.length !== 2 || runs[0]?.name !== RULE || runs[1]?.name !== `${RULE}_babel`) {
  throw new Error(`Expected the main and Babel ${RULE} suites, received ${runs.length}.`);
}

const authored = {
  valid: runs.flatMap((run) =>
    run.valid.map((value, index) => normalizeCase(run.name, value, false, index)),
  ),
  invalid: runs.flatMap((run) =>
    run.invalid.map((value, index) => normalizeCase(run.name, value, true, index)),
  ),
};
const enriched = enrichWithPublishedRule(authored);
const authoredDiagnostics = authored.invalid.reduce(
  (count, testCase) =>
    count +
    (typeof testCase.authoredErrors === 'number'
      ? testCase.authoredErrors
      : (testCase.authoredErrors as unknown[]).length),
  0,
);
const exactDiagnostics = enriched.invalid.reduce(
  (count, testCase) =>
    count + (testCase.expectedDiagnostics as Array<Record<string, unknown>>).length,
  0,
);
const parserExpandedValid = enriched.valid.reduce(
  (count, testCase) => count + (testCase.parsers as string[]).length,
  0,
);
const parserExpandedInvalid = enriched.invalid.reduce(
  (count, testCase) => count + (testCase.parsers as string[]).length,
  0,
);
const parserExpandedDiagnostics = enriched.invalid.reduce(
  (count, testCase) =>
    count +
    (testCase.parsers as string[]).length *
      (testCase.expectedDiagnostics as Array<Record<string, unknown>>).length,
  0,
);
const fixableInvalid = enriched.invalid.filter(
  (testCase) => typeof testCase.firstPassOutput === 'string',
).length;
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: `v${VERSION}`,
    commit: COMMIT,
    sourceFile: SOURCE_FILE,
    ruleFile: RULE_FILE,
    license: 'MIT',
    eslintVersion: ESLINT_VERSION,
    typescriptEslintParserVersion: TYPESCRIPT_ESLINT_VERSION,
    babelEslintParserVersion: BABEL_ESLINT_VERSION,
    parserMatrix: 'authored semantic cases replayed with Oxc-compatible JSX and TSX',
    tool: 'tools/tasks/sync-stylistic-jsx-curly-brace-presence-tests.ts',
    inventory: {
      authoredValid: enriched.valid.length,
      authoredInvalid: enriched.invalid.length,
      authoredDiagnostics,
      exactDiagnostics,
      fixableInvalid,
      unfixableInvalid: enriched.invalid.length - fixableInvalid,
      authoredTotal: enriched.valid.length + enriched.invalid.length,
      parserExpandedValid,
      parserExpandedInvalid,
      parserExpandedDiagnostics,
      parserExpandedTotal: parserExpandedValid + parserExpandedInvalid,
    },
  },
  valid: enriched.valid,
  invalid: enriched.invalid,
};

mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
writeFileSync(FIXTURE, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', FIXTURE], { cwd: ROOT, stdio: 'inherit' });
console.log(
  `Synced ${RULE} v${VERSION}: ${enriched.valid.length} valid, ` +
    `${enriched.invalid.length} invalid, ${exactDiagnostics} exact diagnostics; ` +
    `${parserExpandedValid + parserExpandedInvalid} Oxc parser-expanded cases.`,
);

function upstreamFile(path: string): string {
  return execFileSync('git', ['-C', UPSTREAM, 'show', `${COMMIT}:${path}`], {
    encoding: 'utf8',
  });
}

function normalizeCase(
  suite: string,
  raw: RawCase,
  invalid: boolean,
  index: number,
): NormalizedCase {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`${suite} ${invalid ? 'invalid' : 'valid'} case ${index} has no code.`);
  }
  const allowed = new Set(
    invalid
      ? [
          'code',
          'output',
          'errors',
          'options',
          'parser',
          'parserOptions',
          'settings',
          'languageOptions',
          'features',
        ]
      : ['code', 'options', 'parser', 'parserOptions', 'settings', 'languageOptions', 'features'],
  );
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported ${suite} case ${index} keys: ${unsupported.join(', ')}`);
  }
  const normalized: NormalizedCase = {
    suite,
    code: value.code,
    options: Array.isArray(value.options) ? clone(value.options) : [],
    features: Array.isArray(value.features) ? clone(value.features) : [],
    ...('parserOptions' in value ? { parserOptions: clone(value.parserOptions) } : {}),
    ...('settings' in value ? { settings: clone(value.settings) } : {}),
  };
  if (!invalid) {
    return normalized;
  }
  if (!Array.isArray(value.errors) && typeof value.errors !== 'number') {
    throw new Error(`${suite} invalid case ${index} is missing errors.`);
  }
  return {
    ...normalized,
    authoredErrors: clone(value.errors),
    ...('output' in value ? { authoredOutput: value.output } : {}),
  };
}

function enrichWithPublishedRule(cases: { valid: NormalizedCase[]; invalid: NormalizedCase[] }): {
  valid: NormalizedCase[];
  invalid: NormalizedCase[];
} {
  const runnerDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-curly-brace-presence-upstream-'));
  try {
    writeFileSync(
      join(runnerDir, 'package.json'),
      `${JSON.stringify(
        {
          private: true,
          type: 'module',
          dependencies: {
            '@babel/core': BABEL_CORE_VERSION,
            '@babel/eslint-parser': BABEL_ESLINT_VERSION,
            '@stylistic/eslint-plugin': VERSION,
            '@typescript-eslint/parser': TYPESCRIPT_ESLINT_VERSION,
            eslint: ESLINT_VERSION,
          },
        },
        null,
        2,
      )}\n`,
    );
    writeFileSync(join(runnerDir, 'captured.json'), `${JSON.stringify(cases)}\n`);
    writeFileSync(join(runnerDir, 'runner.mjs'), enrichmentRunnerSource());
    execFileSync(
      'pnpm',
      ['install', '--dir', runnerDir, '--ignore-workspace', '--lockfile=false', '--silent'],
      { stdio: 'inherit' },
    );
    execFileSync('node', [join(runnerDir, 'runner.mjs')], { stdio: 'inherit' });
    return JSON.parse(readFileSync(join(runnerDir, 'enriched.json'), 'utf8')) as {
      valid: NormalizedCase[];
      invalid: NormalizedCase[];
    };
  } finally {
    rmSync(runnerDir, { recursive: true, force: true });
  }
}

function enrichmentRunnerSource(): string {
  return `
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import babelParser from '@babel/eslint-parser';
import { Linter } from 'eslint';
import tsParser from '@typescript-eslint/parser';
import * as stylisticModule from '@stylistic/eslint-plugin';

const here = fileURLToPath(new URL('.', import.meta.url));
const captured = JSON.parse(readFileSync(join(here, 'captured.json'), 'utf8'));
const stylistic = stylisticModule.default ?? stylisticModule;
const rule = stylistic.rules['${RULE}'];

function parsersFor(testCase) {
  const features = new Set(testCase.features);
  if (testCase.suite.endsWith('_babel') || features.has('no-default')) return ['babel-jsx'];
  if (features.has('ts') || features.has('types')) return ['tsx'];
  if (features.has('no-ts') || features.has('no-ts-new')) return ['babel-jsx'];
  return ['jsx', 'tsx'];
}

function configFor(testCase, parser) {
  const parserOptions = {
    ecmaVersion: 'latest',
    sourceType: 'module',
    ecmaFeatures: { jsx: true },
    ...(testCase.parserOptions ?? {}),
  };
  return [{
    files: ['**/*.{js,jsx,ts,tsx}'],
    languageOptions: {
      ...(parser === 'tsx'
        ? { parser: tsParser }
        : parser === 'babel-jsx'
          ? { parser: babelParser }
          : {}),
      parserOptions: parser === 'babel-jsx'
        ? {
            ...parserOptions,
            requireConfigFile: false,
            babelOptions: { parserOpts: { plugins: ['jsx'] } },
          }
        : parserOptions,
    },
    plugins: {
      stylistic: { rules: { '${RULE}': rule } },
    },
    rules: {
      'stylistic/${RULE}': ['error', ...testCase.options],
    },
    settings: testCase.settings ?? {},
  }];
}

function filenameFor(parser) {
  return parser === 'tsx' ? 'fixture.tsx' : 'fixture.jsx';
}

function verify(testCase, parser) {
  const messages = new Linter().verify(testCase.code, configFor(testCase, parser), {
    filename: filenameFor(parser),
  });
  const fatal = messages.find(message => message.fatal);
  if (fatal) {
    throw new Error(
      'Parser ' + parser + ' rejected ' + testCase.suite + ': ' + JSON.stringify(fatal),
    );
  }
  return messages;
}

function offsetAt(source, line, column) {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line) {
    const match = /\\r\\n|[\\n\\r\\u2028\\u2029]/u.exec(source.slice(offset));
    if (!match) throw new Error('Cannot map diagnostic location');
    offset += match.index + match[0].length;
    currentLine += 1;
  }
  return offset + column - 1;
}

function normalizeMessage(source, message) {
  return {
    messageId: message.messageId,
    message: message.message,
    data: {},
    range: [
      offsetAt(source, message.line, message.column),
      offsetAt(source, message.endLine, message.endColumn),
    ],
    loc: {
      line: message.line,
      column: message.column,
      endLine: message.endLine,
      endColumn: message.endColumn,
    },
    fix: message.fix ? { range: message.fix.range, text: message.fix.text } : null,
  };
}

function applyFixes(source, messages) {
  const fixes = messages
    .map((message, index) => ({ index, fix: message.fix }))
    .filter(({ fix }) => fix)
    .sort((left, right) =>
      left.fix.range[0] - right.fix.range[0] ||
      left.fix.range[1] - right.fix.range[1] ||
      left.index - right.index
    );
  if (fixes.length === 0) return null;
  const accepted = [];
  let lastEnd = -1;
  for (const { fix } of fixes) {
    if (lastEnd >= fix.range[0]) continue;
    accepted.push(fix);
    lastEnd = fix.range[1];
  }
  let output = source;
  for (const fix of accepted.reverse()) {
    output = output.slice(0, fix.range[0]) + fix.text + output.slice(fix.range[1]);
  }
  return output;
}

function assertAuthoredErrors(testCase, messages, index) {
  if (typeof testCase.authoredErrors === 'number') {
    if (messages.length !== testCase.authoredErrors) {
      throw new Error(
        'Invalid case ' + index + ' expected ' + testCase.authoredErrors +
        ' errors, received ' + messages.length,
      );
    }
    return;
  }
  const expectedIds = testCase.authoredErrors.map(error => error.messageId);
  const actualIds = messages.map(message => message.messageId);
  if (JSON.stringify(expectedIds) !== JSON.stringify(actualIds)) {
    const filteredEverywhere =
      testCase.features.includes('no-default') &&
      testCase.features.includes('no-ts-new') &&
      testCase.features.includes('no-babel-new');
    if (!filteredEverywhere) {
      throw new Error(
        'Invalid case ' + index + ' IDs differ: expected ' +
        JSON.stringify(expectedIds) + ', received ' + JSON.stringify(actualIds),
      );
    }
  }
}

const valid = captured.valid.map((testCase, index) => {
  const parsers = parsersFor(testCase);
  for (const parser of parsers) {
    const messages = verify(testCase, parser);
    if (messages.length !== 0) {
      throw new Error(
        'Published rule reported valid case ' + index + ' for ' + parser + ': ' +
        JSON.stringify(messages),
      );
    }
  }
  const { features, ...rest } = testCase;
  return { ...rest, parsers };
});

const invalid = captured.invalid.map((testCase, index) => {
  const parsers = parsersFor(testCase);
  const runs = parsers.map(parser => verify(testCase, parser));
  for (const messages of runs) assertAuthoredErrors(testCase, messages, index);
  const canonical = runs[0];
  const expectedDiagnostics = canonical.map(message => normalizeMessage(testCase.code, message));
  for (let parserIndex = 1; parserIndex < runs.length; parserIndex++) {
    const normalized = runs[parserIndex].map(message =>
      normalizeMessage(testCase.code, message),
    );
    if (JSON.stringify(normalized) !== JSON.stringify(expectedDiagnostics)) {
      throw new Error(
        'Parser diagnostics differ for invalid case ' + index + ': ' +
        parsers[0] + ' vs ' + parsers[parserIndex],
      );
    }
  }

  const firstPassOutput = applyFixes(testCase.code, canonical);
  const fixed = new Linter().verifyAndFix(
    testCase.code,
    configFor(testCase, parsers[0]),
    { filename: filenameFor(parsers[0]) },
  );
  const recursiveOutput = firstPassOutput === null ? null : fixed.output;
  const filteredEverywhere =
    testCase.features.includes('no-default') &&
    testCase.features.includes('no-ts-new') &&
    testCase.features.includes('no-babel-new');
  if (
    !filteredEverywhere &&
    typeof testCase.authoredOutput === 'string' &&
    firstPassOutput !== testCase.authoredOutput
  ) {
    throw new Error(
      'First-pass output differs for invalid case ' + index +
      ': expected ' + JSON.stringify(testCase.authoredOutput) +
      ', received ' + JSON.stringify(firstPassOutput),
    );
  }
  const { features, authoredErrors, authoredOutput, ...rest } = testCase;
  return {
    ...rest,
    parsers,
    authoredErrors,
    ...(authoredOutput !== undefined ? { output: authoredOutput } : {}),
    firstPassOutput,
    recursiveOutput,
    expectedDiagnostics,
  };
});

writeFileSync(join(here, 'enriched.json'), JSON.stringify({ valid, invalid }));
`;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export const skipBabel = false;',
    'export function run(options) {',
    '  globalThis[key].push({ name: options.name, valid: options.valid || [], invalid: options.invalid || [] });',
    '}',
  ].join('\n');
  const parserStub = [
    'function authored(tests) { return tests.flat(Infinity).filter(Boolean); }',
    'export const BABEL_ESLINT = "__babel_eslint_parser__";',
    'export function babelParserOptions() { return {}; }',
    'export function valids(...tests) { return authored(tests); }',
    'export function invalids(...tests) { return authored(tests); }',
  ].join('\n');
  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///stylistic-test', shortCircuit: true };
      }
      if (specifier === '#test/parsers-jsx') {
        return { url: 'stub:///stylistic-parsers-jsx', shortCircuit: true };
      }
      if (specifier === `./${RULE}` || specifier === './types' || specifier === './types.d.ts') {
        return { url: 'stub:///stylistic-rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///stylistic-test') {
        return { format: 'module', source: testStub, shortCircuit: true };
      }
      if (url === 'stub:///stylistic-parsers-jsx') {
        return { format: 'module', source: parserStub, shortCircuit: true };
      }
      if (url === 'stub:///stylistic-rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
