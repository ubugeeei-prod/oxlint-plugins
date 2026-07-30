// Captures every authored eslint-plugin-playwright v2.10.4 RuleTester case for
// the three no-restricted-* rules from the exact vendored commit.

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
    baselineVersion: string;
    id: string;
    license: string;
    pinnedRef?: string;
    submodule: string;
  }>;
};

const ROOT = process.cwd();
const VERSION = '2.10.4';
const PINNED_COMMIT = '894c0ec261763bb1e073b276c70bbf88b4ebad39';
const RULES = ['no-restricted-locators', 'no-restricted-matchers', 'no-restricted-roles'] as const;

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-playwright');
if (!plugin) {
  throw new Error('eslint-plugin-playwright is not registered in tools/port-targets.json');
}
if (plugin.baselineVersion !== VERSION || plugin.pinnedRef !== `v${VERSION}`) {
  throw new Error(
    `Expected eslint-plugin-playwright v${VERSION}, received ` +
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
  throw new Error(`Expected ${plugin.submodule} at ${PINNED_COMMIT}, received ${actualCommit}.`);
}

const sourceFiles = RULES.flatMap((rule) => [`src/rules/${rule}.ts`, `src/rules/${rule}.test.ts`]);
const sourceHashes = Object.fromEntries(
  sourceFiles.map((sourceFile) => {
    const source = upstreamSource(sourceFile);
    return [sourceFile, createHash('sha256').update(source).digest('hex')];
  }),
);
const suites = RULES.map((rule) => {
  const capture = captureSuite(rule, upstreamSource(`src/rules/${rule}.test.ts`));
  return {
    rule,
    valid: capture.valid.map((testCase, index) =>
      normalizeCase(testCase, false, `${rule} valid ${index}`),
    ),
    invalid: capture.invalid.map((testCase, index) =>
      normalizeCase(testCase, true, `${rule} invalid ${index}`),
    ),
  };
});
const inventory = suites.map((suite) => ({
  rule: suite.rule,
  valid: suite.valid.length,
  invalid: suite.invalid.length,
  diagnostics: suite.invalid.reduce(
    (total, testCase) => total + testCase.expectedDiagnostics.length,
    0,
  ),
}));
const fixture = {
  __generated: {
    source: 'eslint-plugin-playwright',
    version: VERSION,
    sourceCommit: PINNED_COMMIT,
    license: plugin.license,
    tool: 'tools/tasks/sync-playwright-restricted-tests.ts',
    sourceFiles,
    sourceHashes,
    inventory: {
      suites: inventory,
      valid: inventory.reduce((total, entry) => total + entry.valid, 0),
      invalid: inventory.reduce((total, entry) => total + entry.invalid, 0),
      diagnostics: inventory.reduce((total, entry) => total + entry.diagnostics, 0),
    },
  },
  suites,
};

const outputDirectory = join(ROOT, 'npm', 'playwright', 'test', 'fixtures');
mkdirSync(outputDirectory, { recursive: true });
const outputPath = join(outputDirectory, `restricted-v${VERSION}.json`);
writeFileSync(outputPath, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', outputPath], { stdio: 'ignore' });
console.log(
  `Captured ${fixture.__generated.inventory.valid} valid, ` +
    `${fixture.__generated.inventory.invalid} invalid, and ` +
    `${fixture.__generated.inventory.diagnostics} exact diagnostics in ${outputPath}.`,
);

function captureSuite(rule: string, source: string): CapturedSuite {
  const test = (input: string): string => `test('test', async () => { ${input} })`;
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
    test: typeof test;
  } = {
    dedent,
    rule: {},
    runRuleTester(capturedRule, _capturedRuleModule, suite) {
      if (capturedRule !== rule) {
        throw new Error(`Expected ${rule}, captured ${capturedRule}.`);
      }
      sandbox.captured = suite;
    },
    test,
  };
  runInNewContext(`"use strict";\n${executable}`, sandbox);
  if (!sandbox.captured) {
    throw new Error(`Failed to capture ${rule}.`);
  }
  return sandbox.captured;
}

function normalizeCase(testCase: RawCase, invalid: boolean, label: string) {
  const normalized = typeof testCase === 'string' ? { code: testCase } : testCase;
  if (typeof normalized.code !== 'string') {
    throw new Error(`${label} is missing source code.`);
  }
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
  };
}

function normalizeError(error: RawError, label: string) {
  if (
    typeof error.messageId !== 'string' ||
    typeof error.column !== 'number' ||
    typeof error.endColumn !== 'number'
  ) {
    throw new Error(`${label} is missing its exact diagnostic contract.`);
  }
  const line = error.line ?? 1;
  return {
    messageId: error.messageId,
    data: error.data ?? {},
    loc: {
      startLine: line,
      startColumn: error.column - 1,
      endLine: error.endLine ?? line,
      endColumn: error.endColumn - 1,
    },
  };
}

function upstreamSource(sourceFile: string): string {
  return execFileSync('git', ['-C', submodule, 'show', `${PINNED_COMMIT}:${sourceFile}`], {
    encoding: 'utf8',
  });
}

function dedent(strings: TemplateStringsArray | string, ...values: unknown[]): string {
  const value =
    typeof strings === 'string' ? strings : String.raw({ raw: [...strings.raw] }, ...values);
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
