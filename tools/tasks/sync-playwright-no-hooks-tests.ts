// Captures every authored eslint-plugin-playwright v2.11.0 RuleTester case for
// `no-hooks` from the exact upstream commit.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { runInNewContext } from 'node:vm';

type RawCase =
  | string
  | {
      code: string;
      errors?: RawError[];
      options?: unknown[];
      output?: string | null;
      settings?: unknown;
    };
type RawError = {
  column?: number;
  data?: Record<string, unknown>;
  endColumn?: number;
  endLine?: number;
  line?: number;
  messageId?: string;
};
type CapturedSuite = {
  invalid: RawCase[];
  valid: RawCase[];
};
type Manifest = {
  plugins: Array<{
    id: string;
    license: string;
    submodule: string;
  }>;
};

const ROOT = process.cwd();
const VERSION = '2.11.0';
const PINNED_COMMIT = 'b6d3e5dac73c8aad4d5e62a933105579c319655f';
const PREVIOUS_VERSION = '2.10.4';
const PREVIOUS_COMMIT = '894c0ec261763bb1e073b276c70bbf88b4ebad39';
const RULE = 'no-hooks';
const SOURCE_FILES = [`src/rules/${RULE}.ts`, `src/rules/${RULE}.test.ts`, `docs/rules/${RULE}.md`];

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-playwright');
if (!plugin) throw new Error('eslint-plugin-playwright is not registered');

const submodule = join(ROOT, plugin.submodule);
if (!existsSync(join(submodule, '.git'))) {
  throw new Error(`Run \`git submodule update --init ${plugin.submodule}\` first.`);
}
for (const commit of [PREVIOUS_COMMIT, PINNED_COMMIT]) {
  execFileSync('git', ['-C', submodule, 'cat-file', '-e', `${commit}^{commit}`]);
}

const sourceHashes = Object.fromEntries(
  SOURCE_FILES.map((sourceFile) => [
    sourceFile,
    createHash('sha256').update(upstreamSource(PINNED_COMMIT, sourceFile)).digest('hex'),
  ]),
);
const changedFromPrevious = SOURCE_FILES.filter(
  (sourceFile) =>
    upstreamSource(PREVIOUS_COMMIT, sourceFile) !== upstreamSource(PINNED_COMMIT, sourceFile),
);
if (changedFromPrevious.length !== 0) {
  throw new Error(
    `${RULE} unexpectedly drifted from ${PREVIOUS_VERSION}: ${changedFromPrevious.join(', ')}`,
  );
}

const suite = captureSuite(upstreamSource(PINNED_COMMIT, `src/rules/${RULE}.test.ts`));
const valid = suite.valid.map((testCase, index) =>
  normalizeCase(testCase, false, `valid ${index}`),
);
const invalid = suite.invalid.map((testCase, index) =>
  normalizeCase(testCase, true, `invalid ${index}`),
);
const inventory = {
  suites: 1,
  valid: valid.length,
  invalid: invalid.length,
  diagnostics: invalid.reduce((total, testCase) => total + testCase.expectedDiagnostics.length, 0),
  fixable: invalid.filter((testCase) => testCase.expectedOutput !== null).length,
};
const fixture = {
  __generated: {
    source: 'eslint-plugin-playwright',
    version: VERSION,
    sourceCommit: PINNED_COMMIT,
    license: plugin.license,
    tool: 'tools/tasks/sync-playwright-no-hooks-tests.ts',
    sourceFiles: SOURCE_FILES,
    sourceHashes,
    previousVersionAudit: {
      version: PREVIOUS_VERSION,
      sourceCommit: PREVIOUS_COMMIT,
      changedFiles: changedFromPrevious,
    },
    inventory,
  },
  rule: RULE,
  valid,
  invalid,
};

const outputDirectory = join(ROOT, 'npm', 'playwright', 'test', 'fixtures');
mkdirSync(outputDirectory, { recursive: true });
const outputPath = join(outputDirectory, `${RULE}-v${VERSION}.json`);
writeFileSync(outputPath, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', outputPath], { stdio: 'ignore' });
console.log(
  `Captured ${inventory.valid} valid, ${inventory.invalid} invalid, ` +
    `${inventory.diagnostics} diagnostics, and ${inventory.fixable} fixed outputs.`,
);

function captureSuite(source: string): CapturedSuite {
  const executable = source.replace(/^import .*$/gmu, '');
  const sandbox: {
    captured?: CapturedSuite;
    dedent: typeof dedent;
    rule: object;
    runRuleTester: (
      capturedRule: string,
      capturedRuleModule: unknown,
      suite: CapturedSuite,
    ) => void;
  } = {
    dedent,
    rule: {},
    runRuleTester(capturedRule, _capturedRuleModule, suite) {
      if (capturedRule !== RULE) {
        throw new Error(`Expected ${RULE}, captured ${capturedRule}.`);
      }
      sandbox.captured = suite;
    },
  };
  runInNewContext(`"use strict";\n${executable}`, sandbox);
  if (!sandbox.captured) throw new Error(`Failed to capture ${RULE}.`);
  return sandbox.captured;
}

function normalizeCase(testCase: RawCase, invalid: boolean, label: string) {
  const normalized = typeof testCase === 'string' ? { code: testCase } : testCase;
  if (typeof normalized.code !== 'string') throw new Error(`${label} is missing source code.`);
  const errors = invalid ? normalized.errors : [];
  if (invalid && (!Array.isArray(errors) || errors.length === 0)) {
    throw new Error(`${label} is missing expected errors.`);
  }
  return {
    code: normalized.code,
    options: normalized.options ?? [],
    settings: normalized.settings ?? null,
    expectedDiagnostics: (errors ?? []).map((error, index) =>
      normalizeError(error, `${label} error ${index}`),
    ),
    expectedOutput: invalid && typeof normalized.output === 'string' ? normalized.output : null,
  };
}

function normalizeError(error: RawError, label: string) {
  if (typeof error.messageId !== 'string') throw new Error(`${label} is missing messageId.`);
  const line = error.line ?? 1;
  return {
    messageId: error.messageId,
    data: error.data ?? {},
    ...(typeof error.column === 'number'
      ? {
          loc: {
            startLine: line,
            startColumn: error.column - 1,
            ...(typeof error.endColumn === 'number'
              ? {
                  endLine: error.endLine ?? line,
                  endColumn: error.endColumn - 1,
                }
              : {}),
          },
        }
      : {}),
  };
}

function upstreamSource(commit: string, sourceFile: string): string {
  return execFileSync('git', ['-C', submodule, 'show', `${commit}:${sourceFile}`], {
    encoding: 'utf8',
  });
}

function dedent(
  strings: TemplateStringsArray | string,
  ...values: Array<string | number | bigint | boolean | null | undefined>
): string {
  const value =
    typeof strings === 'string'
      ? strings
      : strings.reduce(
          (output, segment, index) => output + segment + String(values[index] ?? ''),
          '',
        );
  const lines = value
    .replace(/^\n/u, '')
    .replace(/\n\s*$/u, '')
    .split('\n');
  const indents = lines
    .filter((line) => line.trim().length > 0)
    .map((line) => line.match(/^\s*/u)?.[0].length ?? 0);
  const indent = indents.length > 0 ? Math.min(...indents) : 0;
  return lines.map((line) => line.slice(indent)).join('\n');
}
