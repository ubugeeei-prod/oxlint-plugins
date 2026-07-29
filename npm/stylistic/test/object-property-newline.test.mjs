import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { runNativeStylisticLint } from '../api.js';
import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');
const rule = plugin.rules['object-property-newline'];

describe('object-property-newline integration boundaries', () => {
  it('exposes the exact stable metadata contract', () => {
    expect(rule.meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforce placing object properties on separate lines.',
        recommended: false,
        requiresTypeChecking: false,
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        propertiesOnNewline: 'Object properties must go on a new line.',
        propertiesOnNewlineAll:
          "Object properties must go on a new line if they aren't all on the same line.",
      },
      schema: { type: 'array' },
    });
  });

  it('reports default object members in source order with exact fixes', () => {
    const source = 'const value = { first: 1, nested: { left: 1, right: 2 }, ...rest };';
    const reports = runRule(source);
    expect(reports.map((report) => report.messageId)).toEqual([
      'propertiesOnNewline',
      'propertiesOnNewline',
      'propertiesOnNewline',
    ]);
    expect(reports.map((report) => source.slice(...report.node.range))).toEqual([
      'nested',
      'right',
      '...',
    ]);
    expect(reports.map(reportFix)).toEqual([
      { range: [25, 26], replacementText: '\n' },
      { range: [44, 45], replacementText: '\n' },
      { range: [56, 57], replacementText: '\n' },
    ]);
  });

  it('implements allowAllPropertiesOnSameLine without skipping mixed layouts', () => {
    const options = [{ allowAllPropertiesOnSameLine: true }];
    expect(runRule('const value = { first: 1, second: 2 };', options)).toEqual([]);

    const mixed = runRule('const value = { first: 1, second: 2,\nthird: 3, fourth: 4 };', options);
    expect(mixed.map((report) => report.messageId)).toEqual([
      'propertiesOnNewlineAll',
      'propertiesOnNewlineAll',
    ]);
    expect(mixed.map((report) => report.data)).toEqual([undefined, undefined]);
  });

  it('checks TypeScript type literals, interfaces, and TSX object expressions', () => {
    const source = [
      'type Payload = { id: number; name: string };',
      'interface Account { id: number; name: string }',
      'const view = <Panel data={{ first: 1, second: 2 }} />;',
    ].join('\n');
    const reports = runRule(source, [], undefined, 'fixture.tsx');
    expect(reports.map((report) => source.slice(...report.node.range))).toEqual([
      'name',
      'name',
      'second',
    ]);
  });

  it('does not apply the rule to destructuring, imports, exports, or block-like braces', () => {
    const source = [
      'const { first, second } = value;',
      "import { first, second } from 'module';",
      'export { first, second };',
      'if (condition) { first(); second(); }',
      'class Example { first = 1; second = 2 }',
      'enum Choice { First, Second }',
    ].join('\n');
    expect(runRule(source, [], undefined, 'fixture.ts')).toEqual([]);
  });

  it('keeps reports but suppresses unsafe fixes across comments', () => {
    const reports = runRule('const value = { first: 1, /* retain */ second: 2 };');
    expect(reports).toMatchObject([
      {
        messageId: 'propertiesOnNewline',
        node: { range: [39, 45] },
      },
    ]);
    expect(reports[0].suggest).toBeUndefined();
  });

  it('maps Unicode-native byte ranges and fixes to UTF-16 plugin offsets', () => {
    const source = "const 日本語 = { 最初: '一', 次: '二' };";
    const reports = runRule(source);
    const keyStart = source.indexOf('次');
    expect(reports).toMatchObject([
      {
        messageId: 'propertiesOnNewline',
        node: { range: [keyStart, keyStart + 1] },
      },
    ]);
    expect(reportFix(reports[0])).toEqual({
      range: [source.indexOf(',') + 1, keyStart],
      replacementText: '\n',
    });
  });

  it('recognizes CRLF, CR, LF, line separator, and paragraph separator', () => {
    for (const linebreak of ['\n', '\r', '\r\n', '\u2028', '\u2029']) {
      expect(
        runRule(`const value = { first: 1,${linebreak}second: 2 };`),
        JSON.stringify(linebreak),
      ).toEqual([]);
    }
  });

  it('runs through shared settings and batches with another native rule', () => {
    const source = 'const value = { first: 1, second: 2 };  \n';
    const reports = runRule('object-property-newline', source, [], {
      corsaStylistic: {
        rules: {
          'object-property-newline': [{ allowAllPropertiesOnSameLine: false }],
          'no-trailing-spaces': [],
        },
      },
    });
    expect(reports.map((report) => report.messageId)).toEqual(['propertiesOnNewline']);
  });

  it('returns byte-accurate diagnostics from the native API', () => {
    const source = "const 日本語 = { 最初: '一', 次: '二' };";
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [{ name: 'object-property-newline', options: [] }],
    });
    const start = Buffer.byteLength("const 日本語 = { 最初: '一', ");
    const fixStart = Buffer.byteLength("const 日本語 = { 最初: '一',");
    expect(diagnostics).toEqual([
      {
        ruleName: 'object-property-newline',
        messageId: 'propertiesOnNewline',
        message: 'Object properties must go on a new line.',
        range: { start, end: start + Buffer.byteLength('次') },
        suggestions: [
          {
            messageId: 'propertiesOnNewline',
            message: 'Object properties must go on a new line.',
            fixes: [
              {
                range: { start: fixStart, end: start },
                replacementText: '\n',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('reports and fixes JavaScript and TypeScript through a real Oxlint jsPlugin run', () => {
    const oxlint = findOxlintCli();
    const temporaryDirectory = mkdtempSync(join(tmpdir(), 'stylistic-object-property-'));
    try {
      const sourcePath = join(temporaryDirectory, 'sample.ts');
      const configPath = join(temporaryDirectory, 'oxlint.config.jsonc');
      writeFileSync(
        sourcePath,
        'type Payload = { id: number; name: string };\nconst value = { first: 1, second: 2 };\n',
      );
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: { 'stylistic/object-property-newline': 'error' },
        }),
      );

      const lint = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
        { encoding: 'utf8' },
      );
      expect(lint.status).toBe(1);
      expect(lint.stderr).toBe('');
      expect(JSON.parse(lint.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(object-property-newline)',
          message: 'Object properties must go on a new line.',
        },
        {
          code: 'stylistic(object-property-newline)',
          message: 'Object properties must go on a new line.',
        },
      ]);

      const fix = spawnSync(oxlint, ['-c', configPath, '--fix-suggestions', sourcePath], {
        encoding: 'utf8',
      });
      expect(fix.status).toBe(0);
      expect(readFileSync(sourcePath, 'utf8')).toBe(
        'type Payload = { id: number;\nname: string };\nconst value = { first: 1,\nsecond: 2 };\n',
      );
    } finally {
      rmSync(temporaryDirectory, { recursive: true, force: true });
    }
  });
});

function runRule(
  sourceOrRuleName,
  sourceOrOptions = [],
  optionsOrSettings,
  settingsOrFilename,
  filename,
) {
  const invokedWithRuleName = sourceOrRuleName === 'object-property-newline';
  const sourceText = invokedWithRuleName ? sourceOrOptions : sourceOrRuleName;
  const options = invokedWithRuleName ? optionsOrSettings : sourceOrOptions;
  const settings = invokedWithRuleName ? settingsOrFilename : optionsOrSettings;
  const resolvedFilename = invokedWithRuleName ? filename : settingsOrFilename;
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  rule
    .createOnce({
      options: options ?? [],
      sourceCode,
      settings,
      filename: resolvedFilename ?? 'fixture.tsx',
      report(descriptor) {
        reports.push(descriptor);
      },
    })
    .Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function reportFix(report) {
  if (!report.suggest?.[0]) {
    return null;
  }
  return report.suggest[0].fix({
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  })[0];
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
  return candidates[candidates.length - 1];
}
