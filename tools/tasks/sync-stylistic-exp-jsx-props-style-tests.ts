// Captures every authored @stylistic/exp-jsx-props-style v5.10.0
// RuleTester case once, then replays it through the exact published rule to
// record messages, data, UTF-16 ranges, first-pass fixes, and recursive output.

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
const RULE = 'exp-jsx-props-style';
const SUITE_NAME = 'jsx-props-style';
const SOURCE_DIRECTORY = 'jsx-props-style';
const UPSTREAM_VERSION = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '10.4.1';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${SOURCE_DIRECTORY}/${SOURCE_DIRECTORY}.test.ts`;
const RULE_FILE = `packages/eslint-plugin/rules/${SOURCE_DIRECTORY}/${SOURCE_DIRECTORY}.ts`;
const FIXTURE_FILE = join(
  ROOT,
  'npm',
  'stylistic',
  'test',
  'fixtures',
  `${RULE}-${UPSTREAM_VERSION}.json`,
);
const CAPTURE_KEY = '__stylisticExpJsxPropsStyleCapture__';
const MESSAGES = {
  shouldWrap: 'Prop `{{prop}}` must be placed on a new line',
  shouldNotWrap: 'Prop `{{prop}}` should not be placed on a new line',
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
for (const [messageId, template] of Object.entries(MESSAGES)) {
  if (!ruleSource.includes(`${messageId}: '${template}'`)) {
    throw new Error(`Pinned ${RULE_FILE} no longer contains exact ${messageId} metadata.`);
  }
}

registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-exp-jsx-props-style-sync-'));
const captureFile = join(tempDir, `${SOURCE_DIRECTORY}.test.ts`);
const replayDir = join(tempDir, 'exact-replay');

try {
  writeFileSync(captureFile, source);
  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  await import(`${pathToFileURL(captureFile).href}?commit=${UPSTREAM_COMMIT}`);
  const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
  if (runs.length !== 1 || runs[0]?.name !== SUITE_NAME) {
    throw new Error(`Expected one captured ${SUITE_NAME} suite, received ${runs.length}.`);
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
      ): { fixed: boolean; output: string; messages: LintMessage[] };
    };
  };
  const plugin = require('@stylistic/eslint-plugin') as unknown;
  const linter = new Linter();

  const valid = runs[0].valid.map(normalizeCase);
  for (const [index, testCase] of valid.entries()) {
    const messages = verify(linter, plugin, testCase);
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
    const messages = verify(linter, plugin, testCase);
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
      return {
        messageId: captured.messageId,
        message: message.message,
        data: captured.data,
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

    const firstPassOutput = applyFixes(testCase.code as string, messages);
    const authoredOutput = typeof testCase.output === 'string' ? testCase.output : null;
    if (firstPassOutput !== authoredOutput) {
      throw new Error(
        `Published rule first-pass output diverged for invalid case ${index}: ${JSON.stringify(firstPassOutput)} !== ${JSON.stringify(authoredOutput)}.`,
      );
    }
    const recursive = linter.verifyAndFix(testCase.code as string, ruleConfig(plugin, testCase), {
      filename: 'fixture.jsx',
    });

    return {
      ...testCase,
      output: firstPassOutput,
      recursiveOutput: recursive.output,
      recursiveDiagnostics: recursive.messages.map((message) => ({
        messageId: message.messageId,
        message: message.message,
        line: message.line,
        column: message.column,
        endLine: message.endLine,
        endColumn: message.endColumn,
        range: locationRange(recursive.output, message),
      })),
      diagnostics,
    };
  });

  const diagnosticCount = invalid.reduce(
    (count, testCase) => count + testCase.diagnostics.length,
    0,
  );
  const fixableDiagnostics = invalid.reduce(
    (count, testCase) =>
      count + testCase.diagnostics.filter((diagnostic) => diagnostic.fix !== null).length,
    0,
  );
  const fixableInvalid = invalid.filter((testCase) =>
    testCase.diagnostics.some((diagnostic) => diagnostic.fix !== null),
  ).length;
  const fixture = {
    __generated: {
      source: '@stylistic/eslint-plugin',
      rule: RULE,
      version: UPSTREAM_VERSION,
      commit: UPSTREAM_COMMIT,
      sourceFile: SOURCE_FILE,
      ruleFile: RULE_FILE,
      sourceSha256: sha256(source),
      ruleSourceSha256: sha256(ruleSource),
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-exp-jsx-props-style-tests.ts',
      capturePolicy:
        'Each authored semantic case is captured once; exact replay uses the published rule with ESLint Espree in JSX mode.',
      exactReplay: {
        eslint: ESLINT_VERSION,
        parser: 'espree bundled with ESLint',
      },
      inventory: {
        valid: valid.length,
        invalid: invalid.length,
        diagnostics: diagnosticCount,
        fixableDiagnostics,
        unfixableDiagnostics: diagnosticCount - fixableDiagnostics,
        fixableInvalid,
        unfixableInvalid: invalid.length - fixableInvalid,
        total: valid.length + invalid.length,
      },
    },
    valid,
    invalid,
  };

  if (
    valid.length !== 17 ||
    invalid.length !== 11 ||
    diagnosticCount !== 17 ||
    fixableDiagnostics !== 15
  ) {
    throw new Error(
      `Unexpected authored inventory: ${valid.length} valid, ${invalid.length} invalid, ${diagnosticCount} diagnostics, ${fixableDiagnostics} fixable diagnostics.`,
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

function ruleConfig(plugin: unknown, testCase: Record<string, unknown>): unknown[] {
  const options = Array.isArray(testCase.options) ? testCase.options : [];
  return [
    {
      files: ['**/*.jsx'],
      languageOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        parserOptions: {
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
  plugin: unknown,
  testCase: Record<string, unknown>,
): LintMessage[] {
  return linter.verify(testCase.code as string, ruleConfig(plugin, testCase), {
    filename: 'fixture.jsx',
  });
}

function applyFixes(source: string, messages: LintMessage[]): string | null {
  const fixes = messages
    .flatMap((message) => (message.fix ? [message.fix] : []))
    .sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);
  if (fixes.length === 0) {
    return null;
  }
  let output = source;
  for (const fix of fixes) {
    output = output.slice(0, fix.range[0]) + fix.text + output.slice(fix.range[1]);
  }
  return output;
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
    'export function $(strings, ...values) {',
    '  let value = strings[0];',
    '  for (let index = 0; index < values.length; index += 1)',
    '    value += String(values[index]) + strings[index + 1];',
    "  const lines = value.replace(/^\\r?\\n/, '').replace(/\\r?\\n\\s*$/, '').split(/\\r?\\n/);",
    '  const indents = lines.filter(line => line.trim()).map(line => line.match(/^\\s*/)[0].length);',
    '  const indent = indents.length ? Math.min(...indents) : 0;',
    "  return lines.map(line => line.slice(indent)).join('\\n');",
    '}',
  ].join('\n');

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///stylistic-test', shortCircuit: true };
      }
      if (
        specifier === `./${SOURCE_DIRECTORY}` ||
        specifier === './types' ||
        specifier === './types.d.ts'
      ) {
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
