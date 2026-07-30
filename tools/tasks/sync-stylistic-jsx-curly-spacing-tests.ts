// Captures every authored stable @stylistic/jsx-curly-spacing v5.10.0
// RuleTester case from the exact pinned upstream commit. The parser helper
// repeats these semantic cases across compatible JSX parsers; native replay
// exercises the same sources with Oxc's JSX and TSX modes.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = { name: string; valid: RawCase[]; invalid: RawCase[] };

const ROOT = process.cwd();
const RULE = 'jsx-curly-spacing';
const VERSION = 'v5.10.0';
const COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const UPSTREAM = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const RULE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.ts`;
const FIXTURE = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures', `${RULE}-${VERSION}.json`);
const CAPTURE_KEY = '__stylisticJsxCurlySpacingCapture__';
const MESSAGES = {
  noNewlineAfter: "There should be no newline after '{{token}}'",
  noNewlineBefore: "There should be no newline before '{{token}}'",
  noSpaceAfter: "There should be no space after '{{token}}'",
  noSpaceBefore: "There should be no space before '{{token}}'",
  spaceNeededAfter: "A space is required after '{{token}}'",
  spaceNeededBefore: "A space is required before '{{token}}'",
} as const;

if (!existsSync(UPSTREAM)) {
  throw new Error(`Missing ${UPSTREAM}; initialize upstream/eslint-stylistic.`);
}
const actualCommit = execFileSync('git', ['-C', UPSTREAM, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== COMMIT) {
  throw new Error(`Expected eslint-stylistic ${COMMIT}, received ${actualCommit}.`);
}
const ruleSource = upstreamFile(RULE_FILE);
for (const [messageId, template] of Object.entries(MESSAGES)) {
  if (!ruleSource.includes(`${messageId}: '${template.replaceAll("'", "\\'")}'`)) {
    throw new Error(`Pinned ${RULE_FILE} no longer contains exact ${messageId} metadata.`);
  }
}

registerCaptureHooks();
const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-curly-spacing-sync-'));
const sourcePath = join(temp, `${RULE}.test.ts`);
writeFileSync(sourcePath, upstreamFile(SOURCE_FILE));
(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
try {
  await import(`${pathToFileURL(sourcePath).href}?commit=${COMMIT}`);
} finally {
  rmSync(temp, { recursive: true, force: true });
}

const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
if (runs.length !== 1 || runs[0].name !== RULE) {
  throw new Error(`Expected one ${RULE} suite, received ${runs.length}.`);
}
const valid = runs[0].valid.map((value, index) => normalizeCase(value, false, index));
const invalid = runs[0].invalid.map((value, index) => normalizeCase(value, true, index));
const diagnostics = invalid.reduce(
  (count, testCase) => count + (testCase.errors as unknown[]).length,
  0,
);
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: VERSION,
    commit: COMMIT,
    sourceFile: SOURCE_FILE,
    ruleFile: RULE_FILE,
    license: 'MIT',
    parserMatrix: ['default', '@babel/eslint-parser', '@typescript-eslint/parser'],
    parserExpansion: 'authored semantic cases replayed with Oxc JSX/TSX',
    tool: 'tools/tasks/sync-stylistic-jsx-curly-spacing-tests.ts',
    inventory: {
      valid: valid.length,
      invalid: invalid.length,
      diagnostics,
      fixableInvalid: invalid.filter((testCase) => typeof testCase.output === 'string').length,
      unfixableInvalid: invalid.filter((testCase) => testCase.output === null).length,
      total: valid.length + invalid.length,
    },
  },
  valid,
  invalid,
};
mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
writeFileSync(FIXTURE, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', FIXTURE], { stdio: 'inherit' });
console.log(
  `Synced ${RULE} ${VERSION}: ${valid.length} valid, ${invalid.length} invalid, ${diagnostics} diagnostics.`,
);

function upstreamFile(path: string): string {
  return execFileSync('git', ['-C', UPSTREAM, 'show', `${COMMIT}:${path}`], {
    encoding: 'utf8',
  });
}

function normalizeCase(raw: RawCase, invalid: boolean, index: number): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`${invalid ? 'invalid' : 'valid'} case ${index} has no code.`);
  }
  const allowed = new Set(
    invalid
      ? ['code', 'output', 'errors', 'options', 'parserOptions', 'settings', 'languageOptions']
      : ['code', 'options', 'parserOptions', 'settings', 'languageOptions'],
  );
  const unsupported = Object.keys(value).filter((key) => key !== 'features' && !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported case ${index} keys: ${unsupported.join(', ')}`);
  }
  if (
    invalid &&
    ((typeof value.output !== 'string' && value.output !== null) || !Array.isArray(value.errors))
  ) {
    throw new Error(`Invalid case ${index} is missing output/errors.`);
  }
  const normalized = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
  delete normalized.features;
  if (invalid) {
    normalized.errors = (normalized.errors as Array<Record<string, unknown>>).map(
      (error, errorIndex) => {
        const messageId = error.messageId as keyof typeof MESSAGES;
        const data = error.data as { token?: unknown } | undefined;
        if (!MESSAGES[messageId] || (data?.token !== '{' && data?.token !== '}')) {
          throw new Error(`Invalid case ${index} error ${errorIndex} has an unsupported contract.`);
        }
        return {
          ...error,
          message: MESSAGES[messageId].replace('{{token}}', data.token),
        };
      },
    );
  }
  return normalized;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export function run(options) {',
    '  globalThis[key].push({ name: options.name, valid: options.valid || [], invalid: options.invalid || [] });',
    '}',
  ].join('\n');
  const parserStub = [
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
        return { format: 'module', source: parserStub, shortCircuit: true };
      }
      if (url === 'stub:///stylistic-rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}
