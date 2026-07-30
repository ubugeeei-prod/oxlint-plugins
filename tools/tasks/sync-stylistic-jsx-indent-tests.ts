// Captures every authored stable @stylistic/jsx-indent v5.10.0 RuleTester
// semantic case once, then replays it through the exact published rule to
// record messages, data, locations, ranges, first-pass fixes, and recursive
// fix output. The upstream parser matrix duplicates compatible cases across
// parsers; the fixture intentionally stores each authored case once.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire, registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = {
  name: string;
  valid: RawCase[];
  invalid: RawCase[];
};
type LintMessage = {
  ruleId?: string | null;
  messageId?: string;
  message: string;
  line: number;
  column: number;
  endLine?: number;
  endColumn?: number;
  fix?: {
    range: [number, number];
    text: string;
  };
};
type ParserKind = 'typescript-eslint' | 'babel' | 'authored-unexercised';
type ReplayParsers = {
  typescript: unknown;
  babel: unknown;
  babelPresetReact: string;
  babelDoExpressions: string;
  babelFunctionBind: string;
  babelDecorators: string;
};

const ROOT = process.cwd();
const RULE = 'jsx-indent';
const UPSTREAM_VERSION = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '9.39.2';
const TYPESCRIPT_ESLINT_PARSER_VERSION = '8.60.0';
const BABEL_ESLINT_PARSER_VERSION = '7.28.6';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const RULE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.ts`;
const PARSER_MATRIX_FILE = 'shared/test-utils/parsers-jsx.ts';
const FIXTURE_FILE = join(
  ROOT,
  'npm',
  'stylistic',
  'test',
  'fixtures',
  `${RULE}-${UPSTREAM_VERSION}.json`,
);
const CAPTURE_KEY = '__stylisticJsxIndentCapture__';
const MESSAGE_PATTERN =
  /^Expected indentation of (-?\d+) (space|tab) (character|characters) but found (-?\d+)\.$/;

if (!existsSync(UPSTREAM_DIR)) {
  throw new Error(
    `Upstream checkout not found at ${UPSTREAM_DIR}. Initialize upstream/eslint-stylistic first.`,
  );
}
const actualCommit = execFileSync('git', ['-C', UPSTREAM_DIR, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== UPSTREAM_COMMIT) {
  throw new Error(`Expected eslint-stylistic at ${UPSTREAM_COMMIT}, received ${actualCommit}.`);
}

const source = upstreamFile(SOURCE_FILE);
const ruleSource = upstreamFile(RULE_FILE);
const parserMatrixSource = upstreamFile(PARSER_MATRIX_FILE);
for (const expected of [
  "description: 'Enforce JSX indentation. Deprecated, use `indent` rule instead.'",
  "fixable: 'whitespace'",
  'defaultOptions: [4]',
  "wrongIndent: 'Expected indentation of {{needed}} {{type}} {{characters}} but found {{gotten}}.'",
]) {
  if (!ruleSource.includes(expected)) {
    throw new Error(`Pinned ${RULE_FILE} no longer contains ${JSON.stringify(expected)}.`);
  }
}

registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-indent-sync-'));
const captureFile = join(tempDir, `${RULE}.test.ts`);
const replayDir = join(tempDir, 'exact-replay');

try {
  writeFileSync(captureFile, source);
  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  await import(`${pathToFileURL(captureFile).href}?commit=${UPSTREAM_COMMIT}`);
  const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
  if (runs.length !== 1 || runs[0]?.name !== RULE) {
    throw new Error(`Expected one captured ${RULE} suite, received ${runs.length}.`);
  }

  execFileSync(
    'npm',
    [
      'install',
      '--prefix',
      replayDir,
      '--ignore-scripts',
      '--no-audit',
      '--no-fund',
      '--legacy-peer-deps',
      `@stylistic/eslint-plugin@${UPSTREAM_VERSION.slice(1)}`,
      `eslint@${ESLINT_VERSION}`,
      `@typescript-eslint/parser@${TYPESCRIPT_ESLINT_PARSER_VERSION}`,
      'typescript@5.9.3',
      `@babel/eslint-parser@${BABEL_ESLINT_PARSER_VERSION}`,
      '@babel/core@7.28.5',
      '@babel/preset-react@7.28.5',
      '@babel/plugin-syntax-do-expressions@7.28.6',
      '@babel/plugin-syntax-function-bind@7.28.6',
      '@babel/plugin-syntax-decorators@7.28.6',
    ],
    { stdio: 'ignore' },
  );

  const require = createRequire(join(replayDir, 'package.json'));
  const { Linter } = require('eslint') as {
    Linter: new () => {
      verify(code: string, config: unknown[], options: { filename: string }): LintMessage[];
      verifyAndFix(
        code: string,
        config: unknown[],
        options: { filename: string },
      ): { fixed: boolean; output: string };
    };
  };
  const parsers = {
    typescript: require('@typescript-eslint/parser') as unknown,
    babel: require('@babel/eslint-parser') as unknown,
    babelPresetReact: require.resolve('@babel/preset-react'),
    babelDoExpressions: require.resolve('@babel/plugin-syntax-do-expressions'),
    babelFunctionBind: require.resolve('@babel/plugin-syntax-function-bind'),
    babelDecorators: require.resolve('@babel/plugin-syntax-decorators'),
  } satisfies ReplayParsers;
  const plugin = require('@stylistic/eslint-plugin') as unknown;
  const linter = new Linter();

  const valid = runs[0].valid.map((raw, index) => {
    const testCase = normalizeCase(raw, false, index);
    const parserKind = parserFor(testCase);
    if (parserKind !== 'authored-unexercised') {
      const messages = verify(linter, parsers, plugin, testCase, parserKind);
      if (messages.length !== 0) {
        throw new Error(
          `Published rule rejected authored valid case ${index} with ${parserKind}: ${messages[0].message}`,
        );
      }
    }
    return { ...testCase, parserKind };
  });

  const invalid = runs[0].invalid.map((raw, index) => {
    const testCase = normalizeCase(raw, true, index);
    const parserKind = parserFor(testCase);
    if (parserKind === 'authored-unexercised') {
      throw new Error(`Invalid authored case ${index} has no compatible upstream parser.`);
    }
    const messages = verify(linter, parsers, plugin, testCase, parserKind);
    const capturedErrors = testCase.errors as Array<Record<string, unknown>>;
    if (messages.length !== capturedErrors.length) {
      throw new Error(
        `Published rule produced ${messages.length} diagnostics for invalid case ${index}; expected ${capturedErrors.length}.`,
      );
    }

    const diagnostics = messages.map((message, messageIndex) => {
      if (message.messageId !== 'wrongIndent') {
        throw new Error(
          `Invalid case ${index} diagnostic ${messageIndex} has unexpected message ${String(message.messageId)}.`,
        );
      }
      const data = messageData(message.message);
      const authored = capturedErrors[messageIndex];
      if (authored.messageId && authored.messageId !== message.messageId) {
        throw new Error(`Invalid case ${index} diagnostic ${messageIndex} messageId diverged.`);
      }
      if (
        authored.message &&
        authored.message !== message.message &&
        !(authored.message instanceof RegExp)
      ) {
        throw new Error(`Invalid case ${index} diagnostic ${messageIndex} text diverged.`);
      }
      if (
        authored.data &&
        Object.entries(authored.data as Record<string, unknown>).some(
          ([key, value]) => (data as unknown as Record<string, unknown>)[key] !== value,
        )
      ) {
        throw new Error(
          `Invalid case ${index} diagnostic ${messageIndex} data diverged: authored=${JSON.stringify(authored.data)} actual=${JSON.stringify(data)}.`,
        );
      }
      if (typeof authored.line === 'number' && authored.line !== message.line) {
        throw new Error(`Invalid case ${index} diagnostic ${messageIndex} line diverged.`);
      }
      if (typeof authored.column === 'number' && authored.column !== message.column) {
        throw new Error(`Invalid case ${index} diagnostic ${messageIndex} column diverged.`);
      }
      return {
        messageId: 'wrongIndent',
        message: message.message,
        data,
        line: message.line,
        column: message.column,
        endLine: message.endLine,
        endColumn: message.endColumn,
        range: locationRange(testCase.code as string, message),
        fix: message.fix
          ? {
              range: message.fix.range,
              replacementText: message.fix.text,
            }
          : null,
      };
    });

    const firstPassOutput = applyFixes(testCase.code as string, diagnostics);
    const authoredOutput = typeof testCase.output === 'string' ? testCase.output : null;
    if (
      (authoredOutput !== null && firstPassOutput !== authoredOutput) ||
      (authoredOutput === null && firstPassOutput !== testCase.code)
    ) {
      throw new Error(`Published rule first-pass fix diverged for invalid case ${index}.`);
    }
    const recursiveOutput = linter.verifyAndFix(
      testCase.code as string,
      ruleConfig(parsers, plugin, testCase, parserKind),
      { filename: filenameFor(parserKind) },
    ).output;

    return {
      code: testCase.code,
      ...(Array.isArray(testCase.options) ? { options: testCase.options } : {}),
      ...(Array.isArray(testCase.features) ? { features: testCase.features } : {}),
      ...(testCase.parserOptions ? { parserOptions: testCase.parserOptions } : {}),
      parserKind,
      output: authoredOutput,
      recursiveOutput,
      diagnostics,
    };
  });

  const diagnosticCount = invalid.reduce(
    (count, testCase) => count + testCase.diagnostics.length,
    0,
  );
  const fixableInvalid = invalid.filter((testCase) =>
    testCase.diagnostics.some((diagnostic) => diagnostic.fix !== null),
  ).length;
  if (
    valid.length !== 106 ||
    invalid.length !== 65 ||
    diagnosticCount !== 84 ||
    fixableInvalid !== 65
  ) {
    throw new Error(
      `Unexpected authored inventory: ${valid.length} valid, ${invalid.length} invalid, ${diagnosticCount} diagnostics, ${fixableInvalid} fixable invalid.`,
    );
  }
  const fixture = {
    __generated: {
      source: '@stylistic/eslint-plugin',
      version: UPSTREAM_VERSION,
      commit: UPSTREAM_COMMIT,
      sourceFile: SOURCE_FILE,
      ruleFile: RULE_FILE,
      parserMatrixFile: PARSER_MATRIX_FILE,
      sourceSha256: sha256(source),
      ruleSourceSha256: sha256(ruleSource),
      parserMatrixSourceSha256: sha256(parserMatrixSource),
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-jsx-indent-tests.ts',
      capturePolicy:
        'Each authored semantic case is captured once; compatible cases are exactly replayed through the published rule.',
      exactReplay: {
        eslint: ESLINT_VERSION,
        typescriptEslintParser: TYPESCRIPT_ESLINT_PARSER_VERSION,
        babelEslintParser: BABEL_ESLINT_PARSER_VERSION,
      },
      inventory: {
        valid: valid.length,
        invalid: invalid.length,
        diagnostics: diagnosticCount,
        fixableInvalid,
        unfixableInvalid: invalid.length - fixableInvalid,
        total: valid.length + invalid.length,
      },
    },
    valid,
    invalid,
  };

  mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
  writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('vp', ['fmt', FIXTURE_FILE], { stdio: 'inherit' });
  console.log(
    `Synced ${RULE} ${UPSTREAM_VERSION}: ${valid.length} valid, ${invalid.length} invalid, ${diagnosticCount} diagnostics.`,
  );
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}

function upstreamFile(path: string): string {
  return execFileSync('git', ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${path}`], {
    encoding: 'utf8',
  });
}

