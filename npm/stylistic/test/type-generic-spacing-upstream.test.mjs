import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');
const fixture = JSON.parse(
  readFileSync(join(packageRoot, 'test/fixtures/type-generic-spacing.json'), 'utf8'),
);
const rule = plugin.rules['type-generic-spacing'];

function runRule(sourceText, options = [], filename = 'fixture.ts') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = rule.createOnce({
    filename,
    options,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function firstFix(report) {
  return report.suggest?.[0]?.fix({
    replaceTextRange(range, replacementText) {
      return { range, text: replacementText };
    },
  })[0];
}

function applyFixes(sourceText, reports) {
  const fixes = reports
    .map(firstFix)
    .filter(Boolean)
    .sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);
  let output = sourceText;
  for (const fix of fixes) {
    output = output.slice(0, fix.range[0]) + fix.text + output.slice(fix.range[1]);
  }
  return output;
}

function findOxlintCli() {
  const store = join(workspaceRoot, 'node_modules/.pnpm');
  const candidates = readdirSync(store)
    .filter((entry) => entry.startsWith('oxlint@'))
    .map((entry) => join(store, entry, 'node_modules/oxlint/bin/oxlint'))
    .filter((candidate) => existsSync(candidate))
    .sort((left, right) => left.localeCompare(right));
  if (candidates.length === 0) {
    throw new Error('Could not find oxlint CLI in node_modules/.pnpm.');
  }
  return candidates.at(-1);
}

describe('@stylistic/type-generic-spacing v5.10.0 exact parity', () => {
  it('pins the source, replay dependencies, inventory, and diagnostic total', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/type-generic-spacing/type-generic-spacing.test.ts',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-type-generic-spacing-tests.ts',
      exactReplay: {
        eslint: '10.4.1',
        typescriptEslintParser: '8.60.0',
      },
    });
    expect(fixture.valid).toHaveLength(15);
    expect(fixture.invalid).toHaveLength(18);
    expect(fixture.invalid.flatMap((testCase) => testCase.errors)).toHaveLength(28);
    expect(fixture.invalid.every((testCase) => typeof testCase.output === 'string')).toBe(true);
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options)).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays upstream invalid case %i with exact ranges and fixes',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(
        reports.map((report) => ({
          messageId: report.messageId,
          message: rule.meta.messages[report.messageId],
          range: report.node.range,
          fix: firstFix(report),
        })),
      ).toEqual(
        testCase.errors.map((error) => ({
          messageId: error.messageId,
          message: error.message,
          range: error.range,
          fix: error.fix,
        })),
      );
      expect(applyFixes(testCase.code, reports)).toBe(testCase.output);
      expect(runRule(testCase.output, testCase.options)).toEqual([]);
    },
  );
});

