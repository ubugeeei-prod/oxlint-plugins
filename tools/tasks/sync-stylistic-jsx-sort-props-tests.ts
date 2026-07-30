// Captures every authored stable @stylistic/jsx-sort-props v5.10.0
// RuleTester case from the exact pinned upstream commit. The published rule is
// then replayed with ESLint's JSX parser and @typescript-eslint/parser so the
// committed fixture includes exact messages, ranges, first-pass fixes, and
// recursive output for the complete Oxc JSX/TSX parser matrix.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = { name: string; valid: RawCase[]; invalid: RawCase[] };
type NormalizedCase = {
  code: string;
  options: unknown[];
  features: string[];
  [key: string]: unknown;
};

const ROOT = process.cwd();
const RULE = 'jsx-sort-props';
const VERSION = '5.10.0';
const COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '10.4.1';
const TYPESCRIPT_ESLINT_VERSION = '8.60.0';
const UPSTREAM = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const FIXTURE = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures', `${RULE}-v${VERSION}.json`);
const CAPTURE_KEY = '__stylisticJsxSortPropsCapture__';

if (!existsSync(join(UPSTREAM, '.git'))) {
  throw new Error(`Missing ${UPSTREAM}; initialize upstream/eslint-stylistic.`);
}
const actualCommit = execFileSync('git', ['-C', UPSTREAM, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== COMMIT) {
  throw new Error(`Expected eslint-stylistic ${COMMIT}, received ${actualCommit}.`);
}

registerCaptureHooks();
const captureDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-sort-props-capture-'));
const captureFile = join(captureDir, `${RULE}.test.ts`);
writeFileSync(
  captureFile,
  execFileSync('git', ['-C', UPSTREAM, 'show', `${COMMIT}:${SOURCE_FILE}`], {
    encoding: 'utf8',
  }),
);
(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
try {
  await import(`${pathToFileURL(captureFile).href}?commit=${COMMIT}`);
} finally {
  rmSync(captureDir, { recursive: true, force: true });
}

const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
if (runs.length !== 1 || runs[0]?.name !== RULE) {
  throw new Error(`Expected one ${RULE} suite, received ${runs.length}.`);
}

const authored = {
  valid: runs[0].valid.map((value, index) => normalizeCase(value, false, index)),
  invalid: runs[0].invalid.map((value, index) => normalizeCase(value, true, index)),
};
const enriched = enrichWithPublishedRule(authored);
const diagnostics = enriched.invalid.reduce(
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
    license: 'MIT',
    eslintVersion: ESLINT_VERSION,
    typescriptEslintParserVersion: TYPESCRIPT_ESLINT_VERSION,
    parserMatrix: 'authored semantic cases replayed with Oxc JSX and TSX',
    tool: 'tools/tasks/sync-stylistic-jsx-sort-props-tests.ts',
    inventory: {
      authoredValid: enriched.valid.length,
      authoredInvalid: enriched.invalid.length,
      authoredDiagnostics: diagnostics,
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
execFileSync('pnpm', ['exec', 'vp', 'fmt', FIXTURE], { cwd: ROOT, stdio: 'inherit' });
console.log(
  `Synced ${RULE} v${VERSION}: ${enriched.valid.length} valid, ` +
    `${enriched.invalid.length} invalid, ${diagnostics} authored diagnostics; ` +
    `${parserExpandedValid + parserExpandedInvalid} Oxc parser-expanded cases.`,
);

function normalizeCase(raw: RawCase, invalid: boolean, index: number): NormalizedCase {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`${invalid ? 'invalid' : 'valid'} case ${index} has no code.`);
  }
  const allowed = new Set(
    invalid
      ? [
          'code',
          'output',
          'errors',
          'options',
          'parserOptions',
          'settings',
          'languageOptions',
          'features',
          'verifyFixChanges',
        ]
      : ['code', 'options', 'parserOptions', 'settings', 'languageOptions', 'features'],
  );
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported case ${index} keys: ${unsupported.join(', ')}`);
  }

  const normalized: NormalizedCase = {
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
    throw new Error(`Invalid case ${index} is missing errors.`);
  }
  return {
    ...normalized,
    authoredErrors: clone(value.errors),
    ...('output' in value ? { authoredOutput: value.output } : {}),
    ...('verifyFixChanges' in value ? { verifyFixChanges: value.verifyFixChanges } : {}),
  };
}

function enrichWithPublishedRule(cases: { valid: NormalizedCase[]; invalid: NormalizedCase[] }): {
  valid: NormalizedCase[];
  invalid: NormalizedCase[];
} {
  const runnerDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-sort-props-upstream-'));
  try {
    writeFileSync(
      join(runnerDir, 'package.json'),
      `${JSON.stringify(
        {
          private: true,
          type: 'module',
          dependencies: {
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
import { Linter } from 'eslint';
import tsParser from '@typescript-eslint/parser';
import * as stylisticModule from '@stylistic/eslint-plugin';

const here = fileURLToPath(new URL('.', import.meta.url));
const captured = JSON.parse(readFileSync(join(here, 'captured.json'), 'utf8'));
const stylistic = stylisticModule.default ?? stylisticModule;
const rule = stylistic.rules['${RULE}'];

function parsersFor(testCase) {
  return testCase.features.includes('ts') ? ['tsx'] : ['jsx', 'tsx'];
}

function configFor(testCase, parser) {
  return [{
    files: ['**/*.{js,jsx,ts,tsx}'],
    languageOptions: {
      ...(parser === 'tsx' ? { parser: tsParser } : {}),
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: { jsx: true },
        ...(testCase.parserOptions ?? {}),
      },
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

function verify(testCase, parser) {
  return new Linter().verify(testCase.code, configFor(testCase, parser), {
    filename: parser === 'tsx' ? 'fixture.tsx' : 'fixture.jsx',
  });
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
    throw new Error(
      'Invalid case ' + index + ' IDs differ: expected ' +
      JSON.stringify(expectedIds) + ', received ' + JSON.stringify(actualIds),
    );
  }
  for (let errorIndex = 0; errorIndex < testCase.authoredErrors.length; errorIndex++) {
    const expected = testCase.authoredErrors[errorIndex];
    const actual = messages[errorIndex];
    for (const key of ['line', 'column', 'endLine', 'endColumn']) {
      if (expected[key] !== undefined && expected[key] !== actual[key]) {
        throw new Error(
          'Invalid case ' + index + ' error ' + errorIndex + ' ' + key +
          ' differs: expected ' + expected[key] + ', received ' + actual[key],
        );
      }
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

  const firstFix = canonical.find(message => message.fix)?.fix;
  const firstPassOutput = firstFix
    ? testCase.code.slice(0, firstFix.range[0]) +
      firstFix.text +
      testCase.code.slice(firstFix.range[1])
    : null;
  if (
    typeof testCase.authoredOutput === 'string' &&
    firstPassOutput !== testCase.authoredOutput
  ) {
    throw new Error('First-pass output differs for invalid case ' + index);
  }
  const fixed = new Linter().verifyAndFix(
    testCase.code,
    configFor(testCase, parsers[0]),
    { filename: parsers[0] === 'tsx' ? 'fixture.tsx' : 'fixture.jsx' },
  );
  const recursiveOutput = fixed.fixed ? fixed.output : null;
  const {
    features,
    authoredErrors,
    authoredOutput,
    verifyFixChanges,
    ...rest
  } = testCase;
  return {
    ...rest,
    parsers,
    ...(authoredOutput !== undefined ? { output: authoredOutput } : {}),
    ...(verifyFixChanges !== undefined ? { verifyFixChanges } : {}),
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
    'export function run(options) {',
    '  globalThis[key].push({ name: options.name, valid: options.valid || [], invalid: options.invalid || [] });',
    '}',
  ].join('\n');
  const parserStub = [
    'function authored(tests) { return tests.flat(Infinity).filter(Boolean); }',
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