function normalizeCase(raw: RawCase, invalid: boolean, index: number): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  const clone = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
  if (typeof clone.code !== 'string') {
    throw new TypeError(
      `Captured ${RULE} ${invalid ? 'invalid' : 'valid'} case ${index} has no code.`,
    );
  }
  if (invalid && !Array.isArray(clone.errors)) {
    throw new TypeError(`Captured ${RULE} invalid case ${index} has no errors array.`);
  }
  return clone;
}

function parserFor(testCase: Record<string, unknown>): ParserKind {
  const features = new Set(Array.isArray(testCase.features) ? testCase.features : []);
  if (features.has('flow')) {
    // The upstream parser matrix skips base, Babel, and TypeScript parsers for
    // this historical authored case, so it is pinned but was not executed.
    return 'authored-unexercised';
  }
  return features.has('do expressions') || features.has('bind operator')
    ? 'babel'
    : 'typescript-eslint';
}

function filenameFor(parserKind: Exclude<ParserKind, 'authored-unexercised'>): string {
  return parserKind === 'babel' ? 'fixture.jsx' : 'fixture.tsx';
}

function ruleConfig(
  parsers: ReplayParsers,
  plugin: unknown,
  testCase: Record<string, unknown>,
  parserKind: Exclude<ParserKind, 'authored-unexercised'>,
): unknown[] {
  const options = Array.isArray(testCase.options) ? testCase.options : [];
  const authoredParserOptions =
    testCase.parserOptions && typeof testCase.parserOptions === 'object'
      ? testCase.parserOptions
      : {};
  const parser =
    parserKind === 'babel'
      ? {
          parser: parsers.babel,
          parserOptions: {
            ...authoredParserOptions,
            requireConfigFile: false,
            babelOptions: {
              presets: [parsers.babelPresetReact],
              plugins: [
                parsers.babelDoExpressions,
                parsers.babelFunctionBind,
                [parsers.babelDecorators, { legacy: true }],
              ],
            },
            ecmaFeatures: { jsx: true },
          },
        }
      : {
          parser: parsers.typescript,
          parserOptions: {
            ...authoredParserOptions,
            ecmaVersion: 'latest',
            sourceType: 'module',
            ecmaFeatures: { jsx: true },
          },
        };
  return [
    {
      files: ['**/*.{jsx,tsx}'],
      languageOptions: parser,
      plugins: { stylistic: plugin },
      rules: {
        [`stylistic/${RULE}`]: ['error', ...options],
      },
    },
  ];
}

