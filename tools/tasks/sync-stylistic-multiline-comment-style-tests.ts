// Captures the complete stable @stylistic/eslint-plugin fixture inventory from
// the pinned upstream submodule. The committed JSON preserves case order,
// options, message IDs/data, exact report locations/ranges, fixed output, and
// explicitly unfixable cases for exhaustive native-plugin replay.

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
type RawError = {
  messageId: string;
  line: number;
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
const RULE = 'multiline-comment-style';
const PINNED_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const CAPTURE_KEY = '__stylisticMultilineCommentStyleCapture__';
const MESSAGES = {
  expectedBlock: 'Expected a block comment instead of consecutive line comments.',
  expectedBareBlock: 'Expected a block comment without padding stars.',
  startNewline: "Expected a linebreak after '/*'.",
  endNewline: "Expected a linebreak before '*/'.",
  missingStar: "Expected a '*' at the start of this line.",
  alignment: 'Expected this line to be aligned with the start of the comment.',
  expectedLines: 'Expected multiple line comments instead of a block comment.',
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
const sourceFile = join(packageRoot, 'rules', RULE, `${RULE}.test.ts`);
if (!existsSync(sourceFile)) {
  throw new Error(`Upstream fixture source is missing: ${sourceFile}`);
}

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
      return {
        format: 'module',
        source: [
          `const captureKey = ${JSON.stringify(CAPTURE_KEY)};`,
          'export function run(config) { globalThis[captureKey] = config; }',
          `export const $ = ${unindent.toString()};`,
        ].join('\n'),
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

(globalThis as Record<string, unknown>)[CAPTURE_KEY] = undefined;
await import(`${pathToFileURL(sourceFile).href}?commit=${PINNED_COMMIT}`);
const captured = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as Capture | undefined;
if (!captured || captured.name !== RULE) {
  throw new Error(`Did not capture the upstream ${RULE} run() block`);
}

const valid = captured.valid.map((testCase, index) => normalizeValid(testCase, index));
const invalid = captured.invalid.map((testCase, index) => normalizeInvalid(testCase, index));
const diagnosticCount = invalid.reduce(
  (count, testCase) => count + testCase.expectedDiagnostics.length,
  0,
);
const unfixableCount = invalid.filter((testCase) => testCase.output === null).length;
const fixture = {
  __generated: {
    source: plugin.npm,
    version: plugin.baselineVersion,
    sourceCommit: PINNED_COMMIT,
    sourceFile: `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`,
    license: plugin.license,
    tool: 'tools/tasks/sync-stylistic-multiline-comment-style-tests.ts',
    inventory: {
      valid: valid.length,
      invalid: invalid.length,
      total: valid.length + invalid.length,
      diagnostics: diagnosticCount,
      fixableInvalid: invalid.length - unfixableCount,
      unfixableInvalid: unfixableCount,
    },
  },
  valid,
  invalid,
};

const fixturesDir = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures');
mkdirSync(fixturesDir, { recursive: true });
const fixturePath = join(fixturesDir, `${RULE}.json`);
writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
console.log(
  `Synced @stylistic/${RULE} v${plugin.baselineVersion} (${PINNED_COMMIT}): ` +
    `${valid.length} valid, ${invalid.length} invalid, ${diagnosticCount} diagnostics ` +
    `(${unfixableCount} unfixable).`,
);

function normalizeValid(raw: RawCase, index: number) {
  const value = normalizeCase(raw, false, index);
  return {
    code: value.code,
    ...('options' in value ? { options: clone(value.options) } : {}),
  };
}

function normalizeInvalid(raw: RawCase, index: number) {
  const value = normalizeCase(raw, true, index);
  if (!('output' in value) || (typeof value.output !== 'string' && value.output !== null)) {
    throw new Error(`Invalid case ${index} is missing its fixed output`);
  }
  if (!Array.isArray(value.errors)) {
    throw new Error(`Invalid case ${index} is missing its ordered errors`);
  }
  const errors = value.errors.map((error, errorIndex) =>
    normalizeError(error, value.code, index, errorIndex),
  );
  return {
    code: value.code,
    ...('options' in value ? { options: clone(value.options) } : {}),
    output: value.output,
    errors: errors.map((error) => ({
      messageId: error.messageId,
      line: error.line,
    })),
    expectedDiagnostics: errors.map(({ expected }) => expected),
  };
}

function normalizeCase(raw: RawCase, invalid: boolean, index: number) {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`Unsupported ${invalid ? 'invalid' : 'valid'} case at index ${index}`);
  }
  const allowed = new Set(invalid ? ['code', 'options', 'output', 'errors'] : ['code', 'options']);
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(
      `Unsupported ${invalid ? 'invalid' : 'valid'} case keys at index ${index}: ${unsupported.join(', ')}`,
    );
  }
  return value as Record<string, unknown> & { code: string };
}