describe('type-generic-spacing regression matrix', () => {
  it.each([
    [
      'nested type references',
      'type Box = Outer< Inner< string >, Promise< number > >;',
      'type Box = Outer<Inner<string>, Promise<number>>;',
    ],
    [
      'generic calls',
      'const value = factory< Map< string, Set< number > > >();',
      'const value = factory<Map<string, Set<number>>>();',
    ],
    [
      'generic constructors',
      'const value = new Factory< Array< string > >();',
      'const value = new Factory<Array<string>>();',
    ],
    [
      'instantiation expressions',
      'const ctor = Factory< string >;',
      'const ctor = Factory<string>;',
    ],
  ])('fixes %s recursively in one native pass', (_label, source, output) => {
    const reports = runRule(source);
    expect(reports.length).toBeGreaterThan(0);
    expect(applyFixes(source, reports)).toBe(output);
    expect(runRule(output)).toEqual([]);
  });

  it.each([
    'interface Log { <T>(name: T): void }',
    'const arrow = <T>(name: T) => name;',
    'type FunctionType = <T>(name: T) => T;',
    'type ConstructorType = new <T>(name: T) => T;',
    'const expression = function <T>(name: T) {};',
    'const expression = class <T> {};',
  ])('preserves the upstream prefix space exception in %s', (source) => {
    expect(runRule(source)).toEqual([]);
  });

  it.each([
    ['function named <T>() {}', 'function named<T>() {}'],
    ['class Named <T> {}', 'class Named<T> {}'],
    ['interface Api { method <T>(value: T): T }', 'interface Api { method<T>(value: T): T }'],
    ['interface Api { new <T>(value: T): T }', 'interface Api { new<T>(value: T): T }'],
  ])('removes non-exempt declaration-prefix whitespace from %s', (source, output) => {
    const reports = runRule(source);
    expect(reports).toHaveLength(1);
    expect(applyFixes(source, reports)).toBe(output);
  });

  it('preserves leading CR/LF gaps but matches the upstream comment and Unicode quirks', () => {
    for (const source of [
      'type A = Box<\n string>;',
      'type B = Box<string\n >;',
      'type C<\r\n T> = T;',
    ]) {
      expect(runRule(source)).toEqual([]);
    }

    for (const [source, output] of [
      ['type A = Box< \n string>;', 'type A = Box<string>;'],
      ['type B = Box</* spaced */string>;', 'type B = Box<string>;'],
      ['type C = Box<string/* spaced */>;', 'type C = Box<string>;'],
      ['type D = Box<\u2028string>;', 'type D = Box<string>;'],
    ]) {
      const reports = runRule(source);
      expect(reports).toHaveLength(1);
      expect(applyFixes(source, reports)).toBe(output);
    }
  });

  it.each([
    ['type A<T=true> = T;', 'type A<T = true> = T;'],
    [
      'type B<T extends string=Array<number>> = T;',
      'type B<T extends string = Array<number>> = T;',
    ],
    ['type C<T/* keep */=/* keep */string> = T;', 'type C<T/* keep */ = /* keep */string> = T;'],
    ['type D<T\t=\tstring> = T;', 'type D<T = string> = T;'],
    ['type E<T\n=\nstring> = T;', 'type E<T = string> = T;'],
  ])('normalizes default spacing in %s', (source, output) => {
    const reports = runRule(source);
    expect(reports).toHaveLength(1);
    expect(applyFixes(source, reports)).toBe(output);
  });

  it('maps UTF-8 native offsets to exact UTF-16 plugin ranges across CRLF', () => {
    const source = 'type 日本語< 値 = string > = 値;\r\nconst 結果 = factory< 数字 >();';
    const reports = runRule(source);
    const firstValue = source.indexOf('値');
    expect(reports.map((report) => report.node.range)).toEqual([
      [firstValue - 1, firstValue],
      [source.indexOf(' >'), source.indexOf(' >') + 1],
      [source.indexOf(' 数字'), source.indexOf(' 数字') + 1],
      [
        source.indexOf(' >', source.indexOf(' 数字')),
        source.indexOf(' >', source.indexOf(' 数字')) + 1,
      ],
    ]);
    expect(applyFixes(source, reports)).toBe(
      'type 日本語<値 = string> = 値;\r\nconst 結果 = factory<数字>();',
    );
  });

  it('handles TSX generic arrows without treating JSX tags as generics', () => {
    const source = [
      'const identity = <T,>(value: T) => <Panel value={value} />;',
      'const nested = <T extends Box< string >,>(value: T) => value;',
    ].join('\n');
    const reports = runRule(source, [], 'fixture.tsx');
    expect(reports.map((report) => report.messageId)).toEqual([
      'genericSpacingMismatch',
      'genericSpacingMismatch',
    ]);
    expect(applyFixes(source, reports)).toBe(
      [
        'const identity = <T,>(value: T) => <Panel value={value} />;',
        'const nested = <T extends Box<string>,>(value: T) => value;',
      ].join('\n'),
    );
  });

  it.each(['type Broken<T = > = T;', 'const value = factory<;', 'const jsx = <Panel value={;'])(
    'does not emit diagnostics for invalid syntax: %s',
    (source) => {
      expect(runRule(source, [], source.includes('jsx') ? 'fixture.tsx' : 'fixture.ts')).toEqual(
        [],
      );
    },
  );

  it.each([[], [{}], ['never'], [42], [null]])(
    'ignores unsupported option shape %j because the stable schema is empty',
    (options) => {
      expect(runRule('type Box< T > = T;', options)).toHaveLength(2);
    },
  );

  it('runs and fixes the rule through an actual Oxlint jsPlugins config', () => {
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-type-generic-spacing-'));
    try {
      const sourcePath = join(temp, 'sample.ts');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(sourcePath, 'type 日本語< 値=string > = 値;\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: { 'stylistic/type-generic-spacing': 'error' },
        }),
      );

      const lint = spawnSync(
        findOxlintCli(),
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
        { encoding: 'utf8' },
      );
      expect(lint.status).toBe(1);
      expect(lint.stderr).toBe('');
      const diagnostics = JSON.parse(lint.stdout).diagnostics;
      expect(diagnostics).toHaveLength(3);
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(type-generic-spacing)',
        'stylistic(type-generic-spacing)',
        'stylistic(type-generic-spacing)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Generic spaces mismatch',
        'Generic spaces mismatch',
        'Generic spaces mismatch',
      ]);

      const fix = spawnSync(findOxlintCli(), ['-c', configPath, '--fix-suggestions', sourcePath], {
        encoding: 'utf8',
      });
      expect(fix.status).toBe(0);
      expect(fix.stderr).toBe('');
      expect(readFileSync(sourcePath, 'utf8')).toBe('type 日本語<値 = string> = 値;\n');
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });
});