function verify(
  linter: {
    verify(code: string, config: unknown[], options: { filename: string }): LintMessage[];
  },
  parsers: ReplayParsers,
  plugin: unknown,
  testCase: Record<string, unknown>,
  parserKind: Exclude<ParserKind, 'authored-unexercised'>,
): LintMessage[] {
  const messages = linter.verify(
    testCase.code as string,
    ruleConfig(parsers, plugin, testCase, parserKind),
    { filename: filenameFor(parserKind) },
  );
  const parseError = messages.find((message) => message.ruleId === null);
  if (parseError) {
    throw new Error(`Published replay parse error with ${parserKind}: ${parseError.message}`);
  }
  return messages;
}

function messageData(message: string): {
  needed: number;
  type: string;
  characters: string;
  gotten: number;
} {
  const match = MESSAGE_PATTERN.exec(message);
  if (!match) {
    throw new Error(`Unexpected ${RULE} message: ${message}`);
  }
  return {
    needed: Number(match[1]),
    type: match[2],
    characters: match[3],
    gotten: Number(match[4]),
  };
}

function locationRange(source: string, message: LintMessage): [number, number] {
  if (message.endLine === undefined || message.endColumn === undefined) {
    throw new Error(`Published ${RULE} diagnostic is missing its end location.`);
  }
  return [
    offsetAt(source, message.line, message.column),
    offsetAt(source, message.endLine, message.endColumn),
  ];
}

