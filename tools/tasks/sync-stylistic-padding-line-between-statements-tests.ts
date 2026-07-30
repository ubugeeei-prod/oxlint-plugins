// Captures both stable @stylistic/padding-line-between-statements RuleTester
// suites from the pinned v5.10.0 submodule, then asks the published v5.10.0
// rule for exact diagnostics, first-pass fixes, and converged fixed output.
//
// Re-run with `pnpm run port:tests:stylistic:padding-line-between-statements`.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type Language = 'javascript' | 'typescript';
type RawCase = string | Record<string, unknown>;
type Capture = { name: string; valid: RawCase[]; invalid: RawCase[] };
type FixtureCase = Record<string, unknown> & {
  code: string;
  language: Language;
  options: unknown[];
};
type FixtureCases = { valid: FixtureCase[]; invalid: FixtureCase[] };

const ROOT = process.cwd();
const RULE = 'padding-line-between-statements';
const VERSION = '5.10.0';
const REF = `v${VERSION}`;
const COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '10.4.1';
const TS_PARSER_VERSION = '8.60.0';
const UPSTREAM = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILES = [
  `packages/eslint-plugin/rules/${RULE}/${RULE}._js_.test.ts`,
  `packages/eslint-plugin/rules/${RULE}/${RULE}._ts_.test.ts`,
] as const;
const FIXTURE = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures', `${RULE}-v${VERSION}.json`);
const CAPTURE_KEY = '__stylisticPaddingLineBetweenStatementsCapture__';

if (!existsSync(join(UPSTREAM, '.git'))) {
  throw new Error(
    `Upstream checkout not found at ${UPSTREAM}. Run \`git submodule update --init upstream/eslint-stylistic\`.`,
  );
}
const actualCommit = execFileSync('git', ['-C', UPSTREAM, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== COMMIT) {
  throw new Error(`Expected eslint-stylistic ${COMMIT}, received ${actualCommit}.`);
}

registerCaptureHooks();
const captureDir = mkdtempSync(join(tmpdir(), 'stylistic-padding-line-capture-'));
const captured: FixtureCases = { valid: [], invalid: [] };
try {
  for (const [index, sourceFile] of SOURCE_FILES.entries()) {
    const language: Language = index === 0 ? 'javascript' : 'typescript';
    const captureFile = join(captureDir, `${language}.test.ts`);
    writeFileSync(
      captureFile,
      execFileSync('git', ['-C', UPSTREAM, 'show', `${COMMIT}:${sourceFile}`], {
        encoding: 'utf8',
      }),
    );
    (globalThis as Record<string, unknown>)[CAPTURE_KEY] = undefined;
    await import(`${pathToFileURL(captureFile).href}?capture=${Date.now()}-${index}`);
    const run = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as Capture | undefined;
    if (!run || run.name !== RULE) {
      throw new Error(`Expected captured ${RULE} suite from ${sourceFile}.`);
    }
    captured.valid.push(...run.valid.map((testCase) => normalizeCase(testCase, language)));
    captured.invalid.push(...run.invalid.map((testCase) => normalizeCase(testCase, language)));
  }
} finally {
  rmSync(captureDir, { recursive: true, force: true });
}

const enriched = enrichWithPublishedRule(captured);
const diagnostics = enriched.invalid.reduce(
  (total, testCase) => total + (testCase.errors as unknown[]).length,
  0,
);
const fixableInvalid = enriched.invalid.filter((testCase) => testCase.output !== null).length;
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: REF,
    commit: COMMIT,
    sourceFiles: SOURCE_FILES,
    license: 'MIT',
    eslintVersion: ESLINT_VERSION,
    typescriptEslintParserVersion: TS_PARSER_VERSION,
    tool: 'tools/tasks/sync-stylistic-padding-line-between-statements-tests.ts',
    inventory: {
      valid: enriched.valid.length,
      invalid: enriched.invalid.length,
      diagnostics,
      fixableInvalid,
      unfixableInvalid: enriched.invalid.length - fixableInvalid,
      total: enriched.valid.length + enriched.invalid.length,
    },
  },
  valid: enriched.valid,
  invalid: enriched.invalid,
};

mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
writeFileSync(FIXTURE, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('pnpm', ['exec', 'vp', 'fmt', FIXTURE], { cwd: ROOT, stdio: 'inherit' });
console.log(
  `Synced ${RULE} ${REF}: ${enriched.valid.length} valid, ` +
    `${enriched.invalid.length} invalid, ${diagnostics} diagnostics.`,
);

function normalizeCase(raw: RawCase, language: Language): FixtureCase {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (typeof value.code !== 'string') {
    throw new TypeError(`Captured ${RULE} case is missing string code.`);
  }
  const clone = JSON.parse(JSON.stringify(value)) as FixtureCase;
  clone.language = language;
  clone.options = Array.isArray(clone.options) ? clone.options : [];
  return clone;
}

function enrichWithPublishedRule(cases: FixtureCases): FixtureCases {
  const runnerDir = mkdtempSync(join(tmpdir(), 'stylistic-padding-line-upstream-'));
  try {
    writeFileSync(
      join(runnerDir, 'package.json'),
      `${JSON.stringify(
        {
          private: true,
          type: 'module',
          dependencies: {
            '@stylistic/eslint-plugin': VERSION,
            '@typescript-eslint/parser': TS_PARSER_VERSION,
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
    return JSON.parse(
      execFileSync(
        'node',
        [
          '-e',
          "process.stdout.write(require('fs').readFileSync(process.argv[1]))",
          join(runnerDir, 'enriched.json'),
        ],
        { encoding: 'utf8' },
      ),
    ) as FixtureCases;
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

function configFor(testCase) {
  const parserOptions = { ...(testCase.parserOptions ?? {}) };
  const sourceType = parserOptions.sourceType ?? 'module';
  delete parserOptions.sourceType;
  return [{
    files: ['**/*.{js,jsx,ts}'],
    languageOptions: {
      ecmaVersion: parserOptions.ecmaVersion ?? 'latest',
      sourceType,
      ...(testCase.language === 'typescript' ? { parser: tsParser } : {}),
      parserOptions,
    },
    plugins: { stylistic: { rules: { '${RULE}': rule } } },
    rules: { 'stylistic/${RULE}': ['error', ...testCase.options] },
  }];
}

function filenameFor(testCase) {
  if (testCase.language === 'typescript') return 'fixture.ts';
  return testCase.parserOptions?.ecmaFeatures?.jsx ? 'fixture.jsx' : 'fixture.js';
}

function diagnostic(message) {
  return {
    messageId: message.messageId,
    message: message.message,
    line: message.line,
    column: message.column,
    endLine: message.endLine,
    endColumn: message.endColumn,
    ...(message.fix
      ? { fix: { range: message.fix.range, text: message.fix.text } }
      : { fix: null }),
  };
}

function verify(testCase) {
  return new Linter().verify(testCase.code, configFor(testCase), {
    filename: filenameFor(testCase),
  });
}

const valid = captured.valid.map((testCase, index) => {
  const messages = verify(testCase);
  if (messages.length !== 0) {
    throw new Error('Published rule reported valid case ' + index + ': ' + JSON.stringify(messages));
  }
  return testCase;
});

const invalid = captured.invalid.map((testCase, index) => {
  const messages = verify(testCase);
  const expectedIds = testCase.errors.map(error => error.messageId);
  const actualIds = messages.map(message => message.messageId);
  if (JSON.stringify(actualIds) !== JSON.stringify(expectedIds)) {
    throw new Error(
      'Published rule IDs differ for invalid case ' + index + ': expected '
      + JSON.stringify(expectedIds) + ', received ' + JSON.stringify(actualIds)
      + ', case ' + JSON.stringify(testCase),
    );
  }
  const fixed = new Linter().verifyAndFix(testCase.code, configFor(testCase), {
    filename: filenameFor(testCase),
  });
  const output = fixed.fixed ? fixed.output : null;
  if (Object.prototype.hasOwnProperty.call(testCase, 'output') && testCase.output !== output) {
    throw new Error(
      'Published rule output differs for invalid case ' + index + ': expected '
      + JSON.stringify(testCase.output) + ', received ' + JSON.stringify(output),
    );
  }
  return { ...testCase, errors: messages.map(diagnostic), output };
});

writeFileSync(join(here, 'enriched.json'), JSON.stringify({ valid, invalid }));
`;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = '${CAPTURE_KEY}';`,
    'export function run(options) { globalThis[key] = options; }',
    'const whitespaceOnly = /^\\s*$/;',
    'export function $(value) {',
    '  const source = typeof value === "string" ? value : value[0];',
    '  const lines = source.split("\\n");',
    '  const blank = lines.map((line) => whitespaceOnly.test(line));',
    '  const commonIndent = lines.reduce((min, line, index) => {',
    '    if (blank[index]) return min;',
    '    return Math.min(min, line.match(/^\\s*/)?.[0].length ?? min);',
    '  }, Number.POSITIVE_INFINITY);',
    '  let head = 0;',
    '  while (head < lines.length && blank[head]) head += 1;',
    '  let tail = 0;',
    '  while (tail < lines.length && blank[lines.length - tail - 1]) tail += 1;',
    '  return lines.slice(head, lines.length - tail)',
    '    .map((line) => line.slice(commonIndent)).join("\\n");',
    '}',
  ].join('\n');

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///test', shortCircuit: true };
      }
      if (specifier === `./${RULE}` || specifier === './types' || specifier === './types.d.ts') {
        return { url: 'stub:///rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///test') {
        return { format: 'module', source: testStub, shortCircuit: true };
      }
      if (url === 'stub:///rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}
