// Captures the stable @stylistic/eslint-plugin RuleTester suite from the
// vendored v5.10.0 submodule, then asks the published v5.10.0 rule for exact
// diagnostic locations, messages, fixes, and iterative fixed output. Both the
// Rust and JavaScript suites replay the resulting committed fixture.
//
// Re-run with `pnpm run port:tests:stylistic:multiline-ternary`.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = {
  name: string;
  valid: RawCase[];
  invalid: RawCase[];
};
type NormalizedCase = Record<string, unknown> & {
  code: string;
  options: unknown[];
};
type EnrichedFixture = {
  valid: NormalizedCase[];
  invalid: NormalizedCase[];
};

const ROOT = process.cwd();
const UPSTREAM_VERSION = '5.10.0';
const UPSTREAM_REF = `v${UPSTREAM_VERSION}`;
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '10.4.1';
const TYPESCRIPT_ESLINT_VERSION = '8.60.0';
const RULE = 'multiline-ternary';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const FIXTURES_DIR = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures');
const FIXTURE_FILE = join(FIXTURES_DIR, `${RULE}-v${UPSTREAM_VERSION}.json`);
const CAPTURE_KEY = '__stylisticMultilineTernaryCapture__';

if (!existsSync(join(UPSTREAM_DIR, '.git'))) {
  throw new Error(
    `Upstream checkout not found at ${UPSTREAM_DIR}. Run \`git submodule update --init upstream/eslint-stylistic\` first.`,
  );
}
const actualUpstreamCommit = execFileSync('git', ['-C', UPSTREAM_DIR, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualUpstreamCommit !== UPSTREAM_COMMIT) {
  throw new Error(
    `Expected eslint-stylistic ${UPSTREAM_COMMIT}, received ${actualUpstreamCommit}. Run \`git submodule update upstream/eslint-stylistic\`.`,
  );
}

registerCaptureHooks();
const captureDir = mkdtempSync(join(tmpdir(), 'stylistic-multiline-ternary-capture-'));
const captureFile = join(captureDir, `${RULE}.test.ts`);
const source = execFileSync(
  'git',
  ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${SOURCE_FILE}`],
  { encoding: 'utf8' },
);
writeFileSync(captureFile, source);

(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
await import(`${pathToFileURL(captureFile).href}?capture=${Date.now()}`);
const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
rmSync(captureDir, { recursive: true, force: true });

if (runs.length !== 1 || runs[0]?.name !== RULE) {
  throw new Error(`Expected one captured ${RULE} suite, received ${runs.length}.`);
}

const captured = {
  valid: runs[0].valid.map(normalizeCase),
  invalid: runs[0].invalid.map(normalizeCase),
};
const enriched = enrichWithPublishedRule(captured);
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: UPSTREAM_REF,
    commit: UPSTREAM_COMMIT,
    sourceFile: SOURCE_FILE,
    license: 'MIT',
    eslintVersion: ESLINT_VERSION,
    typescriptEslintParserVersion: TYPESCRIPT_ESLINT_VERSION,
    tool: 'tools/tasks/sync-stylistic-multiline-ternary-tests.ts',
  },
  valid: enriched.valid,
  invalid: enriched.invalid,
};

mkdirSync(FIXTURES_DIR, { recursive: true });
writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('pnpm', ['exec', 'vp', 'fmt', FIXTURE_FILE], {
  cwd: ROOT,
  stdio: 'inherit',
});
const diagnostics = enriched.invalid.reduce(
  (total, testCase) => total + (testCase.errors as unknown[]).length,
  0,
);
console.log(
  `Synced ${RULE} from @stylistic/eslint-plugin ${UPSTREAM_REF}: ${enriched.valid.length} valid, ${enriched.invalid.length} invalid, ${diagnostics} diagnostics.`,
);

function normalizeCase(raw: RawCase): NormalizedCase {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (typeof value.code !== 'string') {
    throw new TypeError(`Captured ${RULE} case is missing string code.`);
  }

  const clone = JSON.parse(JSON.stringify(value)) as NormalizedCase;
  clone.options = Array.isArray(clone.options) ? clone.options : [];
  if ('parser' in value) {
    clone.parser = 'typescript';
  }
  return clone;
}

function enrichWithPublishedRule(captured: EnrichedFixture): EnrichedFixture {
  const runnerDir = mkdtempSync(join(tmpdir(), 'stylistic-multiline-ternary-upstream-'));
  try {
    writeFileSync(
      join(runnerDir, 'package.json'),
      `${JSON.stringify(
        {
          private: true,
          type: 'module',
          dependencies: {
            '@stylistic/eslint-plugin': UPSTREAM_VERSION,
            '@typescript-eslint/parser': TYPESCRIPT_ESLINT_VERSION,
            eslint: ESLINT_VERSION,
          },
        },
        null,
        2,
      )}\n`,
    );
    writeFileSync(join(runnerDir, 'captured.json'), `${JSON.stringify(captured)}\n`);
    writeFileSync(join(runnerDir, 'runner.mjs'), enrichmentRunnerSource());
    execFileSync(
      'pnpm',
      ['install', '--dir', runnerDir, '--ignore-workspace', '--lockfile=false', '--silent'],
      { stdio: 'inherit' },
    );
    execFileSync('node', [join(runnerDir, 'runner.mjs')], { stdio: 'inherit' });
    return JSON.parse(
      execFileSync(
        'node',
        [
          '-e',
          "process.stdout.write(require('fs').readFileSync(process.argv[1]))",
          join(runnerDir, 'enriched.json'),
        ],
        { encoding: 'utf8' },
      ),
    ) as EnrichedFixture;
  } finally {
    rmSync(runnerDir, { recursive: true, force: true });
  }
}

function enrichmentRunnerSource(): string {
  return `
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Linter } from 'eslint';
import tsParser from '@typescript-eslint/parser';
import * as stylisticModule from '@stylistic/eslint-plugin';

const here = fileURLToPath(new URL('.', import.meta.url));
const captured = JSON.parse(readFileSync(join(here, 'captured.json'), 'utf8'));
const stylistic = stylisticModule.default ?? stylisticModule;
const rule = stylistic.rules['${RULE}'];

function configFor(testCase) {
  return [{
    languageOptions: {
      ecmaVersion: testCase.parserOptions?.ecmaVersion ?? 'latest',
      sourceType: 'module',
      ...(testCase.parser === 'typescript' ? { parser: tsParser } : {}),
      ...(testCase.parserOptions ? { parserOptions: testCase.parserOptions } : {}),
    },
    plugins: {
      stylistic: { rules: { '${RULE}': rule } },
    },
    rules: {
      'stylistic/${RULE}': ['error', ...testCase.options],
    },
  }];
}

function diagnostic(message) {
  return {
    messageId: message.messageId,
    message: message.message,
    line: message.line,
    column: message.column,
    endLine: message.endLine,
    endColumn: message.endColumn,
    ...(message.fix
      ? { fix: { range: message.fix.range, text: message.fix.text } }
      : { fix: null }),
  };
}

function verify(testCase) {
  return new Linter().verify(testCase.code, configFor(testCase), {
    filename: testCase.parser === 'typescript' ? 'fixture.ts' : 'fixture.js',
  });
}

const valid = captured.valid.map((testCase, index) => {
  const messages = verify(testCase);
  if (messages.length !== 0) {
    throw new Error('Published rule reported valid case ' + index + ': ' + JSON.stringify(messages));
  }
  return testCase;
});

const invalid = captured.invalid.map((testCase, index) => {
  const messages = verify(testCase);
  const expectedIds = testCase.errors.map(error => error.messageId);
  const actualIds = messages.map(message => message.messageId);
  if (JSON.stringify(actualIds) !== JSON.stringify(expectedIds)) {
    throw new Error(
      'Published rule IDs differ for invalid case ' + index + ': expected '
      + JSON.stringify(expectedIds) + ', received ' + JSON.stringify(actualIds),
    );
  }

  const fixed = new Linter().verifyAndFix(testCase.code, configFor(testCase), {
    filename: testCase.parser === 'typescript' ? 'fixture.ts' : 'fixture.js',
  });
  const output = fixed.fixed ? fixed.output : null;
  if (Object.prototype.hasOwnProperty.call(testCase, 'output') && testCase.output !== output) {
    throw new Error(
      'Published rule output differs for invalid case ' + index + ': expected '
      + JSON.stringify(testCase.output) + ', received ' + JSON.stringify(output),
    );
  }

  return {
    ...testCase,
    errors: messages.map(diagnostic),
    output,
  };
});

writeFileSync(join(here, 'enriched.json'), JSON.stringify({ valid, invalid }));
`;
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
    'const whitespaceOnly = /^\\s*$/;',
    'export function $(value) {',
    '  const source = typeof value === "string" ? value : value[0];',
    '  const lines = source.split("\\n");',
    '  const blank = lines.map((line) => whitespaceOnly.test(line));',
    '  const commonIndent = lines.reduce((min, line, index) => {',
    '    if (blank[index]) return min;',
    '    return Math.min(min, line.match(/^\\s*/)?.[0].length ?? min);',
    '  }, Number.POSITIVE_INFINITY);',
    '  let head = 0;',
    '  while (head < lines.length && blank[head]) head += 1;',
    '  let tail = 0;',
    '  while (tail < lines.length && blank[lines.length - tail - 1]) tail += 1;',
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
      if (specifier === '@typescript-eslint/parser') {
        return { url: 'stub:///typescript-parser', shortCircuit: true };
      }
      if (
        specifier === './multiline-ternary' ||
        specifier === './types' ||
        specifier === './types.d.ts'
      ) {
        return { url: 'stub:///rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///test') {
        return { format: 'module', source: testStub, shortCircuit: true };
      }
      if (url === 'stub:///typescript-parser') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      if (url === 'stub:///rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}
