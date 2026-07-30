// Captures every authored eslint-plugin-perfectionist/sort-exports v5.9.1
// valid and invalid case. The exact upstream commit and source hashes prevent
// silent fixture drift.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';

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

type CapturedCase = {
  kind: 'valid' | 'invalid';
  name: string;
  code: string;
  options: Array<Record<string, unknown>>;
  settings?: Record<string, unknown>;
  filename: string;
  authoredOutput?: string | null;
};

const ROOT = process.cwd();
const RULE = 'sort-exports';
const PINNED_COMMIT = 'b35e8e4caf0c8d350cf386e504241f21827dd60b';
const ESLINT_VERSION = '9.39.2';
const TYPESCRIPT_ESLINT_VERSION = '8.60.0';
const SOURCE_HASHES = {
  'rules/sort-exports.ts': 'dd6ee1cd385e1f8cca77357a781f5bd82176d67d7e33695ca177624095077baf',
  'rules/sort-exports/types.ts': '5f09e2870357909e40e8a33acce0bf2fb796350246e970f77432e2a3c8a309df',
  'test/rules/sort-exports.test.ts':
    '1590b05abda2fa82c9e9579514b04f3c03137c26834711fb94fce6af399d85e4',
} as const;

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-perfectionist');
if (!plugin) {
  throw new Error('eslint-plugin-perfectionist is not registered in tools/port-targets.json');
}
if (plugin.baselineVersion !== '5.9.1' || plugin.pinnedRef !== 'v5.9.1') {
  throw new Error(
    `Expected perfectionist v5.9.1 manifest pin, received ${plugin.baselineVersion} / ${plugin.pinnedRef}`,
  );
}

