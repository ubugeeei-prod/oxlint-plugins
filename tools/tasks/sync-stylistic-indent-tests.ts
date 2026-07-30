// Captures every stable @stylistic/indent v5.10.0 RuleTester case from the
// exact pinned upstream commit. JavaScript, TypeScript, JSX/TSX, and CSS
// authored suites are kept separate so parser coverage remains auditable.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type Capture = {
  name: string;
  lang?: string;
  parserOptions?: Record<string, unknown>;
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
const RULE = 'indent';
const VERSION = '5.10.0';
const PINNED_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const CAPTURE_KEY = '__stylisticIndentCapture__';
const SOURCE_FILES = [
  'packages/eslint-plugin/rules/indent/indent._js_.test.ts',
  'packages/eslint-plugin/rules/indent/indent._jsx_.test.ts',
  'packages/eslint-plugin/rules/indent/indent._ts_.test.ts',
  'packages/eslint-plugin/rules/indent/indent._css_.test.ts',
] as const;
const SUPPORT_FILES = [
  'packages/eslint-plugin/rules/indent/indent.ts',
  'packages/eslint-plugin/rules/indent/fixtures/indent-invalid-fixture-1.js',
  'packages/eslint-plugin/rules/indent/fixtures/indent-valid-fixture-1.js',
] as const;

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

const indentRoot = join(submodule, plugin.packageSubdir ?? '.', 'rules', RULE);
(globalThis as Record<string, unknown>).__dirname = indentRoot;
(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
registerCaptureHooks();

for (const sourceFile of SOURCE_FILES) {
  const absolute = join(submodule, sourceFile);
  if (!existsSync(absolute)) {
    throw new Error(`Pinned upstream source is missing: ${sourceFile}`);
  }
  await import(pathToFileURL(absolute).href);
}

const captures = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as Capture[];
if (captures.length !== SOURCE_FILES.length) {
  throw new Error(`Expected ${SOURCE_FILES.length} captured suites, received ${captures.length}.`);
}

const capturedSuites = captures.map((capture, suiteIndex) => {
  const sourceFile = SOURCE_FILES[suiteIndex];
  const language = capture.lang ?? (capture.name === 'jsx-indent' ? 'jsx' : 'ts');
  return {
    name: capture.name,
    language,
    sourceFile,
    sourceHash: sha256(join(submodule, sourceFile)),
    valid: capture.valid.map((testCase, index) =>
      normalizeCase(testCase, capture, language, false, `${capture.name} valid ${index}`),
    ),
    invalid: capture.invalid.map((testCase, index) =>
      normalizeCase(testCase, capture, language, true, `${capture.name} invalid ${index}`),
    ),
  };
});

const suites = enrichWithPublishedRule(capturedSuites);
const fixture = {
  __generated: {
    source: plugin.npm,
    version: VERSION,
    sourceCommit: PINNED_COMMIT,
    sourceFiles: SOURCE_FILES,
    sourceHashes: Object.fromEntries(
      [...SOURCE_FILES, ...SUPPORT_FILES].map((sourceFile) => [
        sourceFile,
        sha256(join(submodule, sourceFile)),
      ]),
    ),
    license: plugin.license,
    parserMatrix: 'ESLint 10: Babel disabled; JSX authored cases expand to Espree and TypeScript',
    tool: 'tools/tasks/sync-stylistic-indent-tests.ts',
    inventory: {
      suites: suites.map((suite) => ({
        name: suite.name,
        language: suite.language,
        valid: suite.valid.length,
        invalid: suite.invalid.length,
      })),
      valid: suites.reduce((total, suite) => total + suite.valid.length, 0),
      invalid: suites.reduce((total, suite) => total + suite.invalid.length, 0),
      diagnostics: suites.reduce(
        (total, suite) =>
          total +
          suite.invalid.reduce(
            (suiteTotal, testCase) =>
              suiteTotal + (testCase.expectedDiagnostics as Array<Record<string, unknown>>).length,
            0,
          ),
        0,
      ),
      fixableInvalid: suites.reduce(
        (total, suite) =>
          total + suite.invalid.filter((testCase) => typeof testCase.output === 'string').length,
        0,
      ),
    },
  },
  suites,
};

const fixturePath = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures', `${RULE}-v${VERSION}.json`);
mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('pnpm', ['exec', 'vp', 'fmt', fixturePath], {
  cwd: ROOT,
  stdio: 'inherit',
});
console.log(
  `Captured @stylistic/${RULE} v${VERSION} (${PINNED_COMMIT}): ` +
    `${fixture.__generated.inventory.valid} valid, ` +
    `${fixture.__generated.inventory.invalid} invalid.`,
);

function normalizeCase(
  raw: RawCase,
  capture: Capture,
  language: string,
  invalid: boolean,
  label: string,
): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`Unsupported ${label}`);
  }

  const parser = parserFor(value, language);
  const normalized: Record<string, unknown> = {
    code: value.code,
    options: Array.isArray(value.options) ? clone(value.options) : [],
    parser,
  };
  const parserOptions = parserOptionsFor(value, capture);
  if (Object.keys(parserOptions).length > 0) {
    normalized.parserOptions = parserOptions;
  }
  if (!invalid) {
    return normalized;
  }
  if ('errors' in value && !Array.isArray(value.errors) && typeof value.errors !== 'number') {
    throw new Error(`${label} has unsupported errors`);
  }
  if ('output' in value && typeof value.output !== 'string' && value.output !== null) {
    throw new Error(`${label} has an unsupported output`);
  }
  return {
    ...normalized,
    upstreamOutput: 'output' in value ? value.output : null,
    upstreamOutputSpecified: 'output' in value,
    upstreamErrors: clone(value.errors ?? null),
  };
}

