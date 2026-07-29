// Captures every stable @stylistic/object-curly-newline v5.10.0 fixture from
// the exact pinned upstream commit. Both JavaScript runs (including the Flow
// compatibility block) and the TypeScript run are part of the contract.

import { execFileSync } from 'node:child_process';
import { registerHooks } from 'node:module';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type Capture = {
  name: string;
  valid: RawCase[];
  invalid: RawCase[];
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
const RULE = 'object-curly-newline';
const PINNED_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const CAPTURE_KEY = '__stylisticObjectCurlyNewlineCaptures__';
const SOURCE_FILES = [`${RULE}._js_.test.ts`, `${RULE}._ts_.test.ts`] as const;
const MESSAGES = {
  unexpectedLinebreakBeforeClosingBrace: 'Unexpected line break before this closing brace.',
  unexpectedLinebreakAfterOpeningBrace: 'Unexpected line break after this opening brace.',
  expectedLinebreakBeforeClosingBrace: 'Expected a line break before this closing brace.',
  expectedLinebreakAfterOpeningBrace: 'Expected a line break after this opening brace.',
} as const;

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

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === '#test') {
      return { url: 'stub:///stylistic-test', shortCircuit: true };
    }
    if (specifier === '#test/parsers-flow') {
      return { url: 'stub:///stylistic-flow-parser', shortCircuit: true };
    }
    if (specifier === `./${RULE}`) {
      return { url: 'stub:///stylistic-rule', shortCircuit: true };
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url === 'stub:///stylistic-test') {
      return {
        format: 'module',
        source: [
          `const captureKey = ${JSON.stringify(CAPTURE_KEY)};`,
          'export const skipBabel = false;',
          'export function run(config) { globalThis[captureKey].push(config); }',
          `export const $ = ${unindent.toString()};`,
        ].join('\n'),
        shortCircuit: true,
      };
    }
    if (url === 'stub:///stylistic-flow-parser') {
      return {
        format: 'module',
        source:
          "export const languageOptionsForBabelFlow = { parser: { meta: { name: '@babel/eslint-parser' } } };",
        shortCircuit: true,
      };
    }
    if (url === 'stub:///stylistic-rule') {
      return {
        format: 'module',
        source: 'export default {};',
        shortCircuit: true,
      };
    }
    return nextLoad(url, context);
  },
});

const captures: Capture[] = [];
(globalThis as Record<string, unknown>)[CAPTURE_KEY] = captures;
for (const sourceFile of SOURCE_FILES) {
  await import(`${pathToFileURL(join(ruleRoot, sourceFile)).href}?commit=${PINNED_COMMIT}`);
}
if (
  captures.length !== 3 ||
  captures.some((capture) => capture.name !== RULE && capture.name !== `${RULE}_babel`)
) {
  throw new Error(
    `Expected JavaScript, Babel Flow, and TypeScript ${RULE} runs, received ${captures
      .map((capture) => capture.name)
      .join(', ')}`,
  );
}

