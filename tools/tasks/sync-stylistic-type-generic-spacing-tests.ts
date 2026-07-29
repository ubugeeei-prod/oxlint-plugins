// Captures the stable @stylistic/eslint-plugin type-generic-spacing RuleTester
// suite from the pinned submodule, then replays each case through the exact
// published rule to record diagnostic locations, ranges, and fixes.
//
// Re-run with `pnpm run port:tests:stylistic:type-generic-spacing`.

import { execFileSync } from 'node:child_process';
import { createRequire, registerHooks } from 'node:module';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
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
const UPSTREAM_REF = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const RULE = 'type-generic-spacing';
const ESLINT_VERSION = '10.4.1';
const TYPESCRIPT_ESLINT_PARSER_VERSION = '8.60.0';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const FIXTURES_DIR = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures');
const FIXTURE_FILE = join(FIXTURES_DIR, `${RULE}.json`);
const CAPTURE_KEY = '__stylisticTypeGenericSpacingSyncCapture__';

if (!existsSync(UPSTREAM_DIR)) {
  throw new Error(
    `Upstream checkout not found at ${UPSTREAM_DIR}. Run \`git submodule update --init upstream/eslint-stylistic\` first.`,
  );
}

const actualCommit = execFileSync('git', ['-C', UPSTREAM_DIR, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== UPSTREAM_COMMIT) {
  throw new Error(
    `Expected upstream/eslint-stylistic at ${UPSTREAM_COMMIT}, received ${actualCommit}.`,
  );
}

registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-type-generic-spacing-sync-'));
const captureFile = join(tempDir, `${RULE}.test.ts`);
const replayDir = join(tempDir, 'exact-replay');

try {
  const source = execFileSync(
    'git',
    ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${SOURCE_FILE}`],
    { encoding: 'utf8' },
  );
  writeFileSync(captureFile, source);

  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  await import(`${pathToFileURL(captureFile).href}?capture=${Date.now()}`);
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
      `@stylistic/eslint-plugin@${UPSTREAM_REF.slice(1)}`,
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
        `Published rule rejected captured valid case ${index}: ${messages[0].message}`,
      );
    }
  }

  const invalid = runs[0].invalid.map((raw, index) => {
    const testCase = normalizeCase(raw);
    const capturedErrors = Array.isArray(testCase.errors) ? testCase.errors : [];
    const messages = verify(linter, parser, plugin, testCase);
    if (messages.length !== capturedErrors.length) {
      throw new Error(
        `Published rule produced ${messages.length} diagnostics for invalid case ${index}; expected ${capturedErrors.length}.`,
      );
    }
    if (
      messages.some((message) => message.messageId !== 'genericSpacingMismatch' || !message.fix)
    ) {
      throw new Error(
        `Published rule returned an unexpected or unfixable diagnostic for case ${index}.`,
      );
    }

    const fixed = verifyAndFix(linter, parser, plugin, testCase);
    if (typeof testCase.output !== 'string' || fixed !== testCase.output) {
      throw new Error(`Published rule fix output diverged for invalid case ${index}.`);
    }

    return {
      ...testCase,
      errors: messages.map((message) => ({
        messageId: message.messageId,
        message: message.message,
        line: message.line,
        column: message.column,
        endLine: message.endLine,
        endColumn: message.endColumn,
        range: locationRange(testCase.code as string, message),
        fix: message.fix,
      })),
    };
  });

  const fixture = {
    __generated: {
      source: '@stylistic/eslint-plugin',
      version: UPSTREAM_REF,
      commit: UPSTREAM_COMMIT,
      sourceFile: SOURCE_FILE,
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-type-generic-spacing-tests.ts',
      exactReplay: {
        eslint: ESLINT_VERSION,
        typescriptEslintParser: TYPESCRIPT_ESLINT_PARSER_VERSION,
      },
    },
    valid,
    invalid,
  };

  mkdirSync(FIXTURES_DIR, { recursive: true });
  writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('vp', ['fmt', FIXTURE_FILE], { stdio: 'ignore' });
  console.log(
    `Synced ${RULE} from @stylistic/eslint-plugin ${UPSTREAM_REF}: ${valid.length} valid, ${invalid.length} invalid, ${invalid.flatMap((testCase) => testCase.errors).length} exact diagnostics.`,
  );
} finally {
  rmSync(tempDir, { recursive: true, force: true });
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
      files: ['**/*.ts'],
      languageOptions: {
        parser,
        parserOptions: {
          ecmaVersion: 'latest',
          sourceType: 'module',
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
    filename: 'fixture.ts',
  });
}

function verifyAndFix(
  linter: {
    verifyAndFix(
      code: string,
      config: unknown[],
      options: { filename: string },
    ): { output: string };
  },
  parser: unknown,
  plugin: unknown,
  testCase: Record<string, unknown>,
): string {
  return linter.verifyAndFix(testCase.code as string, ruleConfig(parser, plugin, testCase), {
    filename: 'fixture.ts',
  }).output;
}

function locationRange(
  source: string,
  message: Pick<LintMessage, 'line' | 'column' | 'endLine' | 'endColumn'>,
): [number, number] {
  if (message.endLine === undefined || message.endColumn === undefined) {
    throw new Error('Published rule diagnostic is missing its end location.');
  }
  return [
    offsetAt(source, message.line, message.column),
    offsetAt(source, message.endLine, message.endColumn),
  ];
}

function offsetAt(source: string, line: number, column: number): number {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line && offset < source.length) {
    const character = source[offset];
    if (character === '\r') {
      offset += source[offset + 1] === '\n' ? 2 : 1;
      currentLine += 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      offset += 1;
      currentLine += 1;
    } else {
      offset += 1;
    }
  }
  return offset + column - 1;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = '${CAPTURE_KEY}';`,
    'export function run(options) {',
    '  globalThis[key].push({',
    '    name: options.name,',
    '    valid: options.valid || [],',
    '    invalid: options.invalid || [],',
    '  });',
    '}',
    'const fullWhitespace = /^\\s*$/;',
    'export function $(value) {',
    '  const source = typeof value === "string" ? value : value[0];',
    '  const lines = source.split("\\n");',
    '  const whitespaceLines = lines.map((line) => fullWhitespace.test(line));',
    '  const commonIndent = lines.reduce((min, line, index) => {',
    '    if (whitespaceLines[index]) return min;',
    '    const indent = line.match(/^\\s*/)?.[0].length;',
    '    return indent === undefined ? min : Math.min(min, indent);',
    '  }, Number.POSITIVE_INFINITY);',
    '  let head = 0;',
    '  while (head < lines.length && whitespaceLines[head]) head += 1;',
    '  let tail = 0;',
    '  while (tail < lines.length && whitespaceLines[lines.length - tail - 1]) tail += 1;',
    '  return lines.slice(head, lines.length - tail)',
    '    .map((line) => line.slice(commonIndent))',
    '    .join("\\n");',
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
