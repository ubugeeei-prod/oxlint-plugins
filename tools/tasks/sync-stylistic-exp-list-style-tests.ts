// Captures every authored @stylistic/exp-list-style v5.10.0 RuleTester case
// from the exact vendored commit, then enriches it with the published rule's
// exact diagnostics, UTF-16 ranges, fixes, and recursive fixed output.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type Language = 'typescript' | 'json';
type CapturedRun = {
  name: string;
  lang: 'ts' | 'json';
  valid: RawCase[];
  invalid: RawCase[];
};
type NormalizedCase = {
  code: string;
  options: unknown[];
  language: Language;
  [key: string]: unknown;
};
type Manifest = {
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    baselineVersion: string;
    pinnedRef?: string;
    license: string;
  }>;
};

const ROOT = process.cwd();
const RULE = 'exp-list-style';
const UPSTREAM_RULE = 'list-style';
const VERSION = '5.10.0';
const PINNED_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const ESLINT_VERSION = '10.0.0';
const TYPESCRIPT_ESLINT_VERSION = '8.56.0';
const JSONC_ESLINT_PARSER_VERSION = '2.4.2';
const SOURCE_FILES = [
  `packages/eslint-plugin/rules/${UPSTREAM_RULE}/${UPSTREAM_RULE}.test.ts`,
  `packages/eslint-plugin/rules/${UPSTREAM_RULE}/${UPSTREAM_RULE}._json_.test.ts`,
] as const;
const CAPTURE_KEY = '__stylisticExpListStyleCapture__';

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-stylistic');
if (!plugin) {
  throw new Error('eslint-stylistic is not registered in tools/port-targets.json');
}
if (plugin.baselineVersion !== VERSION || plugin.pinnedRef !== `v${VERSION}`) {
  throw new Error(
    `Expected @stylistic v${VERSION} manifest pin, received ` +
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

registerCaptureHooks();
const captureDir = mkdtempSync(join(tmpdir(), 'stylistic-exp-list-style-capture-'));
try {
  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  for (const [index, sourceFile] of SOURCE_FILES.entries()) {
    const captureFile = join(captureDir, `list-style-${index}.test.ts`);
    writeFileSync(
      captureFile,
      execFileSync('git', ['-C', submodule, 'show', `${PINNED_COMMIT}:${sourceFile}`], {
        encoding: 'utf8',
      }),
    );
    await import(`${pathToFileURL(captureFile).href}?commit=${PINNED_COMMIT}&suite=${index}`);
  }

  const captures = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
  if (
    captures.length !== SOURCE_FILES.length ||
    captures.some((capture) => capture.name !== UPSTREAM_RULE)
  ) {
    throw new Error(`Expected ${SOURCE_FILES.length} captured ${UPSTREAM_RULE} suites.`);
  }

  const captured = {
    valid: captures.flatMap((capture) =>
      capture.valid.map((testCase, index) =>
        normalizeCase(testCase, false, capture.lang, `valid ${capture.lang} ${index}`),
      ),
    ),
    invalid: captures.flatMap((capture) =>
      capture.invalid.map((testCase, index) =>
        normalizeCase(testCase, true, capture.lang, `invalid ${capture.lang} ${index}`),
      ),
    ),
  };
  const enriched = enrichWithPublishedRule(captured);
  const diagnostics = enriched.invalid.reduce(
    (total, testCase) =>
      total + (testCase.expectedDiagnostics as Array<Record<string, unknown>>).length,
    0,
  );
  const unfixableInvalid = enriched.invalid.filter((testCase) => testCase.output === null).length;
  const fixture = {
    __generated: {
      source: plugin.npm,
      version: plugin.baselineVersion,
      sourceCommit: PINNED_COMMIT,
      sourceFiles: SOURCE_FILES,
      license: plugin.license,
      eslintVersion: ESLINT_VERSION,
      parserVersions: {
        typescriptEslint: TYPESCRIPT_ESLINT_VERSION,
        typescript: '5.9.3',
        jsoncEslintParser: JSONC_ESLINT_PARSER_VERSION,
      },
      tool: 'tools/tasks/sync-stylistic-exp-list-style-tests.ts',
      inventory: {
        logicalValid: captured.valid.length,
        logicalInvalid: captured.invalid.length,
        valid: enriched.valid.length,
        invalid: enriched.invalid.length,
        diagnostics,
        unfixableInvalid,
        total: enriched.valid.length + enriched.invalid.length,
        fixableInvalid: enriched.invalid.length - unfixableInvalid,
        languages: Object.fromEntries(
          (['typescript', 'json'] as const).map((language) => [
            language,
            {
              valid: enriched.valid.filter((testCase) => testCase.language === language).length,
              invalid: enriched.invalid.filter((testCase) => testCase.language === language).length,
            },
          ]),
        ),
      },
    },
    valid: enriched.valid,
    invalid: enriched.invalid,
  };

  const fixturePath = join(
    ROOT,
    'npm',
    'stylistic',
    'test',
    'fixtures',
    `${RULE}-v${VERSION}.json`,
  );
  mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
  writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('pnpm', ['exec', 'vp', 'fmt', fixturePath], {
    cwd: ROOT,
    stdio: 'inherit',
  });
  console.log(
    `Synced @stylistic/${RULE} v${VERSION} (${PINNED_COMMIT}): ` +
      `${enriched.valid.length} valid, ${enriched.invalid.length} invalid, ` +
      `${diagnostics} diagnostics (${unfixableInvalid} unfixable).`,
  );
} finally {
  rmSync(captureDir, { recursive: true, force: true });
}

function normalizeCase(
  raw: RawCase,
  invalid: boolean,
  lang: CapturedRun['lang'],
  label: string,
): NormalizedCase {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`Unsupported ${label}`);
  }
  const allowed = new Set(
    invalid
      ? ['code', 'options', 'parserOptions', 'output', 'errors', 'description']
      : ['code', 'options', 'parserOptions', 'description'],
  );
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported ${label} keys: ${unsupported.join(', ')}`);
  }

  const normalized: NormalizedCase = {
    code: value.code,
    options: Array.isArray(value.options) ? clone(value.options) : [],
    language: lang === 'json' ? 'json' : 'typescript',
    ...('parserOptions' in value ? { parserOptions: clone(value.parserOptions) } : {}),
    ...(typeof value.description === 'string' ? { description: value.description } : {}),
  };
  if (!invalid) {
    return normalized;
  }
  if (!('output' in value) || (typeof value.output !== 'string' && value.output !== null)) {
    throw new Error(`${label} is missing its fixed output`);
  }
  if (!Array.isArray(value.errors)) {
    throw new Error(`${label} is missing its ordered errors`);
  }
  return {
    ...normalized,
    authoredOutput: value.output,
    upstreamErrors: clone(value.errors),
  };
}

function enrichWithPublishedRule(cases: { valid: NormalizedCase[]; invalid: NormalizedCase[] }): {
  valid: NormalizedCase[];
  invalid: NormalizedCase[];
} {
  const runnerDir = mkdtempSync(join(tmpdir(), 'stylistic-exp-list-style-upstream-'));
  try {
    writeFileSync(
      join(runnerDir, 'package.json'),
      `${JSON.stringify(
        {
          private: true,
          type: 'module',
          dependencies: {
            '@stylistic/eslint-plugin': VERSION,
            '@typescript-eslint/parser': TYPESCRIPT_ESLINT_VERSION,
            eslint: ESLINT_VERSION,
            'jsonc-eslint-parser': JSONC_ESLINT_PARSER_VERSION,
            typescript: '5.9.3',
          },
        },
        null,
        2,
      )}\n`,
    );
    writeFileSync(join(runnerDir, 'captured.json'), `${JSON.stringify(cases)}\n`);
    writeFileSync(join(runnerDir, 'runner.mjs'), enrichmentRunnerSource());
    execFileSync(
      'pnpm',
      ['install', '--dir', runnerDir, '--ignore-workspace', '--lockfile=false', '--silent'],
      { stdio: 'inherit' },
    );
    execFileSync('node', [join(runnerDir, 'runner.mjs')], { stdio: 'inherit' });
    return JSON.parse(readFileSync(join(runnerDir, 'enriched.json'), 'utf8'));
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
import jsonParser from 'jsonc-eslint-parser';
import * as stylisticModule from '@stylistic/eslint-plugin';

const here = fileURLToPath(new URL('.', import.meta.url));
const captured = JSON.parse(readFileSync(join(here, 'captured.json'), 'utf8'));
const stylistic = stylisticModule.default ?? stylisticModule;
const rule = stylistic.rules['${RULE}'];

function filenameFor(testCase) {
  return testCase.language === 'json' ? 'fixture.json' : 'fixture.ts';
}

function configFor(testCase) {
  const parser = testCase.language === 'json' ? jsonParser : tsParser;
  return [{
    files: [testCase.language === 'json' ? '**/*.json' : '**/*.ts'],
    languageOptions: {
      parser,
      ...(testCase.language === 'typescript'
        ? {
            ecmaVersion: testCase.parserOptions?.ecmaVersion ?? 'latest',
            sourceType: testCase.parserOptions?.sourceType ?? 'module',
          }
        : {}),
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

function offsetAt(source, line, column) {
  let currentLine = 1;
  let index = 0;
  while (currentLine < line && index < source.length) {
    const code = source.charCodeAt(index);
    if (code === 13) {
      index += source.charCodeAt(index + 1) === 10 ? 2 : 1;
      currentLine += 1;
    } else if (code === 10 || code === 0x2028 || code === 0x2029) {
      index += 1;
      currentLine += 1;
    } else {
      index += 1;
    }
  }
  return index + column - 1;
}

function diagnostic(source, message) {
  const range = [
    offsetAt(source, message.line, message.column),
    offsetAt(source, message.endLine, message.endColumn),
  ];
  return {
    messageId: message.messageId,
    message: message.message,
    data: messageData(message.messageId, message.message),
    range,
    loc: {
      line: message.line,
      column: message.column,
      endLine: message.endLine,
      endColumn: message.endColumn,
    },
    ...(message.fix
      ? { fix: { range: message.fix.range, text: message.fix.text } }
      : { fix: null }),
  };
}

function messageData(messageId, message) {
  const prefixes = {
    shouldSpacing: 'Should have space between ',
    shouldNotSpacing: 'Should not have space(s) between ',
    shouldWrap: 'Should have line break between ',
    shouldNotWrap: 'Should not have line break(s) between ',
  };
  const prefix = prefixes[messageId];
  const suffix = message.slice(prefix.length);
  const marker = "' and '";
  const split = suffix.indexOf(marker, 1);
  if (!prefix || !suffix.startsWith("'") || split < 0 || !suffix.endsWith("'")) {
    throw new Error('Cannot recover diagnostic data from ' + JSON.stringify(message));
  }
  return {
    prev: suffix.slice(1, split),
    next: suffix.slice(split + marker.length, -1),
  };
}

function verify(testCase) {
  return new Linter().verify(testCase.code, configFor(testCase), {
    filename: filenameFor(testCase),
  });
}

function applySinglePass(source, messages) {
  const fixes = messages
    .map(message => message.fix)
    .filter(Boolean)
    .sort((left, right) => left.range[0] - right.range[0] || left.range[1] - right.range[1]);
  if (fixes.length === 0) {
    return null;
  }

  let output = '';
  let last = 0;
  for (const fix of fixes) {
    if (last > fix.range[0]) {
      continue;
    }
    output += source.slice(last, fix.range[0]) + fix.text;
    last = fix.range[1];
  }
  return output + source.slice(last);
}

const valid = captured.valid.map((testCase, index) => {
  const messages = verify(testCase);
  if (messages.length !== 0) {
    throw new Error(
      'Published rule reported valid case ' + index + ': ' + JSON.stringify(testCase)
      + ' -> ' + JSON.stringify(messages),
    );
  }
  return testCase;
});

const invalid = captured.invalid.map((testCase, index) => {
  const messages = verify(testCase);
  const expectedIds = testCase.upstreamErrors.map(error => error.messageId);
  const actualIds = messages.map(message => message.messageId);
  if (JSON.stringify(actualIds) !== JSON.stringify(expectedIds)) {
    throw new Error(
      'Published rule IDs differ for invalid case ' + index + ': expected '
      + JSON.stringify(expectedIds) + ', received ' + JSON.stringify(actualIds),
    );
  }
  for (const [messageIndex, expected] of testCase.upstreamErrors.entries()) {
    const actual = messages[messageIndex];
    if (
      (expected.line !== undefined && expected.line !== actual.line)
      || (expected.column !== undefined && expected.column !== actual.column)
    ) {
      throw new Error(
        'Published rule location differs for invalid case ' + index + ', message '
        + messageIndex + ': expected ' + JSON.stringify(expected)
        + ', received ' + JSON.stringify(actual),
      );
    }
  }

  const fixed = new Linter().verifyAndFix(testCase.code, configFor(testCase), {
    filename: filenameFor(testCase),
  });
  const output = fixed.fixed ? fixed.output : null;
  const authoredOutput = applySinglePass(testCase.code, messages);
  if (testCase.authoredOutput !== authoredOutput) {
    throw new Error(
      'Published rule output differs for invalid case ' + index + ': expected '
      + JSON.stringify(testCase.authoredOutput) + ', received ' + JSON.stringify(authoredOutput),
    );
  }

  const { authoredOutput: _capturedOutput, upstreamErrors, ...normalized } = testCase;
  return {
    ...normalized,
    expectedDiagnostics: messages.map(message => diagnostic(testCase.code, message)),
    authoredOutput,
    output,
  };
});

writeFileSync(join(here, 'enriched.json'), JSON.stringify({ valid, invalid }));
`;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export function run(options) {',
    '  globalThis[key].push({',
    '    name: options.name,',
    '    lang: options.lang,',
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
      if (specifier === './list-style' || specifier === './types' || specifier === './types.d.ts') {
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

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