const suites = captures.map((capture, suiteIndex) => {
  const sourceFile = suiteIndex < 2 ? SOURCE_FILES[0] : SOURCE_FILES[1];
  const language =
    capture.name === `${RULE}_babel`
      ? 'flow'
      : sourceFile.includes('_ts_')
        ? 'typescript'
        : 'javascript';
  const valid = capture.valid.map((testCase, index) =>
    normalizeCase(testCase, false, `${language} valid ${index}`),
  );
  const invalid = capture.invalid.map((testCase, index) =>
    normalizeCase(testCase, true, `${language} invalid ${index}`),
  );
  return {
    name: capture.name,
    language,
    sourceFile: `packages/eslint-plugin/rules/${RULE}/${sourceFile}`,
    valid,
    invalid,
  };
});

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
    tool: 'tools/tasks/sync-stylistic-object-curly-newline-tests.ts',
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
const fixturePath = join(fixturesDir, `${RULE}.json`);
writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
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
  const code = value.code;
  const allowed = new Set(
    invalid
      ? ['code', 'options', 'parserOptions', 'languageOptions', 'output', 'errors']
      : ['code', 'options', 'parserOptions', 'languageOptions'],
  );
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported ${label} keys: ${unsupported.join(', ')}`);
  }

  const normalized: Record<string, unknown> = {
    code: value.code,
    ...('options' in value ? { options: clone(value.options) } : {}),
    ...('parserOptions' in value ? { parserOptions: clone(value.parserOptions) } : {}),
  };
  if ('languageOptions' in value) {
    normalized.parser = 'babel-flow';
  }
  if (!invalid) {
    return normalized;
  }
  if (!('output' in value) || (typeof value.output !== 'string' && value.output !== null)) {
    throw new Error(`${label} is missing its fixed output`);
  }
  if (!Array.isArray(value.errors)) {
    throw new Error(`${label} is missing its ordered errors`);
  }
  const errors = value.errors.map((error, errorIndex) =>
    normalizeError(error, code, `${label} error ${errorIndex}`),
  );
  return {
    ...normalized,
    output: value.output,
    errors: errors.map((error) => error.upstream),
    expectedDiagnostics: errors.map((error) => error.exact),
  };
}

function normalizeError(error: unknown, code: string, label: string) {
  if (!error || typeof error !== 'object') {
    throw new Error(`Unsupported ${label}`);
  }
  const raw = clone(error) as Record<string, unknown>;
  const allowed = new Set(['messageId', 'line', 'column', 'endLine', 'endColumn', 'type']);
  const unsupported = Object.keys(raw).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported ${label} keys: ${unsupported.join(', ')}`);
  }
  const messageId = raw.messageId;
  if (typeof messageId !== 'string' || !(messageId in MESSAGES)) {
    throw new Error(`Unknown messageId ${String(messageId)} in ${label}`);
  }
  if (typeof raw.line !== 'number' || typeof raw.column !== 'number') {
    throw new Error(`${label} requires an exact upstream line and column`);
  }
  const start = offsetAt(code, raw.line, raw.column);
  const end = start + 1;
  const endPosition = positionAt(code, end);
  return {
    upstream: {
      messageId,
      ...('type' in raw ? { type: raw.type } : {}),
      line: raw.line,
      column: raw.column,
      ...('endLine' in raw ? { endLine: raw.endLine } : {}),
      ...('endColumn' in raw ? { endColumn: raw.endColumn } : {}),
    },
    exact: {
      messageId,
      message: MESSAGES[messageId as keyof typeof MESSAGES],
      data: {},
      range: [start, end],
      loc: {
        line: raw.line,
        column: raw.column,
        endLine: endPosition.line,
        endColumn: endPosition.column,
      },
    },
  };
}

function offsetAt(source: string, targetLine: number, targetColumn: number) {
  const lines = sourceLines(source);
  const line = lines[targetLine - 1];
  if (!line) {
    throw new Error(`Line ${targetLine} is outside the source`);
  }
  const offset = line.start + targetColumn - 1;
  if (offset >= line.endWithTerminator) {
    throw new Error(`Column ${targetColumn} is outside line ${targetLine}`);
  }
  return offset;
}

function positionAt(source: string, offset: number) {
  const lines = sourceLines(source);
  const index = Math.max(
    0,
    lines.findIndex((line, lineIndex) => offset <= line.end || lineIndex === lines.length - 1),
  );
  return {
    line: index + 1,
    column: offset - lines[index].start + 1,
  };
}

function sourceLines(source: string) {
  const lines: Array<{ start: number; end: number; endWithTerminator: number }> = [];
  let start = 0;
  for (let index = 0; index < source.length; index++) {
    const char = source[index];
    if (char !== '\n' && char !== '\r' && char !== '\u2028' && char !== '\u2029') {
      continue;
    }
    const terminatorLength = char === '\r' && source[index + 1] === '\n' ? 2 : 1;
    lines.push({ start, end: index, endWithTerminator: index + terminatorLength });
    index += terminatorLength - 1;
    start = index + 1;
  }
  lines.push({ start, end: source.length, endWithTerminator: source.length });
  return lines;
}

function unindent(value: string | TemplateStringsArray) {
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

function clone(value: unknown): unknown {
  return JSON.parse(JSON.stringify(value));
}