function parserFor(value: Record<string, unknown>, language: string): string {
  if (
    value.parser === 'typescript' ||
    (value.parser as { __parser?: string } | undefined)?.__parser === 'typescript' ||
    marker(value.languageOptions) === 'typescript'
  ) {
    return 'typescript';
  }
  if (value.parser === 'espree') {
    return 'espree';
  }
  if (language === 'ts' || language === 'jsx') {
    return 'typescript';
  }
  if (language === 'css') {
    return 'css';
  }
  return 'espree';
}

function parserOptionsFor(
  value: Record<string, unknown>,
  capture: Capture,
): Record<string, unknown> {
  const languageOptions =
    value.languageOptions && typeof value.languageOptions === 'object'
      ? (value.languageOptions as Record<string, unknown>)
      : {};
  const caseOptions =
    value.parserOptions && typeof value.parserOptions === 'object'
      ? (value.parserOptions as Record<string, unknown>)
      : {};
  const languageParserOptions =
    languageOptions.parserOptions && typeof languageOptions.parserOptions === 'object'
      ? (languageOptions.parserOptions as Record<string, unknown>)
      : {};
  return clone({
    ...capture.parserOptions,
    ...caseOptions,
    ...languageParserOptions,
  });
}

function marker(value: unknown): string | undefined {
  if (!value || typeof value !== 'object') {
    return undefined;
  }
  return (value as { parser?: { __parser?: string } }).parser?.__parser;
}

