// Captures every stable @stylistic/object-property-newline v5.10.0 RuleTester
// case from the exact pinned source commit, then audits the captured cases
// against the published package for exact messages, locations, fixes, and
// recursively fixed output.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type Capture = {
  name: string;
  valid: RawCase[];
  invalid: RawCase[];
};
type Suite = {
  name: string;
  language: 'javascript' | 'typescript';
  sourceFile: string;
  valid: Array<Record<string, unknown>>;
  invalid: Array<Record<string, unknown>>;
};
type Manifest = {
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    packageSubdir?: string;
    baselineVersion: string;
    pinnedRef?: string;
    license: string;
  }>;
};

const ROOT = process.cwd();
const RULE = 'object-property-newline';
const PINNED_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '10.4.1';
const TYPESCRIPT_ESLINT_VERSION = '8.60.0';
const CAPTURE_KEY = '__stylisticObjectPropertyNewlineCaptures__';
const SOURCE_FILES = [`${RULE}._js_.test.ts`, `${RULE}._ts_.test.ts`] as const;

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-stylistic');
if (!plugin) {
  throw new Error('eslint-stylistic is not registered in tools/port-targets.json');
}
if (plugin.baselineVersion !== '5.10.0' || plugin.pinnedRef !== 'v5.10.0') {
  throw new Error(
    `Expected @stylistic v5.10.0 manifest pin, received ${plugin.baselineVersion} / ${plugin.pinnedRef}`,
  );
}

