// Captures every authored eslint-plugin-perfectionist/sort-imports v5.9.1
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
const capturesArrayIncludes = process.argv.includes('--sort-array-includes');
const RULE = capturesArrayIncludes ? 'sort-array-includes' : 'sort-imports';
const PINNED_COMMIT = capturesArrayIncludes
  ? '84aa039c46522f82a61ad43cf676afc92dd64704'
  : 'b35e8e4caf0c8d350cf386e504241f21827dd60b';
const PINNED_REF = capturesArrayIncludes ? 'v5.10.0' : 'v5.9.1';
const TARGET_VERSION = capturesArrayIncludes ? '5.10.0' : '5.9.1';
const ESLINT_VERSION = capturesArrayIncludes ? '10.6.0' : '9.39.2';
const TYPESCRIPT_ESLINT_VERSION = capturesArrayIncludes ? '8.62.1' : '8.60.0';
const SOURCE_HASHES: Readonly<Record<string, string>> = capturesArrayIncludes
  ? {
      'rules/sort-array-includes.ts':
        '3f43cb92d44f5cd60de7ec9de9b4b72be936d3f1bf410d21e325bf18400788b9',
      'rules/sort-array-includes/types.ts':
        '927bfce114499a6e224415245e952a6eaa43c052dfd78b827bd7d8d085e7a098',
      'rules/sort-arrays/types.ts':
        '9aa7faafdb1f2262aa32798623ebd6507b5a80f2ad6fa66446d66bb722bba582',
      'rules/sort-arrays/sort-array.ts':
        'c65199dd2f5b56ae5302368f3e4fda46849f98641415bd77cf8d56a36ab0d60a',
      'test/rules/sort-array-includes.test.ts':
        'c6f8a4dea072ce3d1fc2af72430e7247551bd81870077bde12d5ad7bbb67e534',
    }
  : {
      'rules/sort-imports.ts': 'c5102c424e0364b0e9ce7681b41d9d3543d3a76b9227b3fea70371a4e83efa05',
      'rules/sort-imports/types.ts':
        '81aa65e9d8f085fa7e8479ea9a5e98c1c2e180450632e484494a1d64395ebffe',
      'test/rules/sort-imports.test.ts':
        '8065552b1ccf4d8110524ee48cdc5ee4ab701302d5316a0e2eef9c55ada249ab',
    };

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
const sourceCommit = capturesArrayIncludes
  ? execFileSync('git', ['-C', submodule, 'rev-parse', PINNED_REF], {
      encoding: 'utf8',
    }).trim()
  : actualCommit;
if (sourceCommit !== PINNED_COMMIT) {
  throw new Error(`Expected ${PINNED_COMMIT}, received ${sourceCommit}`);
}
for (const [sourceFile, expectedHash] of Object.entries(SOURCE_HASHES)) {
  const actualHash = createHash('sha256')
    .update(
      capturesArrayIncludes
        ? execFileSync('git', ['-C', submodule, 'show', `${PINNED_REF}:${sourceFile}`])
        : readFileSync(join(submodule, sourceFile)),
    )
    .digest('hex');
  if (actualHash !== expectedHash) {
    throw new Error(`Expected ${sourceFile} hash ${expectedHash}, received ${actualHash}`);
  }
}

