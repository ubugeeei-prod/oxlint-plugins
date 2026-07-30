// Captures the authored named-specifier option matrix against
// eslint-plugin-perfectionist/sort-named-exports v5.9.1. The exact upstream
// commit and source hashes prevent silent fixture drift.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

type TestCase = {
  name: string;
  code: string;
  options?: Array<Record<string, unknown>>;
  filename?: string;
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

type AuthoredFixture = {
  __generated: {
    version: string;
    sourceCommit: string;
  };
  cases: TestCase[];
};

const ROOT = process.cwd();
const RULE = 'sort-named-exports';
const PINNED_COMMIT = 'b35e8e4caf0c8d350cf386e504241f21827dd60b';
const ESLINT_VERSION = '9.39.2';
const TYPESCRIPT_ESLINT_VERSION = '8.60.0';
const SOURCE_HASHES = {
  'rules/sort-named-exports.ts': '8fab71ff2def798b8c5d96d6871125bae80a67e7de6e76ecf15475863b61655e',
  'rules/sort-named-exports/sort-named-export.ts':
    'a6016b975724ee115a23467044096934c8b059076ba5e15bc691d920763a7b4d',
  'test/rules/sort-named-exports.test.ts':
    'af2674f97668787725ea7971db7c847e517865218eda4bb24c1e99584f53ec4c',
} as const;

const authoredFixture = JSON.parse(
  readFileSync(
    join(
      ROOT,
      'npm',
      'perfectionist',
      'test',
      'fixtures',
      'sort-named-imports-options-v5.9.1.json',
    ),
    'utf8',
  ),
) as AuthoredFixture;
if (
  authoredFixture.__generated.version !== '5.9.1' ||
  authoredFixture.__generated.sourceCommit !== PINNED_COMMIT
) {
  throw new Error('Named-import authored matrix is not pinned to Perfectionist v5.9.1.');
}

const cases = authoredFixture.cases.map(toNamedExportCase);

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

const runnerDirectory = mkdtempSync(join(tmpdir(), 'perfectionist-named-exports-'));
try {
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
  writeFileSync(join(runnerDirectory, 'cases.json'), `${JSON.stringify(cases)}\n`);
  writeFileSync(join(runnerDirectory, 'runner.mjs'), runnerSource());
  execFileSync(
    'pnpm',
    ['install', '--dir', runnerDirectory, '--ignore-workspace', '--lockfile=false', '--silent'],
    { stdio: 'inherit' },
  );
  execFileSync('node', [join(runnerDirectory, 'runner.mjs')], { stdio: 'inherit' });
  const captured = JSON.parse(
    readFileSync(join(runnerDirectory, 'captured.json'), 'utf8'),
  ) as Array<{ expectedDiagnostics: unknown[] }>;
  const valid = captured.filter((testCase) => testCase.expectedDiagnostics.length === 0).length;
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
        'Authored named-specifier matrix replayed for export scalar comparators, groups, custom groups, partitions, newline policies, and conditional configuration.',
      authoredCases:
        'npm/perfectionist/test/fixtures/sort-named-imports-options-v5.9.1.json inputs',
      tool: 'tools/tasks/sync-perfectionist-sort-named-exports-options-tests.ts',
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

function toNamedExportCase(testCase: TestCase): TestCase {
  let code = testCase.code.replaceAll('import ', 'export ');
  code = code.replace(/export Default,\s*(?=\{)/gu, 'export ');
  return {
    name: testCase.name.replaceAll('import', 'export'),
    code,
    options: transformOptions(testCase.options ?? []) as Array<Record<string, unknown>>,
    filename: 'fixture.ts',
  };
}

function transformOptions(value: unknown, key?: string): unknown {
  if (Array.isArray(value)) {
    return value.map((entry) => transformOptions(entry, key));
  }
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([entryKey, entryValue]) => [
        entryKey,
        transformOptions(entryValue, entryKey),
      ]),
    );
  }
  if (typeof value !== 'string') {
    return value;
  }
  if (value === 'type-import') {
    return 'type-export';
  }
  if (value === 'value-import') {
    return 'value-export';
  }
  if (key === 'selector' && value === 'import') {
    return 'export';
  }
  if (key === 'matchesAstSelector') {
    return value.replaceAll('ImportDeclaration', 'ExportNamedDeclaration');
  }
  return value;
}

function runnerSource(): string {
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
    plugins: { perfectionist },
    rules: {
      'perfectionist/${RULE}': ['error', ...(testCase.options ?? [])],
    },
  }];
}

const captured = cases.map(testCase => {
  const filename = testCase.filename ?? 'fixture.ts';
  const config = configFor(testCase);
  const diagnostics = linter.verify(testCase.code, config, { filename });
  const unexpected = diagnostics.filter(problem => problem.ruleId !== 'perfectionist/${RULE}');
  if (unexpected.length > 0) {
    throw new Error(
      testCase.name + ' produced non-rule diagnostics: ' + JSON.stringify(unexpected),
    );
  }
  const fixed = linter.verifyAndFix(testCase.code, config, { filename });
  return {
    ...testCase,
    options: testCase.options ?? [],
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
  const match = /^Expected "(.+)" to come before "(.+)"\\.$/u.exec(message);
  if (match) {
    return { right: match[1], left: match[2] };
  }
  const groupMatch = /^Expected "(.+)" \\((.+)\\) to come before "(.+)" \\((.+)\\)\\.$/u.exec(message);
  if (groupMatch) {
    return {
      right: groupMatch[1],
      rightGroup: groupMatch[2],
      left: groupMatch[3],
      leftGroup: groupMatch[4],
    };
  }
  const spacingMatch = /^(?:Extra|Missed) spacing between "(.+)" and "(.+)"\\.$/u.exec(message);
  if (spacingMatch) {
    return { left: spacingMatch[1], right: spacingMatch[2] };
  }
  return {};
}
`;
}