const submodule = join(ROOT, plugin.submodule);
if (!existsSync(join(submodule, '.git'))) {
  throw new Error(
    `Upstream checkout not found at ${submodule}. Run \`git submodule update --init ${plugin.submodule}\` first.`,
  );
}
const actualCommit = execFileSync('git', ['-C', submodule, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== PINNED_COMMIT) {
  throw new Error(
    `Expected ${plugin.submodule} at ${PINNED_COMMIT}, received ${actualCommit}. ` +
      `Run: git submodule update --init ${plugin.submodule}`,
  );
}

const packageRoot = join(submodule, plugin.packageSubdir ?? '.');
const ruleRoot = join(packageRoot, 'rules', RULE);
for (const sourceFile of SOURCE_FILES) {
  if (!existsSync(join(ruleRoot, sourceFile))) {
    throw new Error(`Upstream fixture source is missing: ${join(ruleRoot, sourceFile)}`);
  }
}

registerCaptureHooks();
const captureDirectory = mkdtempSync(join(tmpdir(), 'stylistic-object-property-capture-'));
const captures: Capture[] = [];
(globalThis as Record<string, unknown>)[CAPTURE_KEY] = captures;
try {
  for (const sourceFile of SOURCE_FILES) {
    const temporarySource = join(captureDirectory, sourceFile);
    writeFileSync(
      temporarySource,
      execFileSync(
        'git',
        [
          '-C',
          submodule,
          'show',
          `${PINNED_COMMIT}:packages/eslint-plugin/rules/${RULE}/${sourceFile}`,
        ],
        { encoding: 'utf8' },
      ),
    );
    await import(`${pathToFileURL(temporarySource).href}?commit=${PINNED_COMMIT}`);
  }
} finally {
  rmSync(captureDirectory, { recursive: true, force: true });
}

if (captures.length !== 2 || captures.some((capture) => capture.name !== RULE)) {
  throw new Error(
    `Expected JavaScript and TypeScript ${RULE} runs, received ${captures
      .map((capture) => capture.name)
      .join(', ')}`,
  );
}

const capturedSuites = captures.map((capture, index): Suite => {
  const language = index === 0 ? 'javascript' : 'typescript';
  const sourceFile = `packages/eslint-plugin/rules/${RULE}/${SOURCE_FILES[index]}`;
  return {
    name: capture.name,
    language,
    sourceFile,
    valid: capture.valid.map((testCase, caseIndex) =>
      normalizeCase(testCase, false, `${language} valid ${caseIndex}`),
    ),
    invalid: capture.invalid.map((testCase, caseIndex) =>
      normalizeCase(testCase, true, `${language} invalid ${caseIndex}`),
    ),
  };
});
const suites = enrichWithPublishedRule(capturedSuites);
const inventory = suites.reduce(
  (counts, suite) => {
    counts.valid += suite.valid.length;
    counts.invalid += suite.invalid.length;
    counts.diagnostics += suite.invalid.reduce(
      (total, testCase) =>
        total +
        (
          testCase as {
            expectedDiagnostics: unknown[];
          }
        ).expectedDiagnostics.length,
      0,
    );
    counts.unfixableInvalid += suite.invalid.filter(
      (testCase) => (testCase as { output: string | null }).output === null,
    ).length;
    return counts;
  },
  { valid: 0, invalid: 0, diagnostics: 0, unfixableInvalid: 0 },
);
const fixture = {
  __generated: {
    source: plugin.npm,
    version: plugin.baselineVersion,
    sourceCommit: PINNED_COMMIT,
    sourceFiles: suites.map((suite) => suite.sourceFile),
    license: plugin.license,
    eslintVersion: ESLINT_VERSION,
    typescriptEslintParserVersion: TYPESCRIPT_ESLINT_VERSION,
    tool: 'tools/tasks/sync-stylistic-object-property-newline-tests.ts',
    inventory: {
      ...inventory,
      total: inventory.valid + inventory.invalid,
      fixableInvalid: inventory.invalid - inventory.unfixableInvalid,
    },
  },
  suites,
};

const fixturesDir = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures');
mkdirSync(fixturesDir, { recursive: true });
const fixturePath = join(fixturesDir, `${RULE}-v${plugin.baselineVersion}.json`);
writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('pnpm', ['exec', 'vp', 'fmt', fixturePath], {
  cwd: ROOT,
  stdio: 'inherit',
});
console.log(
  `Synced @stylistic/${RULE} v${plugin.baselineVersion} (${PINNED_COMMIT}): ` +
    `${inventory.valid} valid, ${inventory.invalid} invalid, ${inventory.diagnostics} diagnostics ` +
    `(${inventory.unfixableInvalid} unfixable).`,
);

function normalizeCase(raw: RawCase, invalid: boolean, label: string) {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`Unsupported ${label}`);
  }
  const allowed = new Set(
    invalid
      ? ['code', 'options', 'parserOptions', 'output', 'errors']
      : ['code', 'options', 'parserOptions'],
  );
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported ${label} keys: ${unsupported.join(', ')}`);
  }
  if (invalid && !Array.isArray(value.errors)) {
    throw new Error(`${label} is missing its ordered errors`);
  }

  return {
    code: value.code,
    options: Array.isArray(value.options) ? clone(value.options) : [],
    ...('parserOptions' in value ? { parserOptions: clone(value.parserOptions) } : {}),
    ...(invalid ? { upstreamErrors: clone(value.errors) } : {}),
  };
}

function enrichWithPublishedRule(suites: Suite[]): Suite[] {
  const runnerDirectory = mkdtempSync(join(tmpdir(), 'stylistic-object-property-upstream-'));
  try {
    writeFileSync(
      join(runnerDirectory, 'package.json'),
      `${JSON.stringify(
        {
          private: true,
          type: 'module',
          dependencies: {
            '@stylistic/eslint-plugin': plugin!.baselineVersion,
            '@typescript-eslint/parser': TYPESCRIPT_ESLINT_VERSION,
            eslint: ESLINT_VERSION,
          },
        },
        null,
        2,
      )}\n`,
    );
    writeFileSync(join(runnerDirectory, 'captured.json'), `${JSON.stringify(suites)}\n`);
    writeFileSync(join(runnerDirectory, 'runner.mjs'), enrichmentRunnerSource());
    execFileSync(
      'pnpm',
      ['install', '--dir', runnerDirectory, '--ignore-workspace', '--lockfile=false', '--silent'],
      { stdio: 'inherit' },
    );
    execFileSync('node', [join(runnerDirectory, 'runner.mjs')], { stdio: 'inherit' });
    return JSON.parse(readFileSync(join(runnerDirectory, 'enriched.json'), 'utf8')) as Suite[];
  } finally {
    rmSync(runnerDirectory, { recursive: true, force: true });
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
const suites = JSON.parse(readFileSync(join(here, 'captured.json'), 'utf8'));
const stylistic = stylisticModule.default ?? stylisticModule;
const rule = stylistic.rules['${RULE}'];

function configFor(testCase, language) {
  return [{
    files: ['**/*.{js,ts}'],
    languageOptions: {
      ecmaVersion: testCase.parserOptions?.ecmaVersion ?? 'latest',
      sourceType: 'module',
      ...(language === 'typescript' ? { parser: tsParser } : {}),
      ...(testCase.parserOptions ? { parserOptions: testCase.parserOptions } : {}),
    },
    plugins: {
      stylistic: { rules: { '${RULE}': rule } },
    },
    rules: {
      'stylistic/${RULE}': ['error', ...testCase.options],
    },
  }];
}

function verify(testCase, language) {
  return new Linter().verify(testCase.code, configFor(testCase, language), {
    filename: language === 'typescript' ? 'fixture.ts' : 'fixture.js',
  });
}

function offsetAt(sourceText, line, column) {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line && offset < sourceText.length) {
    const character = sourceText[offset];
    if (character === '\\r') {
      offset += sourceText[offset + 1] === '\\n' ? 2 : 1;
      currentLine += 1;
    } else if (character === '\\n' || character === '\\u2028' || character === '\\u2029') {
      offset += 1;
      currentLine += 1;
    } else {
      offset += 1;
    }
  }
  return offset + column - 1;
}

function exactDiagnostic(message, sourceText) {
  const start = offsetAt(sourceText, message.line, message.column);
  const end = offsetAt(sourceText, message.endLine, message.endColumn);
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

const enriched = suites.map((suite) => ({
  ...suite,
  valid: suite.valid.map((testCase, index) => {
    const messages = verify(testCase, suite.language);
    if (messages.length !== 0) {
      throw new Error(
        suite.language + ' valid case ' + index + ' reported: ' + JSON.stringify(messages),
      );
    }
    return testCase;
  }),
  invalid: suite.invalid.map((testCase, index) => {
    const messages = verify(testCase, suite.language);
    const expectedIds = testCase.upstreamErrors.map(error => error.messageId);
    const actualIds = messages.map(message => message.messageId);
    if (JSON.stringify(actualIds) !== JSON.stringify(expectedIds)) {
      throw new Error(
        suite.language + ' invalid case ' + index + ' IDs differ: expected ' +
        JSON.stringify(expectedIds) + ', received ' + JSON.stringify(actualIds),
      );
    }
    const fixed = new Linter().verifyAndFix(
      testCase.code,
      configFor(testCase, suite.language),
      { filename: suite.language === 'typescript' ? 'fixture.ts' : 'fixture.js' },
    );
    return {
      code: testCase.code,
      options: testCase.options,
      ...(testCase.parserOptions ? { parserOptions: testCase.parserOptions } : {}),
      upstreamErrors: testCase.upstreamErrors,
      expectedDiagnostics: messages.map(message => exactDiagnostic(message, testCase.code)),
      output: fixed.fixed ? fixed.output : null,
    };
  }),
}));

writeFileSync(join(here, 'enriched.json'), JSON.stringify(enriched));
`;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const captureKey = ${JSON.stringify(CAPTURE_KEY)};`,
    'export function run(config) { globalThis[captureKey].push(config); }',
  ].join('\n');

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///stylistic-test', shortCircuit: true };
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
      if (url === 'stub:///stylistic-rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}

function clone(value: unknown): unknown {
  return JSON.parse(JSON.stringify(value));
}
