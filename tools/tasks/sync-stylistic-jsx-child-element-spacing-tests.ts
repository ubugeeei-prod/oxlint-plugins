// Captures every stable @stylistic/jsx-child-element-spacing v5.10.0
// RuleTester case, including the exact JSX parser expansion performed by the
// pinned upstream helper.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawError = {
  messageId: keyof typeof MESSAGES;
  data: { element: string };
  line: number;
  column: number;
};
type RawCase = {
  code: string;
  features?: string[];
  errors?: RawError[];
};
type ExpandedCase = {
  code: string;
  parser: string;
  errors?: RawError[];
};
type CapturedRun = {
  name: string;
  valid: ExpandedCase[];
  invalid: ExpandedCase[];
};

const ROOT = process.cwd();
const RULE = 'jsx-child-element-spacing';
const UPSTREAM_VERSION = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const RULE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.ts`;
const FIXTURE_FILE = join(
  ROOT,
  'npm',
  'stylistic',
  'test',
  'fixtures',
  `${RULE}-${UPSTREAM_VERSION}.json`,
);
const CAPTURE_KEY = '__stylisticJsxChildElementSpacingCapture__';
const MESSAGES = {
  spacingAfterPrev: 'Ambiguous spacing after previous element {{element}}',
  spacingBeforeNext: 'Ambiguous spacing before next element {{element}}',
} as const;
const PARSERS = ['default', '@babel/eslint-parser', '@typescript-eslint/parser'] as const;

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

const ruleSource = upstreamFile(RULE_FILE);
for (const [messageId, template] of Object.entries(MESSAGES)) {
  if (!ruleSource.includes(`${messageId}: '${template}'`)) {
    throw new Error(`Pinned ${RULE_FILE} no longer contains exact ${messageId} metadata.`);
  }
}

registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-child-spacing-sync-'));
const tempFile = join(tempDir, `${RULE}.test.ts`);
writeFileSync(tempFile, upstreamFile(SOURCE_FILE));

(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
try {
  await import(`${pathToFileURL(tempFile).href}?commit=${UPSTREAM_COMMIT}`);
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
if (runs.length !== 1 || runs[0].name !== RULE) {
  throw new Error(`Expected one captured ${RULE} suite, received ${runs.length}.`);
}

const valid = runs[0].valid.map((testCase, index) => normalizeValid(testCase, index));
const invalid = runs[0].invalid.map((testCase, index) => normalizeInvalid(testCase, index));
const diagnostics = invalid.reduce(
  (count, testCase) => count + testCase.expectedDiagnostics.length,
  0,
);
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: UPSTREAM_VERSION,
    commit: UPSTREAM_COMMIT,
    sourceFile: SOURCE_FILE,
    ruleFile: RULE_FILE,
    license: 'MIT',
    tool: 'tools/tasks/sync-stylistic-jsx-child-element-spacing-tests.ts',
    parserMatrix: [...PARSERS],
    inventory: {
      logicalValid: 21,
      logicalInvalid: 7,
      valid: valid.length,
      invalid: invalid.length,
      diagnostics,
      fixableInvalid: 0,
      unfixableInvalid: invalid.length,
      total: valid.length + invalid.length,
    },
  },
  valid,
  invalid,
};

if (valid.length !== 62 || invalid.length !== 20 || diagnostics !== 23) {
  throw new Error(
    `Unexpected expanded inventory: ${valid.length} valid, ${invalid.length} invalid, ${diagnostics} diagnostics.`,
  );
}

mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', FIXTURE_FILE], { stdio: 'inherit' });
console.log(
  `Synced ${RULE} from @stylistic/eslint-plugin ${UPSTREAM_VERSION}: ` +
    `${valid.length} valid, ${invalid.length} invalid, ${diagnostics} diagnostics.`,
);

function upstreamFile(path: string): string {
  return execFileSync('git', ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${path}`], {
    encoding: 'utf8',
  });
}

function normalizeValid(testCase: ExpandedCase, index: number) {
  assertExpandedCase(testCase, `valid ${index}`);
  return {
    code: testCase.code,
    parser: testCase.parser,
  };
}

function normalizeInvalid(testCase: ExpandedCase, index: number) {
  assertExpandedCase(testCase, `invalid ${index}`);
  if (!Array.isArray(testCase.errors)) {
    throw new Error(`Captured ${RULE} invalid ${index} is missing errors.`);
  }

  return {
    code: testCase.code,
    parser: testCase.parser,
    expectedDiagnostics: testCase.errors.map((error, errorIndex) => {
      if (
        !MESSAGES[error.messageId] ||
        typeof error.data?.element !== 'string' ||
        !Number.isInteger(error.line) ||
        !Number.isInteger(error.column)
      ) {
        throw new Error(`Malformed error ${errorIndex} in captured ${RULE} invalid ${index}.`);
      }
      const offset = utf8OffsetAt(testCase.code, error.line, error.column);
      return {
        messageId: error.messageId,
        message: MESSAGES[error.messageId].replace('{{element}}', error.data.element),
        data: error.data,
        range: { start: offset, end: offset },
        location: {
          line: error.line,
          column: error.column,
        },
        fix: null,
      };
    }),
    output: null,
  };
}

function assertExpandedCase(testCase: ExpandedCase, label: string): void {
  if (
    !testCase ||
    typeof testCase.code !== 'string' ||
    !PARSERS.includes(testCase.parser as never)
  ) {
    throw new TypeError(`Captured ${RULE} ${label} is malformed.`);
  }
}

function utf8OffsetAt(source: string, line: number, column: number): number {
  const lines = source.split('\n');
  if (line < 1 || line > lines.length || column < 1 || column > lines[line - 1].length + 1) {
    throw new RangeError(`Invalid location ${line}:${column}.`);
  }
  const utf16Offset =
    lines.slice(0, line - 1).reduce((offset, value) => offset + value.length + 1, 0) + column - 1;
  return Buffer.byteLength(source.slice(0, utf16Offset));
}

function expandCases(...rawCases: RawCase[]): ExpandedCase[] {
  return rawCases.flatMap((rawCase) => {
    const features = new Set(rawCase.features ?? []);
    const parsers = features.has('fragment') ? PARSERS.slice(1) : PARSERS;
    return parsers.map((parser) => {
      const extras = [`features: [${[...features].join(',')}]`, `parser: ${parser}`, '', '', ''];
      return {
        code: `${rawCase.code}\n// ${extras.join(', ')}`,
        parser,
        ...(rawCase.errors ? { errors: JSON.parse(JSON.stringify(rawCase.errors)) } : {}),
      };
    });
  });
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
    `const expandCases = ${expandCases.toString()};`,
    `const PARSERS = ${JSON.stringify(PARSERS)};`,
    'export function valids(...tests) { return expandCases(...tests.flat().filter(Boolean)); }',
    'export function invalids(...tests) { return expandCases(...tests.flat().filter(Boolean)); }',
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
