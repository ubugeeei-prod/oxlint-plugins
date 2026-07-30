// Captures a curated scalar-option contract for
// eslint-plugin-perfectionist/sort-named-imports v5.9.1 from the published
// package. The exact submodule commit and source hashes prevent the fixture
// from silently drifting away from the reviewed upstream implementation.

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

const ROOT = process.cwd();
const RULE = 'sort-named-imports';
const PINNED_COMMIT = 'b35e8e4caf0c8d350cf386e504241f21827dd60b';
const ESLINT_VERSION = '9.39.2';
const TYPESCRIPT_ESLINT_VERSION = '8.60.0';
const SOURCE_HASHES = {
  'rules/sort-named-imports.ts': 'ef0f575cb4aca0248120e8eb831748b2fb7a170e2e2220b8cdee66f1c2740ae6',
  'rules/sort-named-imports/sort-named-import.ts':
    '3e35c4b385cab6e07acac20e2ee4f5049e3cfc1fb05403ac0158cd360e4b3415',
  'test/rules/sort-named-imports.test.ts':
    'a5fe43752e460a2d29432e01e01a21aaa92bf9bc6d5193840dc593c6767ddeee',
} as const;

const cases: TestCase[] = [
  {
    name: 'default alphabetical order',
    code: `import { AAA, BB, C } from 'module'`,
  },
  {
    name: 'default alphabetical inversion',
    code: `import { BB, AAA, C } from 'module'`,
  },
  {
    name: 'default complete reversal',
    code: `import { C, BB, AAA } from 'module'`,
  },
  {
    name: 'multiline alphabetical order',
    code: `import {\n  AAAA,\n  BBB,\n  CC,\n  D,\n} from 'module'`,
  },
  {
    name: 'multiline alphabetical inversion',
    code: `import {\n  AAAA,\n  CC,\n  BBB,\n  D,\n} from 'module'`,
  },
  {
    name: 'default ignores default import',
    code: `import Default, { A, B } from 'module'`,
  },
  {
    name: 'default sorts by local alias',
    code: `import { c as A, b as B, a as C } from 'module'`,
  },
  {
    name: 'default local alias inversion',
    code: `import { a as C, b as B, c as A } from 'module'`,
  },
  {
    name: 'ignoreAlias original names',
    code: `import { A as C, B as B, C as A } from 'module'`,
    options: [{ ignoreAlias: true }],
  },
  {
    name: 'ignoreAlias inversion',
    code: `import { C as A, B as B, A as C } from 'module'`,
    options: [{ ignoreAlias: true }],
  },
  {
    name: 'ignoreCase stable equivalent aliases',
    code: `import { b as a, a as A } from 'module'`,
  },
  {
    name: 'case-sensitive lowercase before uppercase',
    code: `import { b as a, a as A } from 'module'`,
    options: [{ ignoreCase: false }],
  },
  {
    name: 'case-sensitive inversion',
    code: `import { a as A, b as a } from 'module'`,
    options: [{ ignoreCase: false }],
  },
  {
    name: 'alphabetical descending',
    code: `import { C, BB, AAA } from 'module'`,
    options: [{ order: 'desc' }],
  },
  {
    name: 'alphabetical descending inversion',
    code: `import { AAA, BB, C } from 'module'`,
    options: [{ order: 'desc' }],
  },
  {
    name: 'natural numeric order',
    code: `import { item1, item2, item10 } from 'module'`,
    options: [{ type: 'natural' }],
  },
  {
    name: 'natural numeric inversion',
    code: `import { item10, item2, item1 } from 'module'`,
    options: [{ type: 'natural' }],
  },
  {
    name: 'natural leading zero order',
    code: `import { item01, item1, item2 } from 'module'`,
    options: [{ type: 'natural' }],
  },
  {
    name: 'natural leading zero inversion',
    code: `import { item2, item1, item01 } from 'module'`,
    options: [{ type: 'natural' }],
  },
  {
    name: 'natural descending inversion',
    code: `import { item1, item2, item10 } from 'module'`,
    options: [{ type: 'natural', order: 'desc' }],
  },
  {
    name: 'line length descending',
    code: `import { AAA, BB, C } from 'module'`,
    options: [{ type: 'line-length', order: 'desc' }],
  },
  {
    name: 'line length descending inversion',
    code: `import { C, AAA, BB } from 'module'`,
    options: [{ type: 'line-length', order: 'desc' }],
  },
  {
    name: 'line length ascending',
    code: `import { C, BB, AAA } from 'module'`,
    options: [{ type: 'line-length', order: 'asc' }],
  },
  {
    name: 'line length counts alias source',
    code: `import { long as A, B } from 'module'`,
    options: [{ type: 'line-length', order: 'asc' }],
  },
  {
    name: 'custom reverse alphabet',
    code: `import { c, b, a } from 'module'`,
    options: [{ type: 'custom', alphabet: 'cba' }],
  },
  {
    name: 'custom reverse alphabet inversion',
    code: `import { a, b, c } from 'module'`,
    options: [{ type: 'custom', alphabet: 'cba' }],
  },
  {
    name: 'custom unknown characters remain stable',
    code: `import { x, y, z } from 'module'`,
    options: [{ type: 'custom', alphabet: 'abc' }],
  },
  {
    name: 'custom unknown characters compare length',
    code: `import { xx, y, zzz } from 'module'`,
    options: [{ type: 'custom', alphabet: 'abc' }],
  },
  {
    name: 'custom descending inversion',
    code: `import { c, b, a } from 'module'`,
    options: [{ type: 'custom', alphabet: 'abc', order: 'desc' }],
  },
  {
    name: 'unsorted preserves arbitrary order',
    code: `import { c, a, b } from 'module'`,
    options: [{ type: 'unsorted' }],
  },
  {
    name: 'unsorted ignores configured fallback',
    code: `import { c, a, b } from 'module'`,
    options: [{ type: 'unsorted', fallbackSort: { type: 'alphabetical' } }],
  },
  {
    name: 'special characters keep identifier punctuation',
    code: `import { _a, $a, a } from 'module'`,
  },
  {
    name: 'special characters trim',
    code: `import { _a, b, _c } from 'module'`,
    options: [{ specialCharacters: 'trim' }],
  },
  {
    name: 'special characters trim inversion',
    code: `import { _b, a, _c } from 'module'`,
    options: [{ specialCharacters: 'trim' }],
  },
  {
    name: 'special characters remove stable equality',
    code: `import { ab, a_b } from 'module'`,
    options: [{ specialCharacters: 'remove' }],
  },
  {
    name: 'Chinese locale order',
    code: `import { 你好, 世界, a, A, b, B } from 'module'`,
    options: [{ locales: 'zh-CN' }],
  },
  {
    name: 'Chinese locale inversion',
    code: `import { b, B, a, A, 世界, 你好 } from 'module'`,
    options: [{ locales: 'zh-CN' }],
  },
  {
    name: 'locale preference array',
    code: `import { 你好, 世界, a, A, b, B } from 'module'`,
    options: [{ locales: ['zh-CN', 'en-US'] }],
  },
  {
    name: 'Swedish locale inversion',
    code: `import { ä, å, z, a } from 'module'`,
    options: [{ locales: 'sv' }],
  },
  {
    name: 'fallback resolves case-insensitive tie',
    code: `import { bb as a, a as A } from 'module'`,
    options: [
      {
        fallbackSort: { type: 'line-length', order: 'asc' },
      },
    ],
  },
  {
    name: 'fallback descending resolves line-length tie',
    code: `import { c, a, b } from 'module'`,
    options: [
      {
        type: 'line-length',
        order: 'asc',
        fallbackSort: { type: 'alphabetical', order: 'desc' },
      },
    ],
  },
  {
    name: 'type-only import',
    code: `import type { A, B } from 'module'`,
    filename: 'fixture.ts',
  },
  {
    name: 'type specifier inversion',
    code: `import { type B, type A } from 'module'`,
    filename: 'fixture.ts',
  },
  {
    name: 'arbitrary string import inversion',
    code: `import { "B" as b, "A" as a } from 'module'`,
    options: [{ ignoreAlias: true }],
    filename: 'fixture.ts',
  },
  {
    name: 'multiple declarations',
    code: `import { B, A } from 'one';\nimport { D, C } from 'two';`,
  },
  {
    name: 'CRLF multiline inversion',
    code: `import {\r\n  B,\r\n  A,\r\n} from 'module';`,
  },
  {
    name: 'UTF-16 prefix keeps fix offsets',
    code: `'😀';\nimport { B, A } from 'module';`,
  },
  {
    name: 'inline trailing comma inversion',
    code: `import { B, A, } from 'module';`,
  },
  {
    name: 'multiline without trailing comma',
    code: `import {\n  B,\n  A\n} from 'module';`,
  },
  {
    name: 'leading comments move with imports',
    code: `import {\n  // B docs\n  B,\n  // A docs\n  A,\n} from 'module';`,
  },
  {
    name: 'trailing comments stay attached to slots',
    code: `import {\n  B, // first\n  A, // second\n} from 'module';`,
  },
];

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

const runnerDirectory = mkdtempSync(join(tmpdir(), 'perfectionist-named-imports-'));
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
        'Curated exact parity for the supported scalar comparator option subset; grouping, partition, newline, and conditional options are intentionally excluded.',
      tool: 'tools/tasks/sync-perfectionist-sort-named-imports-options-tests.ts',
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
    files: ['**/*.{js,jsx,ts,tsx}'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: { jsx: true },
      },
    },
    plugins: { perfectionist },
    rules: {
      'perfectionist/${RULE}': ['error', ...(testCase.options ?? [])],
    },
  }];
}

const captured = cases.map(testCase => {
  const filename = testCase.filename ?? 'fixture.tsx';
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
  return match ? { right: match[1], left: match[2] } : {};
}
`;
}