function offsetAt(source: string, targetLine: number, targetColumn: number): number {
  let line = 1;
  let lineStart = 0;
  for (let index = 0; index < source.length && line < targetLine; index += 1) {
    const character = source[index];
    if (character === '\r') {
      if (source[index + 1] === '\n') {
        index += 1;
      }
      line += 1;
      lineStart = index + 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      line += 1;
      lineStart = index + 1;
    }
  }
  if (line !== targetLine) {
    throw new RangeError(`Line ${targetLine} is outside the source.`);
  }
  return lineStart + targetColumn - 1;
}

function applyFixes(
  source: string,
  diagnostics: Array<{ fix: { range: [number, number]; replacementText: string } | null }>,
): string {
  const fixes = diagnostics
    .map((diagnostic, index) => ({ index, fix: diagnostic.fix }))
    .filter(
      (
        value,
      ): value is {
        index: number;
        fix: { range: [number, number]; replacementText: string };
      } => value.fix !== null,
    )
    .sort(
      (left, right) =>
        left.fix.range[0] - right.fix.range[0] ||
        left.fix.range[1] - right.fix.range[1] ||
        left.index - right.index,
    );
  const accepted: typeof fixes = [];
  let lastEnd = -1;
  for (const value of fixes) {
    if (value.fix.range[0] <= lastEnd) {
      continue;
    }
    lastEnd = value.fix.range[1];
    accepted.push(value);
  }
  let output = source;
  for (const { fix } of accepted.reverse()) {
    output = `${output.slice(0, fix.range[0])}${fix.replacementText}${output.slice(fix.range[1])}`;
  }
  return output;
}

function sha256(value: string): string {
  return createHash('sha256').update(value).digest('hex');
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export function run(options) {',
    '  globalThis[key].push({',
    '    name: options.name,',
    '    valid: options.valid || [],',
    '    invalid: options.invalid || [],',
    '  });',
    '}',
  ].join('\n');
  const parsersStub = [
    'function authored(tests) { return tests.flat(Infinity).filter(Boolean); }',
    'export function valids(...tests) { return authored(tests); }',
    'export function invalids(...tests) { return authored(tests); }',
  ].join('\n');

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