const runnerDirectory = mkdtempSync(join(tmpdir(), 'perfectionist-sort-imports-'));
try {
  const authoredTestPath = `test/rules/${RULE}.test.ts`;
  const completeAuthoredSource = capturesArrayIncludes
    ? execFileSync('git', ['-C', submodule, 'show', `${PINNED_REF}:${authoredTestPath}`], {
        encoding: 'utf8',
      })
    : readFileSync(join(submodule, authoredTestPath), 'utf8');
  const suiteStart = completeAuthoredSource.indexOf(`describe('${RULE}'`);
  if (suiteStart === -1) {
    throw new Error(`Unable to locate the authored ${RULE} suite.`);
  }
  const authoredSource = completeAuthoredSource.slice(suiteStart);
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
  if (capturesArrayIncludes) {
    const forbiddenCases = authoredCases.filter(
      (testCase) =>
        /\.(?:jsx|tsx)$/u.test(testCase.filename) ||
        /<[A-Z][A-Za-z]*(?:\s|\/?>)/u.test(testCase.code),
    );
    if (forbiddenCases.length > 0) {
      throw new Error(
        `React/JSX/TSX cases are outside this port: ${forbiddenCases
          .map((testCase) => testCase.name)
          .join(', ')}`,
      );
    }
  }

  writeFileSync(
    join(runnerDirectory, 'package.json'),
    `${JSON.stringify(
      {
        private: true,
        type: 'module',
        dependencies: {
          '@typescript-eslint/parser': TYPESCRIPT_ESLINT_VERSION,
          eslint: ESLINT_VERSION,
          'eslint-plugin-perfectionist': TARGET_VERSION,
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
  execFileSync('node', [join(runnerDirectory, 'runner.mjs')], {
    cwd: runnerDirectory,
    stdio: 'inherit',
  });
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
      version: TARGET_VERSION,
      sourceCommit: PINNED_COMMIT,
      sourceHashes: SOURCE_HASHES,
      license: plugin.license,
      eslintVersion: ESLINT_VERSION,
      typescriptEslintParserVersion: TYPESCRIPT_ESLINT_VERSION,
      capturePolicy: capturesArrayIncludes
        ? `Every authored valid and invalid ${RULE} case is captured; React-specific and JSX/TSX syntax is rejected. Authored valid cases remain diagnostic-free; invalid cases are replayed against the published v${TARGET_VERSION} rule for exact diagnostics and fixes.`
        : 'Every authored valid and invalid sort-imports case is captured; none exercises React-specific or JSX/TSX syntax. Authored valid cases remain diagnostic-free; invalid cases are replayed against the published v5.9.1 rule for exact diagnostics and fixes.',
      authoredCases: capturesArrayIncludes
        ? `upstream/eslint-plugin-perfectionist/${authoredTestPath}@${PINNED_REF}`
        : `upstream/eslint-plugin-perfectionist/${authoredTestPath}`,
      tool: capturesArrayIncludes
        ? 'sync-perfectionist-sort-array-includes-options-tests.ts'
        : basename(import.meta.filename),
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
  const fixturePath = join(fixturesDirectory, `${RULE}-options-v${TARGET_VERSION}.json`);
  writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('pnpm', ['exec', 'vp', 'fmt', fixturePath], {
    cwd: ROOT,
    stdio: 'inherit',
  });
  console.log(
    `Captured ${RULE} v${TARGET_VERSION}: ${valid} valid, ${invalid} invalid, ${diagnostics} diagnostics.`,
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
const readClosestTsConfigUtilities = {}
const getTypescriptImportUtilities = {}
const rule = { meta: { schema: {} }, create() { return {} } }

function __normalizeCase(kind, input) {
  const value = typeof input === 'string' ? { code: input } : input
  __capturedCases.push({
    kind,
    name: __currentTestName,
    code: value.code,
    options: value.options ?? [],
    settings: value.settings,
    filename: value.filename ?? 'fixture.ts',
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
          filename: input.filename ?? 'fixture.ts',
          authoredOutput: Object.hasOwn(input, 'output') ? input.output : undefined,
        })
      }
      for (const input of suite.invalid ?? []) {
        __capturedCases.push({
          kind: 'invalid',
          name: [...__describeStack, name, 'invalid'].join(' > '),
          code: input.code,
          options: input.options ?? [],
          settings: input.settings,
          filename: input.filename ?? 'fixture.ts',
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

function createModuleResolutionCache() {
  return {}
}

const vi = {
  resetAllMocks() {},
  spyOn() {
    return {
      mockReturnValue() {
        return this
      },
    }
  },
}

async function validateRuleJsonSchema() {}
function expect() {
  const chain = {
    toThrow() {},
    not: { toThrow() {} },
    resolves: { not: { async toThrow() {} } },
  }
  return chain
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
writeFileSync(
  join(here, 'tsconfig.json'),
  JSON.stringify({
    compilerOptions: {
      baseUrl: '.',
      paths: { '$path': ['./path'] },
    },
  }),
);

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
  const fixed = linter.verifyAndFix(testCase.code, config, { filename });
  const expectedDiagnostics = testCase.kind === 'valid' ? [] : diagnostics;
  return {
    kind: testCase.kind,
    name: testCase.name,
    code: testCase.code,
    options: testCase.options ?? [],
    settings: testCase.settings,
    filename,
    expectedDiagnostics: expectedDiagnostics.map(problem => ({
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
    output: testCase.kind === 'valid' ? null : fixed.fixed ? fixed.output : null,
  };
});

writeFileSync(join(here, 'captured.json'), JSON.stringify(captured));

function dataFromMessage(message) {
  const dependency =
    /^Expected dependency "(.+)" to come before "(.+)"\\.$/u.exec(message);
  if (dependency) {
    return { right: dependency[1], nodeDependentOnRight: dependency[2] };
  }
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