function normalizeError(error: unknown, code: string, caseIndex: number, errorIndex: number) {
  if (!error || typeof error !== 'object') {
    throw new Error(`Unsupported error ${errorIndex} in invalid case ${caseIndex}`);
  }
  const raw = error as Partial<RawError> & Record<string, unknown>;
  const unsupported = Object.keys(raw).filter((key) => key !== 'messageId' && key !== 'line');
  if (unsupported.length > 0 || typeof raw.messageId !== 'string' || typeof raw.line !== 'number') {
    throw new Error(`Unsupported error ${errorIndex} in invalid case ${caseIndex}`);
  }
  if (!(raw.messageId in MESSAGES)) {
    throw new Error(`Unknown messageId ${raw.messageId} in invalid case ${caseIndex}`);
  }
  const range = expectedRange(code, raw.messageId as keyof typeof MESSAGES, raw.line);
  return {
    messageId: raw.messageId,
    line: raw.line,
    expected: {
      messageId: raw.messageId,
      message: MESSAGES[raw.messageId as keyof typeof MESSAGES],
      data: {},
      range: [range.start, range.end],
      loc: {
        line: range.startLine,
        column: range.startColumn,
        endLine: range.endLine,
        endColumn: range.endColumn,
      },
    },
  };
}

function expectedRange(code: string, messageId: keyof typeof MESSAGES, line: number) {
  const lines = sourceLines(code);
  const current = lines[line - 1];
  if (!current) {
    throw new Error(`Expected line ${line} is outside the source`);
  }

  let start: number;
  let end: number;
  let endLine = line;
  if (messageId === 'expectedBlock') {
    const column = current.text.indexOf('//');
    if (column < 0) {
      throw new Error(`Expected line-comment group at line ${line}`);
    }
    start = current.start + column;
    end = current.end;
    while (endLine < lines.length) {
      const next = lines[endLine];
      if (!next.text.includes('//')) {
        break;
      }
      end = next.end;
      endLine += 1;
    }
  } else if (
    messageId === 'expectedBareBlock' ||
    messageId === 'expectedLines' ||
    messageId === 'startNewline'
  ) {
    const column = current.text.indexOf('/*');
    if (column < 0) {
      throw new Error(`Expected block-comment start at line ${line}`);
    }
    start = current.start + column;
    end = start + 2;
  } else if (messageId === 'endNewline') {
    const column = current.text.lastIndexOf('*/');
    if (column < 0) {
      throw new Error(`Expected block-comment end at line ${line}`);
    }
    start = current.start + column;
    end = start + 2;
  } else {
    start = current.start;
    end = current.end;
  }

  const startPosition = positionAt(lines, start);
  const endPosition = positionAt(lines, end);
  return {
    start,
    end,
    startLine: startPosition.line,
    startColumn: startPosition.column,
    endLine: endPosition.line,
    endColumn: endPosition.column,
  };
}

function sourceLines(source: string) {
  const lines: Array<{ start: number; end: number; text: string }> = [];
  let start = 0;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (
      character !== '\n' &&
      character !== '\r' &&
      character !== '\u2028' &&
      character !== '\u2029'
    ) {
      continue;
    }
    lines.push({ start, end: index, text: source.slice(start, index) });
    if (character === '\r' && source[index + 1] === '\n') {
      index += 1;
    }
    start = index + 1;
  }
  lines.push({ start, end: source.length, text: source.slice(start) });
  return lines;
}

function positionAt(lines: ReturnType<typeof sourceLines>, offset: number) {
  const index = Math.max(
    0,
    lines.findIndex((line, lineIndex) => offset <= line.end || lineIndex === lines.length - 1),
  );
  return {
    line: index + 1,
    column: offset - lines[index].start + 1,
  };
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
