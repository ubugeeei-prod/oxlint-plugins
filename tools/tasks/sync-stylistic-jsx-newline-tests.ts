// Captures the stable @stylistic/jsx-newline v5.10.0 RuleTester suite
// from the exact pinned upstream commit, including ESLint 10's Espree and
// TypeScript parser expansion. The published package then supplies exact
// diagnostics, ranges, single-pass fixes, and recursive fixed output.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type Capture = { name: string; valid: RawCase[]; invalid: RawCase[] };
type NormalizedCase = {
  code: string;
  options: unknown[];
  parser: 'espree' | 'typescript';
  [key: string]: unknown;
};
type Manifest = {
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    baselineVersion: string;
    pinnedRef?: string;
    license: string;
  }>;
};

const ROOT = process.cwd();
const RULE = 'jsx-newline';
const VERSION = '5.10.0';
const PINNED_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '10.0.0';
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const CAPTURE_KEY = '__stylisticJsxNewlineCapture__';

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-stylistic');
if (!plugin) {
  throw new Error('eslint-stylistic is not registered in tools/port-targets.json');
}
if (plugin.baselineVersion !== VERSION || plugin.pinnedRef !== `v${VERSION}`) {
  throw new Error(
    `Expected @stylistic v${VERSION} manifest pin, received ` +
      `${plugin.baselineVersion} / ${plugin.pinnedRef}`,
  );
}

