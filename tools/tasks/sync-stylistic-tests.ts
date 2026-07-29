// Captures the stable @stylistic/eslint-plugin fixtures directly from the
// pinned upstream submodule. The committed JSON is the executable compatibility
// contract for the native port: case order, source, options, parser options,
// messages, locations, and fixed output all come from the v5.10.0 test file.
//
// Re-run with `pnpm run port:tests:stylistic`. The synchronizer intentionally
// fails when the submodule is not at the audited commit, so a fixture refresh
// cannot silently mix another upstream release into the stable port.

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
const RULE = 'array-element-newline';
const PINNED_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const CAPTURE_KEY = '__stylisticSyncCapture__';
const MESSAGES = {
  unexpectedLineBreak: 'There should be no linebreak here.',
  missingLineBreak: 'There should be a linebreak after this element.',
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
          'export function run(config) {',
          `  globalThis['${CAPTURE_KEY}'] = config;`,
          '}',
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

const valid = captured.valid.map((testCase, index) => normalizeCase(testCase, false, index));
const invalid = captured.invalid.map((testCase, index) => normalizeCase(testCase, true, index));
const fixture = {
  __generated: {
    source: plugin.npm,
    version: plugin.baselineVersion,
    sourceCommit: PINNED_COMMIT,
    sourceFile: `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`,
    license: plugin.license,
    tool: 'tools/tasks/sync-stylistic-tests.ts',
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
    `${valid.length} valid, ${invalid.length} invalid.`,
);

function normalizeCase(raw: RawCase, isInvalid: boolean, index: number): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`Unsupported ${isInvalid ? 'invalid' : 'valid'} case at index ${index}`);
  }

  const normalized: Record<string, unknown> = { code: value.code };
  if ('options' in value) {
    normalized.options = clone(value.options);
  }
  if ('parserOptions' in value) {
    normalized.parserOptions = clone(value.parserOptions);
  }

  if (isInvalid) {
    if (!('output' in value) || (typeof value.output !== 'string' && value.output !== null)) {
      throw new Error(`Invalid case ${index} is missing its fixed output`);
    }
    if (!Array.isArray(value.errors)) {
      throw new Error(`Invalid case ${index} is missing its ordered errors`);
    }
    normalized.output = value.output;
    normalized.errors = value.errors.map((error, errorIndex) =>
      normalizeError(error, index, errorIndex),
    );
  }

  return normalized;
}

function normalizeError(error: unknown, caseIndex: number, errorIndex: number) {
  if (!error || typeof error !== 'object') {
    throw new Error(`Unsupported error ${errorIndex} in invalid case ${caseIndex}`);
  }
  const record = clone(error) as Record<string, unknown>;
  const messageId = record.messageId;
  if (typeof messageId !== 'string' || !(messageId in MESSAGES)) {
    throw new Error(
      `Unknown messageId ${String(messageId)} in invalid case ${caseIndex}, error ${errorIndex}`,
    );
  }
  return {
    ...record,
    message: MESSAGES[messageId as keyof typeof MESSAGES],
  };
}

function clone(value: unknown): unknown {
  return JSON.parse(JSON.stringify(value));
}