const submodule = join(ROOT, plugin.submodule);
if (!existsSync(join(submodule, '.git'))) {
  throw new Error(`Upstream checkout not found: ${plugin.submodule}`);
}
const actualCommit = execFileSync('git', ['-C', submodule, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== PINNED_COMMIT) {
  throw new Error(`Expected ${PINNED_COMMIT}, received ${actualCommit}`);
}
for (const [sourceFile, expectedHash] of Object.entries(SOURCE_HASHES)) {
  const actualHash = createHash('sha256')
    .update(readFileSync(join(submodule, sourceFile)))
    .digest('hex');
  if (actualHash !== expectedHash) {
    throw new Error(`Expected ${sourceFile} hash ${expectedHash}, received ${actualHash}`);
  }
}

const runnerDirectory = mkdtempSync(join(tmpdir(), 'perfectionist-sort-exports-'));
try {
  const authoredSource = readFileSync(
    join(submodule, 'test', 'rules', 'sort-exports.test.ts'),
    'utf8',
  ).replace(/^import .*$/gmu, '');
  const captureSourcePath = join(runnerDirectory, 'capture.ts');
  writeFileSync(captureSourcePath, `${capturePrelude()}\n${authoredSource}\n${captureEpilogue()}`);
  const compiledDirectory = join(runnerDirectory, 'compiled');
  execFileSync(
    'pnpm',
    [
      'exec',
      'vp',
      'pack',
      captureSourcePath,
      '--format',
      'esm',
      '--out-dir',
      compiledDirectory,
      '--no-clean',
      '--logLevel',
      'error',
    ],
    { cwd: ROOT, stdio: 'inherit' },
  );
  const compiledSource = readdirSync(compiledDirectory).find((file) => /\.(?:mjs|js)$/u.test(file));
  if (!compiledSource) {
    throw new Error(`No compiled capture module found in ${compiledDirectory}`);
  }
  execFileSync('node', [join(compiledDirectory, compiledSource)], {
    cwd: runnerDirectory,
    stdio: 'inherit',
  });
  const authoredCases = JSON.parse(
    readFileSync(join(runnerDirectory, 'authored-cases.json'), 'utf8'),
  ) as CapturedCase[];

  writeFileSync(
    join(runnerDirectory, 'package.json'),
    `${JSON.stringify(
      {
        private: true,
        type: 'module',
        dependencies: {
          '@typescript-eslint/parser': TYPESCRIPT_ESLINT_VERSION,
          eslint: ESLINT_VERSION,
          'eslint-plugin-perfectionist': plugin.baselineVersion,
        },
      },
      null,
      2,
    )}\n`,
  );
  writeFileSync(join(runnerDirectory, 'cases.json'), `${JSON.stringify(authoredCases)}\n`);
  writeFileSync(join(runnerDirectory, 'runner.mjs'), publishedRunnerSource());
  execFileSync(
    'pnpm',
    ['install', '--dir', runnerDirectory, '--ignore-workspace', '--lockfile=false', '--silent'],
    { stdio: 'inherit' },
  );
  execFileSync('node', [join(runnerDirectory, 'runner.mjs')], { stdio: 'inherit' });
  const captured = JSON.parse(
    readFileSync(join(runnerDirectory, 'captured.json'), 'utf8'),
  ) as Array<{ kind: 'valid' | 'invalid'; expectedDiagnostics: unknown[] }>;
  const valid = captured.filter((testCase) => testCase.kind === 'valid').length;
  const invalid = captured.length - valid;
  const diagnostics = captured.reduce(
    (total, testCase) => total + testCase.expectedDiagnostics.length,
    0,
  );
  const fixture = {
    __generated: {
      source: plugin.npm,
      version: plugin.baselineVersion,
      sourceCommit: PINNED_COMMIT,
      sourceHashes: SOURCE_HASHES,
      license: plugin.license,
      eslintVersion: ESLINT_VERSION,
      typescriptEslintParserVersion: TYPESCRIPT_ESLINT_VERSION,
      capturePolicy:
        'Every authored valid and invalid sort-exports case is executed, then replayed against the published v5.9.1 rule for exact diagnostics and fixes.',
      authoredCases: 'upstream/eslint-plugin-perfectionist/test/rules/sort-exports.test.ts',
      tool: basename(import.meta.filename),
      inventory: {
        valid,
        invalid,
        diagnostics,
        total: captured.length,
      },
    },
    cases: captured,
  };
  const fixturesDirectory = join(ROOT, 'npm', 'perfectionist', 'test', 'fixtures');
  mkdirSync(fixturesDirectory, { recursive: true });
  const fixturePath = join(fixturesDirectory, `${RULE}-options-v${plugin.baselineVersion}.json`);
  writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('pnpm', ['exec', 'vp', 'fmt', fixturePath], {
    cwd: ROOT,
    stdio: 'inherit',
  });
  console.log(
    `Captured ${RULE} v${plugin.baselineVersion}: ${valid} valid, ${invalid} invalid, ${diagnostics} diagnostics.`,
  );
} finally {
  rmSync(runnerDirectory, { recursive: true, force: true });
}

function capturePrelude(): string {
  return String.raw`
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

const __capturedCases = []
const __tasks = []
const __describeStack = []
let __currentTestName = ''
const typescriptParser = {}
const rule = { meta: { schema: {} } }

function __normalizeCase(kind, input) {
  const value = typeof input === 'string' ? { code: input } : input
  __capturedCases.push({
    kind,
    name: __currentTestName,
    code: value.code,
    options: value.options ?? [],
    settings: value.settings,
    filename: 'fixture.ts',
    authoredOutput: Object.hasOwn(value, 'output') ? value.output : undefined,
  })
}

function createRuleTester() {
  return {
    valid: async input => __normalizeCase('valid', input),
    invalid: async input => __normalizeCase('invalid', input),
  }
}

function buildOxlintRuleTester() {
  return {
    run(name, suite) {
      for (const input of suite.valid ?? []) {
        __capturedCases.push({
          kind: 'valid',
          name: [...__describeStack, name, 'valid'].join(' > '),
          code: input.code,
          options: input.options ?? [],
          settings: input.settings,
          filename: 'fixture.ts',
        })
      }
      for (const input of suite.invalid ?? []) {
        __capturedCases.push({
          kind: 'invalid',
          name: [...__describeStack, name, 'invalid'].join(' > '),
          code: input.code,
          options: input.options ?? [],
          settings: input.settings,
          filename: 'fixture.ts',
          authoredOutput: Object.hasOwn(input, 'output') ? input.output : undefined,
        })
      }
    },
  }
}

function describe(name, callback) {
  __describeStack.push(name)
  callback()
  __describeStack.pop()
}

function it(name, callback) {
  __tasks.push({
    name: [...__describeStack, name].join(' > '),
    callback,
  })
}

it.each = rows => (name, callback) => {
  for (const row of rows) {
    const values = Array.isArray(row) ? row : [row]
    let interpolation = 0
    const expandedName = name.replaceAll('%s', () => String(values[interpolation++]))
    __tasks.push({
      name: [...__describeStack, expandedName].join(' > '),
      callback: () => callback(...values),
    })
  }
}

function dedent(strings, ...values) {
  const value =
    typeof strings === 'string'
      ? strings
      : strings.reduce((result, string, index) => result + string + (values[index] ?? ''), '')
  const lines = value.replace(/^\n/u, '').replace(/\n\s*$/u, '').split('\n')
  const indentation = lines
    .filter(line => line.trim().length > 0)
    .reduce((minimum, line) => Math.min(minimum, line.match(/^\s*/u)[0].length), Infinity)
  return lines.map(line => line.slice(Number.isFinite(indentation) ? indentation : 0)).join('\n')
}

const Alphabet = {
  generateRecommendedAlphabet() {
    return {
      sortByLocaleCompare() {
        return this
      },
      getCharacters() {
        return '0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'
      },
    }
  },
}

async function validateRuleJsonSchema() {}
function expect() {
  return {
    resolves: {
      not: {
        async toThrow() {},
      },
    },
  }
}
`;
}

function captureEpilogue(): string {
  return String.raw`
for (const task of __tasks) {
  __currentTestName = task.name
  await task.callback()
}
writeFileSync(
  join(process.cwd(), 'authored-cases.json'),
  JSON.stringify(__capturedCases),
)
`;
}

function publishedRunnerSource(): string {
  return `
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Linter } from 'eslint';
import tsParser from '@typescript-eslint/parser';
import perfectionistModule from 'eslint-plugin-perfectionist';

const here = fileURLToPath(new URL('.', import.meta.url));
const cases = JSON.parse(readFileSync(join(here, 'cases.json'), 'utf8'));
const perfectionist = perfectionistModule.default ?? perfectionistModule;
const linter = new Linter();

function configFor(testCase) {
  return [{
    files: ['**/*.{js,ts}'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    plugins: {
      'rule-to-test': {
        rules: {
          '${RULE}': perfectionist.rules['${RULE}'],
        },
      },
    },
    settings: testCase.settings ?? {},
    rules: {
      'rule-to-test/${RULE}': ['error', ...(testCase.options ?? [])],
    },
  }];
}

const captured = cases.map(testCase => {
  const filename = testCase.filename ?? 'fixture.ts';
  const config = configFor(testCase);
  const diagnostics = linter.verify(testCase.code, config, { filename });
  const unexpected = diagnostics.filter(problem => problem.ruleId !== 'rule-to-test/${RULE}');
  if (unexpected.length > 0) {
    throw new Error(
      testCase.name + ' produced non-rule diagnostics: ' + JSON.stringify(unexpected),
    );
  }
  if (testCase.kind === 'valid' && diagnostics.length > 0) {
    throw new Error(testCase.name + ' was authored valid but published with diagnostics.');
  }
  const fixed = linter.verifyAndFix(testCase.code, config, { filename });
  return {
    kind: testCase.kind,
    name: testCase.name,
    code: testCase.code,
    options: testCase.options ?? [],
    settings: testCase.settings,
    filename,
    expectedDiagnostics: diagnostics.map(problem => ({
      messageId: problem.messageId,
      message: problem.message,
      data: dataFromMessage(problem.message),
      loc: {
        startLine: problem.line,
        startColumn: problem.column,
        endLine: problem.endLine,
        endColumn: problem.endColumn,
      },
      fix: problem.fix
        ? {
            range: problem.fix.range,
            text: problem.fix.text,
          }
        : null,
    })),
    output: fixed.fixed ? fixed.output : null,
  };
});

writeFileSync(join(here, 'captured.json'), JSON.stringify(captured));

function dataFromMessage(message) {
  const order = /^Expected "(.+)" to come before "(.+)"\\.$/u.exec(message);
  if (order) {
    return { right: order[1], left: order[2] };
  }
  const group = /^Expected "(.+)" \\((.+)\\) to come before "(.+)" \\((.+)\\)\\.$/u.exec(message);
  if (group) {
    return {
      right: group[1],
      rightGroup: group[2],
      left: group[3],
      leftGroup: group[4],
    };
  }
  const spacing = /^(?:Extra|Missed) spacing between "(.+)" and "(.+)"\\.$/u.exec(message);
  if (spacing) {
    return { left: spacing[1], right: spacing[2] };
  }
  const comment = /^Missed comment "(.+)" above "(.+)"\\.$/u.exec(message);
  if (comment) {
    return { missedCommentAbove: comment[1], right: comment[2] };
  }
  return {};
}
`;
}