const submodule = join(ROOT, plugin.submodule);
if (!existsSync(join(submodule, '.git'))) {
  throw new Error(
    `Upstream checkout not found at ${submodule}. ` +
      `Run \`git submodule update --init ${plugin.submodule}\` first.`,
  );
}
const actualCommit = execFileSync('git', ['-C', submodule, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== PINNED_COMMIT) {
  throw new Error(`Expected ${plugin.submodule} at ${PINNED_COMMIT}, received ${actualCommit}.`);
}

registerCaptureHooks();
const captureDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-newline-capture-'));
try {
  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  const captureFile = join(captureDir, `${RULE}.test.ts`);
  writeFileSync(
    captureFile,
    execFileSync('git', ['-C', submodule, 'show', `${PINNED_COMMIT}:${SOURCE_FILE}`], {
      encoding: 'utf8',
    }),
  );
  await import(`${pathToFileURL(captureFile).href}?commit=${PINNED_COMMIT}`);

  const captures = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as Capture[];
  if (captures.length !== 1 || captures[0]?.name !== RULE) {
    throw new Error(`Expected one captured ${RULE} suite, received ${captures.length}.`);
  }

  const captured = {
    valid: captures[0].valid.map((testCase, index) =>
      normalizeCase(testCase, false, `valid ${index}`),
    ),
    invalid: captures[0].invalid.map((testCase, index) =>
      normalizeCase(testCase, true, `invalid ${index}`),
    ),
  };
  const enriched = enrichWithPublishedRule(captured);
  const diagnostics = enriched.invalid.reduce(
    (total, testCase) =>
      total + (testCase.expectedDiagnostics as Array<Record<string, unknown>>).length,
    0,
  );
  const unfixableInvalid = enriched.invalid.filter((testCase) => testCase.output === null).length;
  const fixture = {
    __generated: {
      source: plugin.npm,
      version: plugin.baselineVersion,
      sourceCommit: PINNED_COMMIT,
      sourceFile: SOURCE_FILE,
      license: plugin.license,
      eslintVersion: ESLINT_VERSION,
      parserVersions: {
        espree: '10.4.0',
        typescriptEslint: '8.56.0',
        typescript: '5.9.3',
      },
      parserMatrix: 'ESLint 10 parsers-jsx expansion; Babel disabled by the stable runner',
      tool: 'tools/tasks/sync-stylistic-jsx-newline-tests.ts',
      inventory: {
        logicalValid: logicalCount(captured.valid),
        logicalInvalid: logicalCount(captured.invalid),
        valid: enriched.valid.length,
        invalid: enriched.invalid.length,
        diagnostics,
        unfixableInvalid,
        total: enriched.valid.length + enriched.invalid.length,
        fixableInvalid: enriched.invalid.length - unfixableInvalid,
      },
    },
    valid: enriched.valid,
    invalid: enriched.invalid,
  };

  const fixturePath = join(
    ROOT,
    'npm',
    'stylistic',
    'test',
    'fixtures',
    `${RULE}-v${VERSION}.json`,
  );
  mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
  writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('pnpm', ['exec', 'vp', 'fmt', fixturePath], {
    cwd: ROOT,
    stdio: 'inherit',
  });
  console.log(
    `Synced @stylistic/${RULE} v${VERSION} (${PINNED_COMMIT}): ` +
      `${enriched.valid.length} valid, ${enriched.invalid.length} invalid, ` +
      `${diagnostics} diagnostics (${unfixableInvalid} unfixable).`,
  );
} finally {
  rmSync(captureDir, { recursive: true, force: true });
}

function logicalCount(cases: NormalizedCase[]): number {
  return new Set(
    cases.map((testCase) =>
      JSON.stringify({
        code: testCase.code.replace(/\n\/\/ features: \[[^\n]*$/u, ''),
        options: testCase.options,
      }),
    ),
  ).size;
}

function normalizeCase(raw: RawCase, invalid: boolean, label: string): NormalizedCase {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (
    !value ||
    typeof value !== 'object' ||
    typeof value.code !== 'string' ||
    !['espree', 'typescript'].includes(String(value.parser))
  ) {
    throw new Error(`Unsupported ${label}`);
  }
  const allowed = new Set(
    invalid
      ? ['code', 'options', 'parserOptions', 'output', 'errors', 'parser']
      : ['code', 'options', 'parserOptions', 'parser'],
  );
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported ${label} keys: ${unsupported.join(', ')}`);
  }

  const normalized: NormalizedCase = {
    code: value.code,
    options: Array.isArray(value.options) ? clone(value.options) : [],
    parser: value.parser as NormalizedCase['parser'],
    ...('parserOptions' in value ? { parserOptions: clone(value.parserOptions) } : {}),
  };
  if (!invalid) {
    return normalized;
  }
  if (!('output' in value) || (typeof value.output !== 'string' && value.output !== null)) {
    throw new Error(`${label} is missing its fixed output`);
  }
  if (!Array.isArray(value.errors)) {
    throw new Error(`${label} is missing its ordered errors`);
  }
  return {
    ...normalized,
    output: value.output,
    upstreamErrors: clone(value.errors),
  };
}

function enrichWithPublishedRule(cases: { valid: NormalizedCase[]; invalid: NormalizedCase[] }): {
  valid: NormalizedCase[];
  invalid: NormalizedCase[];
} {
  const runnerDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-newline-upstream-'));
  try {
    writeFileSync(
      join(runnerDir, 'package.json'),
      `${JSON.stringify(
        {
          private: true,
          type: 'module',
          dependencies: {
            '@stylistic/eslint-plugin': VERSION,
            '@typescript-eslint/parser': '8.56.0',
            eslint: ESLINT_VERSION,
            espree: '10.4.0',
            typescript: '5.9.3',
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
import typescriptParser from '@typescript-eslint/parser';
import { Linter } from 'eslint';
import * as stylisticModule from '@stylistic/eslint-plugin';

const here = fileURLToPath(new URL('.', import.meta.url));
const captured = JSON.parse(readFileSync(join(here, 'captured.json'), 'utf8'));
const stylistic = stylisticModule.default ?? stylisticModule;
const rule = stylistic.rules['${RULE}'];

function configFor(testCase) {
  const typescript = testCase.parser === 'typescript';
  return [{
    files: ['**/*.jsx', '**/*.tsx'],
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'script',
      ...(typescript ? { parser: typescriptParser } : {}),
      parserOptions: typescript
        ? { ecmaFeatures: { jsx: true, modules: true } }
        : {
            ...(testCase.parserOptions ?? {}),
            ecmaFeatures: {
              ...(testCase.parserOptions?.ecmaFeatures ?? {}),
              jsx: true,
            },
          },
    },
    plugins: {
      stylistic: { rules: { '${RULE}': rule } },
    },
    rules: {
      'stylistic/${RULE}': ['error', ...testCase.options],
    },
  }];
}

function filenameFor(testCase) {
  return testCase.parser === 'typescript' ? 'fixture.tsx' : 'fixture.jsx';
}

function verify(testCase) {
  currentSource = testCase.code;
  const messages = new Linter().verify(testCase.code, configFor(testCase), {
    filename: filenameFor(testCase),
  });
  const ruleMessages = messages.filter(message => message.ruleId === 'stylistic/${RULE}');
  const unexpected = messages.filter(message => message.ruleId !== 'stylistic/${RULE}');
  if (unexpected.length !== 0) {
    throw new Error('Unexpected ESLint messages: ' + JSON.stringify(unexpected));
  }
  return ruleMessages;
}

function offsetAt(line, column) {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line) {
    const match = /\\r\\n|[\\n\\r\\u2028\\u2029]/u.exec(currentSource.slice(offset));
    if (!match) throw new Error('Cannot map diagnostic location');
    offset += match.index + match[0].length;
    currentLine += 1;
  }
  return offset + column - 1;
}

function diagnostic(message) {
  const start = offsetAt(message.line, message.column);
  const end = message.endLine === undefined || message.endColumn === undefined
    ? start
    : offsetAt(message.endLine, message.endColumn);
  return {
    messageId: message.messageId,
    message: message.message,
    data: {},
    range: [start, end],
    loc: {
      line: message.line,
      column: message.column,
      endLine: message.endLine,
      endColumn: message.endColumn,
    },
    fix: message.fix ? { range: message.fix.range, text: message.fix.text } : null,
  };
}

function singlePassOutput(source, messages) {
  const fixes = messages.flatMap(message => message.fix ? [message.fix] : []);
  if (fixes.length === 0) return null;
  fixes.sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);
  let output = source;
  for (const fix of fixes) {
    output = output.slice(0, fix.range[0]) + fix.text + output.slice(fix.range[1]);
  }
  return output;
}

let currentSource = '';
const valid = captured.valid.map((testCase, index) => {
  const messages = verify(testCase);
  if (messages.length !== 0) {
    throw new Error('Published rule reported valid case ' + index + ': ' + JSON.stringify(messages));
  }
  return testCase;
});

const invalid = captured.invalid.map((testCase, index) => {
  const messages = verify(testCase);
  const expectedIds = testCase.upstreamErrors.map(error => error.messageId);
  const actualIds = messages.map(message => message.messageId);
  if (JSON.stringify(actualIds) !== JSON.stringify(expectedIds)) {
    throw new Error(
      'Published rule IDs differ for invalid case ' + index + ': expected '
      + JSON.stringify(expectedIds) + ', received ' + JSON.stringify(actualIds),
    );
  }
  for (const [messageIndex, expected] of testCase.upstreamErrors.entries()) {
    const actual = messages[messageIndex];
    for (const key of ['line', 'column', 'endLine', 'endColumn']) {
      if (expected[key] !== undefined && actual[key] !== expected[key]) {
        throw new Error(
          'Published rule location differs for invalid case ' + index + ', message '
          + messageIndex + ', ' + key + ': expected ' + expected[key] + ', received ' + actual[key],
        );
      }
    }
  }
  const output = singlePassOutput(testCase.code, messages);
  if (output !== testCase.output) {
    throw new Error(
      'Published rule output differs for invalid case ' + index + ': expected '
      + JSON.stringify(testCase.output) + ', received ' + JSON.stringify(output),
    );
  }
  const fixed = new Linter().verifyAndFix(testCase.code, configFor(testCase), {
    filename: filenameFor(testCase),
  });
  return {
    code: testCase.code,
    options: testCase.options,
    parser: testCase.parser,
    ...(testCase.parserOptions ? { parserOptions: testCase.parserOptions } : {}),
    output,
    recursiveOutput: fixed.fixed ? fixed.output : null,
    upstreamErrors: testCase.upstreamErrors,
    expectedDiagnostics: messages.map(diagnostic),
  };
});

writeFileSync(join(here, 'enriched.json'), JSON.stringify({ valid, invalid }));
`;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export function run(options) { globalThis[key].push(options); }',
  ].join('\n');
  const parsersStub = `
function addParser(testCase, parser, features) {
  const value = typeof testCase === 'string' ? { code: testCase } : testCase;
  const extras = [
    'features: [' + Array.from(features).join(',') + ']',
    'parser: ' + (parser === 'espree' ? 'default' : '@typescript-eslint/parser'),
    value.parserOptions ? 'parserOptions: ' + JSON.stringify(value.parserOptions) : '',
    value.options ? 'options: ' + JSON.stringify(value.options) : '',
    value.settings ? 'settings: ' + JSON.stringify(value.settings) : '',
  ];
  const extraComment = '\\n// ' + extras.join(', ');
  const { features: _features, languageOptions: _languageOptions, ...rest } = value;
  return {
    ...rest,
    code: value.code + extraComment,
    ...(value.output ? { output: value.output + extraComment } : {}),
    parser,
  };
}
function expand(tests) {
  return tests.flat().filter(Boolean).flatMap(testCase => {
    const value = typeof testCase === 'string' ? { code: testCase } : testCase;
    const features = new Set(value.features || []);
    const skipBase = [
      'class fields', 'no-default', 'bind operator', 'do expressions',
      'decorators', 'flow', 'ts', 'types', 'fragment',
    ].some(feature => features.has(feature));
    const skipTS = [
      'no-ts', 'flow', 'jsx namespace', 'bind operator', 'do expressions',
    ].some(feature => features.has(feature));
    return [
      ...(skipBase ? [] : [addParser(value, 'espree', features)]),
      ...(skipTS || features.has('no-ts-new')
        ? []
        : [addParser(value, 'typescript', features)]),
    ];
  });
}
export function valids(...tests) { return expand(tests); }
export function invalids(...tests) { return expand(tests); }
`;

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
        return { format: 'module', source: parsersStub, shortCircuit: true };
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
