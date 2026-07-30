// Captures every authored @stylistic/jsx-props-no-multi-spaces v5.10.0
// RuleTester case once, then replays it through the exact published rule to
// record messages, ranges, fixes, and outputs.

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
type CapturedError = {
  messageId: keyof typeof MESSAGES;
  data: Record<string, string>;
};
type LintMessage = {
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

const ROOT = process.cwd();
const RULE = 'jsx-props-no-multi-spaces';
const UPSTREAM_VERSION = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '10.4.1';
const TYPESCRIPT_ESLINT_PARSER_VERSION = '8.60.0';
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
const CAPTURE_KEY = '__stylisticJsxPropsNoMultiSpacesCapture__';
const MESSAGES = {
  noLineGap: 'Expected no line gap between “{{prop1}}” and “{{prop2}}”',
  onlyOneSpace: 'Expected only one space between “{{prop1}}” and “{{prop2}}”',
} as const;

if (!existsSync(UPSTREAM_DIR)) {
  throw new Error(
    `Upstream checkout not found at ${UPSTREAM_DIR}. Run \`git submodule update --init upstream/eslint-stylistic\` first.`,
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
for (const [messageId, template] of Object.entries(MESSAGES)) {
  if (!ruleSource.includes(`${messageId}: '${template}'`)) {
    throw new Error(`Pinned ${RULE_FILE} no longer contains exact ${messageId} metadata.`);
  }
}

registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-props-no-multi-spaces-sync-'));
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
      `@stylistic/eslint-plugin@${UPSTREAM_VERSION.slice(1)}`,
      `eslint@${ESLINT_VERSION}`,
      `@typescript-eslint/parser@${TYPESCRIPT_ESLINT_PARSER_VERSION}`,
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
      ): { output: string };
    };
  };
  const parser = require('@typescript-eslint/parser') as unknown;
  const plugin = require('@stylistic/eslint-plugin') as unknown;
  const linter = new Linter();

  const valid = runs[0].valid.map(normalizeCase);
  for (const [index, testCase] of valid.entries()) {
    const messages = verify(linter, parser, plugin, testCase);
    if (messages.length !== 0) {
      throw new Error(
        `Published rule rejected authored valid case ${index}: ${messages[0].message}`,
      );
    }
  }

  const invalid = runs[0].invalid.map((raw, index) => {
    const testCase = normalizeCase(raw);
    const capturedErrors = Array.isArray(testCase.errors)
      ? (testCase.errors as CapturedError[])
      : [];
    const messages = verify(linter, parser, plugin, testCase);
    if (messages.length !== capturedErrors.length) {
      throw new Error(
        `Published rule produced ${messages.length} diagnostics for invalid case ${index}; expected ${capturedErrors.length}.`,
      );
    }

    const diagnostics = messages.map((message, messageIndex) => {
      const captured = capturedErrors[messageIndex];
      if (
        message.messageId !== captured.messageId ||
        !Object.hasOwn(MESSAGES, captured.messageId)
      ) {
        throw new Error(`Unexpected message ${messageIndex} in invalid case ${index}.`);
      }
      const range = locationRange(testCase.code as string, message);
      return {
        messageId: captured.messageId,
        message: message.message,
        data: captured.data,
        line: message.line,
        column: message.column,
        endLine: message.endLine,
        endColumn: message.endColumn,
        range,
        fix: message.fix
          ? {
              range: message.fix.range,
              replacementText: message.fix.text,
            }
          : null,
      };
    });

    const expectedOutput = typeof testCase.output === 'string' ? testCase.output : null;
    const actualOutput = linter.verifyAndFix(
      testCase.code as string,
      ruleConfig(parser, plugin, testCase),
      { filename: 'fixture.tsx' },
    ).output;
    if (
      (expectedOutput !== null && actualOutput !== expectedOutput) ||
      (expectedOutput === null && actualOutput !== testCase.code)
    ) {
      throw new Error(`Published rule fix output diverged for invalid case ${index}.`);
    }

    return {
      ...testCase,
      output: expectedOutput,
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
      tool: 'tools/tasks/sync-stylistic-jsx-props-no-multi-spaces-tests.ts',
      capturePolicy:
        'Each authored semantic case is captured once; exact replay uses @typescript-eslint/parser in TSX mode.',
      exactReplay: {
        eslint: ESLINT_VERSION,
        typescriptEslintParser: TYPESCRIPT_ESLINT_PARSER_VERSION,
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

  if (valid.length !== 16 || invalid.length !== 12 || diagnosticCount !== 17) {
    throw new Error(
      `Unexpected authored inventory: ${valid.length} valid, ${invalid.length} invalid, ${diagnosticCount} diagnostics.`,
    );
  }

  mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
  writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('vp', ['fmt', FIXTURE_FILE], { stdio: 'inherit' });
  console.log(
    `Synced ${RULE} from @stylistic/eslint-plugin ${UPSTREAM_VERSION}: ` +
      `${valid.length} valid, ${invalid.length} invalid, ${diagnosticCount} diagnostics.`,
  );
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}

function upstreamFile(path: string): string {
  return execFileSync('git', ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${path}`], {
    encoding: 'utf8',
  });
}

function normalizeCase(raw: RawCase): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  const clone = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
  if (typeof clone.code !== 'string') {
    throw new TypeError(`Captured ${RULE} case is missing string code.`);
  }
  return clone;
}

function ruleConfig(
  parser: unknown,
  plugin: unknown,
  testCase: Record<string, unknown>,
): unknown[] {
  const options = Array.isArray(testCase.options) ? testCase.options : [];
  return [
    {
      files: ['**/*.tsx'],
      languageOptions: {
        parser,
        parserOptions: {
          ecmaVersion: 'latest',
          sourceType: 'module',
          ecmaFeatures: { jsx: true },
        },
      },
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
  parser: unknown,
  plugin: unknown,
  testCase: Record<string, unknown>,
): LintMessage[] {
  return linter.verify(testCase.code as string, ruleConfig(parser, plugin, testCase), {
    filename: 'fixture.tsx',
  });
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
    'export function valids(...tests) { return tests.flat().filter(Boolean); }',
    'export function invalids(...tests) { return tests.flat().filter(Boolean); }',
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
