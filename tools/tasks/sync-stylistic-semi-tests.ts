// Captures both stable @stylistic/semi v5.10.0 RuleTester suites from the
// exact pinned upstream commit, then enriches them with exact diagnostics,
// individual fixes, single-pass output, and recursive output from the
// published package.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type Capture = {
  lang?: string;
  name: string;
  valid: RawCase[];
  invalid: RawCase[];
};
type NormalizedCase = {
  code: string;
  language: 'js' | 'ts';
  options: unknown[];
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
const RULE = 'semi';
const VERSION = '5.10.0';
const PINNED_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '10.4.1';
const TYPESCRIPT_ESLINT_VERSION = '8.60.0';
const SOURCE_FILES = [
  `packages/eslint-plugin/rules/${RULE}/${RULE}._js_.test.ts`,
  `packages/eslint-plugin/rules/${RULE}/${RULE}._ts_.test.ts`,
] as const;
const CAPTURE_KEY = '__stylisticSemiCapture__';

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
  throw new Error(
    `Expected ${plugin.submodule} at ${PINNED_COMMIT}, received ${actualCommit}. ` +
      `Run \`git submodule update ${plugin.submodule}\`.`,
  );
}

registerCaptureHooks();
const captureDir = mkdtempSync(join(tmpdir(), 'stylistic-semi-capture-'));

try {
  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  for (const [index, sourceFile] of SOURCE_FILES.entries()) {
    const captureFile = join(captureDir, `${RULE}-${index}.test.ts`);
    const source = execFileSync(
      'git',
      ['-C', submodule, 'show', `${PINNED_COMMIT}:${sourceFile}`],
      { encoding: 'utf8' },
    );
    writeFileSync(captureFile, source);
    await import(`${pathToFileURL(captureFile).href}?commit=${PINNED_COMMIT}-${index}`);
  }

  const captures = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as Capture[];
  if (captures.length !== SOURCE_FILES.length || captures.some((run) => run.name !== RULE)) {
    throw new Error(
      `Expected ${SOURCE_FILES.length} captured ${RULE} suites, received ` +
        captures.map((run) => run.name).join(', '),
    );
  }

  const captured = {
    valid: captures.flatMap((run, suiteIndex) =>
      run.valid.map((testCase, caseIndex) =>
        normalizeCase(
          testCase,
          false,
          suiteIndex === 0 ? 'js' : 'ts',
          `suite ${suiteIndex} valid ${caseIndex}`,
        ),
      ),
    ),
    invalid: captures.flatMap((run, suiteIndex) =>
      run.invalid.map((testCase, caseIndex) =>
        normalizeCase(
          testCase,
          true,
          suiteIndex === 0 ? 'js' : 'ts',
          `suite ${suiteIndex} invalid ${caseIndex}`,
        ),
      ),
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
      sourceFiles: SOURCE_FILES,
      license: plugin.license,
      eslintVersion: ESLINT_VERSION,
      typescriptEslintParserVersion: TYPESCRIPT_ESLINT_VERSION,
      tool: 'tools/tasks/sync-stylistic-semi-tests.ts',
      inventory: {
        valid: enriched.valid.length,
        invalid: enriched.invalid.length,
        diagnostics,
        unfixableInvalid,
        total: enriched.valid.length + enriched.invalid.length,
        fixableInvalid: enriched.invalid.length - unfixableInvalid,
        javascript: {
          valid: enriched.valid.filter((testCase) => testCase.language === 'js').length,
          invalid: enriched.invalid.filter((testCase) => testCase.language === 'js').length,
        },
        typescript: {
          valid: enriched.valid.filter((testCase) => testCase.language === 'ts').length,
          invalid: enriched.invalid.filter((testCase) => testCase.language === 'ts').length,
        },
      },
    },
    valid: enriched.valid,
    invalid: enriched.invalid,
  };

  const fixturesDir = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures');
  mkdirSync(fixturesDir, { recursive: true });
  const fixturePath = join(fixturesDir, `${RULE}-v${VERSION}.json`);
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

function normalizeCase(
  raw: RawCase,
  invalid: boolean,
  language: 'js' | 'ts',
  label: string,
): NormalizedCase {
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

  const normalized: NormalizedCase = {
    code: value.code,
    language,
    options: Array.isArray(value.options) ? clone(value.options) : [],
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
  const runnerDir = mkdtempSync(join(tmpdir(), 'stylistic-semi-upstream-'));
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

function filenameFor(testCase) {
  return testCase.language === 'ts' ? 'fixture.ts' : 'fixture.js';
}

function configFor(testCase) {
  const parserOptions = {
    ecmaVersion: 'latest',
    sourceType: 'module',
    ...(testCase.parserOptions ?? {}),
  };
  return [{
    files: [testCase.language === 'ts' ? '**/*.ts' : '**/*.js'],
    languageOptions: {
      ...(testCase.language === 'ts' ? { parser: tsParser } : {}),
      parserOptions,
    },
    plugins: {
      stylistic: { rules: { '${RULE}': rule } },
    },
    rules: {
      'stylistic/${RULE}': ['error', ...testCase.options],
    },
  }];
}

function verify(testCase) {
  currentSource = testCase.code;
  return new Linter().verify(testCase.code, configFor(testCase), {
    filename: filenameFor(testCase),
  }).filter(message => message.ruleId === 'stylistic/${RULE}');
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
  const targetErrors = testCase.upstreamErrors.filter(
    error => error.messageId === 'missingSemi' || error.messageId === 'extraSemi',
  );
  const expectedIds = targetErrors.map(error => error.messageId);
  const actualIds = messages.map(message => message.messageId);
  if (JSON.stringify(actualIds) !== JSON.stringify(expectedIds)) {
    throw new Error(
      'Published rule IDs differ for invalid case ' + index + ': expected '
      + JSON.stringify(expectedIds) + ', received ' + JSON.stringify(actualIds),
    );
  }
  for (const [messageIndex, expected] of targetErrors.entries()) {
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
  const recursiveOutput = fixed.fixed ? fixed.output : null;

  return {
    code: testCase.code,
    language: testCase.language,
    options: testCase.options,
    ...(testCase.parserOptions ? { parserOptions: testCase.parserOptions } : {}),
    output,
    recursiveOutput,
    errors: targetErrors,
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
    `export const $ = ${unindent.toString()};`,
  ].join('\n');

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///stylistic-test', shortCircuit: true };
      }
      if (specifier === `./${RULE}`) {
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

function unindent(value: string | TemplateStringsArray): string {
  const lines = (typeof value === 'string' ? value : value[0]).split('\n');
  const whitespaceLines = lines.map((line) => /^\s*$/.test(line));
  const commonIndent = lines.reduce((minimum, line, index) => {
    if (whitespaceLines[index]) {
      return minimum;
    }
    return Math.min(minimum, line.match(/^\s*/)?.[0].length ?? minimum);
  }, Number.POSITIVE_INFINITY);
  let emptyLinesHead = 0;
  while (emptyLinesHead < lines.length && whitespaceLines[emptyLinesHead]) {
    emptyLinesHead += 1;
  }
  let emptyLinesTail = 0;
  while (emptyLinesTail < lines.length && whitespaceLines[lines.length - emptyLinesTail - 1]) {
    emptyLinesTail += 1;
  }
  return lines
    .slice(emptyLinesHead, lines.length - emptyLinesTail)
    .map((line) => line.slice(commonIndent))
    .join('\n');
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