function sha256(path: string): string {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function enrichWithPublishedRule(suites: Array<Record<string, unknown>>): Array<{
  name: string;
  language: string;
  sourceFile: string;
  sourceHash: string;
  valid: Array<Record<string, unknown>>;
  invalid: Array<Record<string, unknown>>;
}> {
  const runnerDir = mkdtempSync(join(tmpdir(), 'stylistic-indent-upstream-'));
  try {
    writeFileSync(
      join(runnerDir, 'package.json'),
      `${JSON.stringify(
        {
          private: true,
          type: 'module',
          dependencies: {
            '@stylistic/eslint-plugin': VERSION,
            '@typescript-eslint/parser': '8.56.0',
            eslint: '10.0.0',
            typescript: '5.9.3',
          },
        },
        null,
        2,
      )}\n`,
    );
    writeFileSync(join(runnerDir, 'captured.json'), `${JSON.stringify(suites)}\n`);
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
import typescriptParser from '@typescript-eslint/parser';
import { Linter } from 'eslint';
import { t as rule } from '@stylistic/eslint-plugin/rules/indent';

const here = fileURLToPath(new URL('.', import.meta.url));
const suites = JSON.parse(readFileSync(join(here, 'captured.json'), 'utf8'));
const linter = new Linter();

function configFor(testCase) {
  const parserOptions = testCase.parserOptions ?? {};
  const { sourceType: parserSourceType, ...restParserOptions } = parserOptions;
  return [{
    files: ['**/*.{js,jsx,ts,tsx}'],
    languageOptions: {
      ecmaVersion: parserOptions.ecmaVersion ?? 'latest',
      sourceType: parserSourceType ?? 'module',
      ...(testCase.parser === 'typescript' ? { parser: typescriptParser } : {}),
      parserOptions: restParserOptions,
    },
    plugins: {
      stylistic: { rules: { indent: rule } },
    },
    rules: {
      'stylistic/indent': ['error', ...testCase.options],
    },
  }];
}

function filenameFor(testCase, language) {
  if (testCase.parser === 'typescript') {
    return language === 'jsx' ? 'fixture.tsx' : 'fixture.ts';
  }
  return language === 'jsx' ? 'fixture.jsx' : 'fixture.js';
}

function verify(testCase, language, source = testCase.code) {
  const messages = linter.verify(source, configFor(testCase), {
    filename: filenameFor(testCase, language),
  });
  const ruleMessages = messages.filter(message => message.ruleId === 'stylistic/indent');
  const unexpected = messages.filter(message => message.ruleId !== 'stylistic/indent');
  if (unexpected.length !== 0) {
    throw new Error(
      'Unexpected ESLint messages for ' + language + ': ' + JSON.stringify(unexpected),
    );
  }
  return ruleMessages;
}

function offsetAt(source, line, column) {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line) {
    const match = /\\r\\n|[\\n\\r\\u2028\\u2029]/u.exec(source.slice(offset));
    if (!match) {
      throw new Error('Cannot map diagnostic location');
    }
    offset += match.index + match[0].length;
    currentLine += 1;
  }
  return offset + column - 1;
}

function messageData(message) {
  const match = /^Expected indentation of (.+) but found (.+)\\.$/u.exec(message.message);
  if (!match) {
    throw new Error('Unexpected indent message: ' + message.message);
  }
  return { expected: match[1], actual: /^\\d+$/u.test(match[2]) ? Number(match[2]) : match[2] };
}

function diagnostic(source, message) {
  const start = offsetAt(source, message.line, message.column);
  const end =
    message.endLine === undefined || message.endColumn === undefined
      ? start
      : offsetAt(source, message.endLine, message.endColumn);
  return {
    messageId: message.messageId,
    message: message.message,
    data: messageData(message),
    range: [start, end],
    loc: {
      line: message.line,
      column: message.column,
      endLine: message.endLine,
      endColumn: message.endColumn,
    },
    fix: message.fix ? { range: message.fix.range, text: message.fix.text } : null,
  };
}

function singlePassOutput(source, messages) {
  const fixes = messages.flatMap(message => (message.fix ? [message.fix] : []));
  if (fixes.length === 0) {
    return null;
  }
  fixes.sort((left, right) => left.range[0] - right.range[0] || left.range[1] - right.range[1]);
  let output = '';
  let lastPosition = Number.NEGATIVE_INFINITY;
  for (const fix of fixes) {
    if (lastPosition >= fix.range[0]) {
      continue;
    }
    output += source.slice(Math.max(0, lastPosition), fix.range[0]) + fix.text;
    lastPosition = fix.range[1];
  }
  return output + source.slice(Math.max(0, lastPosition));
}

for (const suite of suites) {
  if (suite.language === 'css') {
    continue;
  }
  suite.valid = suite.valid.map((testCase, index) => {
    const messages = verify(testCase, suite.language);
    if (messages.length !== 0) {
      throw new Error(
        'Published rule reported valid ' + suite.name + ' case ' + index + ': ' +
          JSON.stringify(messages),
      );
    }
    return testCase;
  });
  suite.invalid = suite.invalid.map((testCase, index) => {
    const messages = verify(testCase, suite.language);
    if (messages.length === 0) {
      throw new Error('Published rule did not report invalid ' + suite.name + ' case ' + index);
    }
    if (Array.isArray(testCase.upstreamErrors)) {
      if (messages.length !== testCase.upstreamErrors.length) {
        throw new Error(
          'Published rule count differs for ' + suite.name + ' invalid case ' + index +
            ': expected ' + testCase.upstreamErrors.length + ', received ' + messages.length,
        );
      }
      for (const [messageIndex, expected] of testCase.upstreamErrors.entries()) {
        const actual = messages[messageIndex];
        for (const key of ['messageId', 'line', 'column', 'endLine', 'endColumn']) {
          if (expected[key] !== undefined && actual[key] !== expected[key]) {
            throw new Error(
              'Published rule differs for ' + suite.name + ' invalid case ' + index +
                ', message ' + messageIndex + ', ' + key + ': expected ' + expected[key] +
                ', received ' + actual[key],
            );
          }
        }
        if (expected.data !== undefined) {
          const actualData = messageData(actual);
          if (JSON.stringify(expected.data) !== JSON.stringify(actualData)) {
            throw new Error(
              'Published rule data differs for ' + suite.name + ' invalid case ' + index +
                ', message ' + messageIndex + ': expected ' + JSON.stringify(expected.data) +
                ', received ' + JSON.stringify(actualData),
            );
          }
        }
      }
    } else if (
      typeof testCase.upstreamErrors === 'number' &&
      messages.length !== testCase.upstreamErrors
    ) {
      throw new Error(
        'Published rule count differs for ' + suite.name + ' invalid case ' + index +
          ': expected ' + testCase.upstreamErrors + ', received ' + messages.length,
      );
    }

    const output = singlePassOutput(testCase.code, messages);
    if (testCase.upstreamOutputSpecified && output !== testCase.upstreamOutput) {
      throw new Error(
        'Published rule output differs for ' + suite.name + ' invalid case ' + index +
          ': expected ' + JSON.stringify(testCase.upstreamOutput) +
          ', received ' + JSON.stringify(output),
      );
    }
    const fixed = linter.verifyAndFix(testCase.code, configFor(testCase), {
      filename: filenameFor(testCase, suite.language),
    });
    const recursiveMessages = verify(testCase, suite.language, fixed.output);
    return {
      code: testCase.code,
      options: testCase.options,
      parser: testCase.parser,
      ...(testCase.parserOptions ? { parserOptions: testCase.parserOptions } : {}),
      output,
      recursiveOutput: fixed.fixed ? fixed.output : null,
      upstreamErrors: testCase.upstreamErrors,
      expectedDiagnostics: messages.map(message => diagnostic(testCase.code, message)),
      recursiveDiagnostics: recursiveMessages.map(message => diagnostic(fixed.output, message)),
    };
  });
}

writeFileSync(join(here, 'enriched.json'), JSON.stringify(suites));
`;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export const skipBabel = true;',
    `export const $ = ${unindent.toString()};`,
    'export function run(options) {',
    '  globalThis[key].push({',
    '    name: options.name,',
    '    lang: options.lang,',
    '    parserOptions: options.parserOptions,',
    '    valid: options.valid || [],',
    '    invalid: options.invalid || [],',
    '  });',
    '}',
  ].join('\n');
  const parsersJsxStub = `
function addParser(testCase, parser, features) {
  const value = typeof testCase === 'string' ? { code: testCase } : testCase;
  if ('parser' in value) {
    return value;
  }
  const parserOptions = {
    ...(value.parserOptions || {}),
    ecmaFeatures: {
      ...(value.parserOptions?.ecmaFeatures || {}),
      jsx: true,
      modules: true,
      legacyDecorators: features.has('decorators'),
    },
  };
  const extras = [
    'features: [' + Array.from(features).join(',') + ']',
    'parser: ' + (parser === 'espree' ? 'default' : '@typescript-eslint/parser'),
    value.parserOptions ? 'parserOptions: ' + JSON.stringify(value.parserOptions) : '',
    value.options ? 'options: ' + JSON.stringify(value.options) : '',
    value.settings ? 'settings: ' + JSON.stringify(value.settings) : '',
  ];
  const extraComment = '\\n// ' + extras.join(', ');
  const { features: _features, ...rest } = value;
  return {
    ...rest,
    code: value.code + extraComment,
    ...(value.output ? { output: value.output + extraComment } : {}),
    ...(parser === 'typescript'
      ? { languageOptions: { parser: { __parser: 'typescript' }, parserOptions } }
      : {}),
    parser,
  };
}
function expand(tests) {
  return tests.flat().filter(Boolean).flatMap(testCase => {
    const value = typeof testCase === 'string' ? { code: testCase } : testCase;
    const features = new Set(value.features || []);
    const skipBase = [
      'class fields', 'no-default', 'bind operator', 'do expressions',
      'decorators', 'flow', 'ts', 'types', 'fragment',
    ].some(feature => features.has(feature));
    const skipTS = [
      'no-ts', 'flow', 'jsx namespace', 'bind operator', 'do expressions',
    ].some(feature => features.has(feature));
    return [
      ...(skipBase ? [] : [addParser(value, 'espree', features)]),
      ...(skipTS || features.has('no-ts-new')
        ? []
        : [addParser(value, 'typescript', features)]),
    ];
  });
}
export function valids(...tests) { return expand(tests); }
export function invalids(...tests) { return expand(tests); }
`;

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///stylistic-indent-test', shortCircuit: true };
      }
      if (specifier === '#test/parsers-jsx') {
        return { url: 'stub:///stylistic-indent-parsers-jsx', shortCircuit: true };
      }
      if (specifier === '#test/parsers-flow') {
        return { url: 'stub:///stylistic-indent-parsers-flow', shortCircuit: true };
      }
      if (specifier === '#utils/ast') {
        return { url: 'stub:///stylistic-indent-ast', shortCircuit: true };
      }
      if (specifier === '@typescript-eslint/parser') {
        return { url: 'stub:///stylistic-indent-typescript-parser', shortCircuit: true };
      }
      if (specifier === './indent._js_.test') {
        return {
          url: pathToFileURL(join(indentRoot, 'indent._js_.test.ts')).href,
          shortCircuit: true,
        };
      }
      if (specifier === './indent' || specifier === './types' || specifier === './types.d.ts') {
        return { url: 'stub:///stylistic-indent-rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///stylistic-indent-test') {
        return { format: 'module', source: testStub, shortCircuit: true };
      }
      if (url === 'stub:///stylistic-indent-parsers-jsx') {
        return { format: 'module', source: parsersJsxStub, shortCircuit: true };
      }
      if (url === 'stub:///stylistic-indent-parsers-flow') {
        return {
          format: 'module',
          source: 'export const languageOptionsForBabelFlow = {};',
          shortCircuit: true,
        };
      }
      if (url === 'stub:///stylistic-indent-ast') {
        return {
          format: 'module',
          source:
            'export const AST_NODE_TYPES = new Proxy({}, { get: (_target, key) => String(key) });',
          shortCircuit: true,
        };
      }
      if (url === 'stub:///stylistic-indent-typescript-parser') {
        return {
          format: 'module',
          source: "export default { __parser: 'typescript' };",
          shortCircuit: true,
        };
      }
      if (url === 'stub:///stylistic-indent-rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}

function unindent(value: string | TemplateStringsArray): string {
  const lines = (typeof value === 'string' ? value : value[0]).split('\n');
  const whitespaceLines = lines.map((line) => /^\s*$/.test(line));
  const commonIndent = lines.reduce((minimum, line, index) => {
    if (whitespaceLines[index]) {
      return minimum;
    }
    return Math.min(minimum, line.match(/^\s*/)?.[0].length ?? minimum);
  }, Number.POSITIVE_INFINITY);
  let emptyLinesHead = 0;
  while (emptyLinesHead < lines.length && whitespaceLines[emptyLinesHead]) {
    emptyLinesHead += 1;
  }
  let emptyLinesTail = 0;
  while (emptyLinesTail < lines.length && whitespaceLines[lines.length - emptyLinesTail - 1]) {
    emptyLinesTail += 1;
  }
  return lines
    .slice(emptyLinesHead, lines.length - emptyLinesTail)
    .map((line) => line.slice(commonIndent))
    .join('\n');
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
