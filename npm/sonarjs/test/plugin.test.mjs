import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

function runRule(ruleName, sourceText, { filename = 'sample.ts', options = [] } = {}) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[ruleName].createOnce({
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

function findOxlintCli() {
  const store = join(workspaceRoot, 'node_modules/.pnpm');
  const candidates = readdirSync(store)
    .filter((entry) => entry.startsWith('oxlint@'))
    .map((entry) => join(store, entry, 'node_modules/oxlint/bin/oxlint'))
    .filter((candidate) => existsSync(candidate))
    .sort((a, b) => a.localeCompare(b));

  if (candidates.length === 0) {
    throw new Error('Could not find oxlint CLI in node_modules/.pnpm.');
  }

  return candidates[candidates.length - 1];
}

function runOxlint(ruleName, code, filename = 'sample.ts') {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'sonarjs-plugin-'));

  try {
    const sourcePath = join(temp, filename);
    const configPath = join(temp, 'oxlint.config.jsonc');

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'sonarjs',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`sonarjs/${ruleName}`]: 'error',
        },
      }),
    );

    const result = spawnSync(
      oxlint,
      ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
      {
        encoding: 'utf8',
      },
    );
    const payload = result.stdout.trim() === '' ? { diagnostics: [] } : JSON.parse(result.stdout);

    return {
      diagnostics: payload.diagnostics ?? [],
      status: result.status,
      stderr: result.stderr,
    };
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

describe('sonarjs plugin shape', () => {
  it('exposes rules and the recommended config', () => {
    expect(plugin.meta?.name).toBe('sonarjs');
    expect(plugin.implementedSonarjsRuleNames).toEqual([
      'no-nested-template-literals',
      'no-nested-switch',
      'no-nested-conditional',
      'no-collapsible-if',
      'no-redundant-boolean',
      'comma-or-logical-or-case',
      'no-duplicate-in-composite',
      'non-existent-operator',
      'no-identical-conditions',
      'no-all-duplicated-branches',
      'no-identical-expressions',
      'arguments-usage',
      'no-labels',
      'no-delete-var',
      'constructor-for-side-effects',
      'no-empty-character-class',
      'generator-without-yield',
      'no-exclusive-tests',
      'no-built-in-override',
      'class-prototype',
      'max-switch-cases',
      'max-union-size',
      'elseif-without-else',
      'no-case-label-in-switch',
      'for-in',
      'prefer-while',
      'no-small-switch',
      'prefer-default-last',
      'no-inverted-boolean-check',
      'no-useless-catch',
      'no-redundant-optional',
      'prefer-immediate-return',
      'no-redundant-jump',
      'no-primitive-wrappers',
      'no-skipped-tests',
      'prefer-single-boolean-return',
      'no-unthrown-error',
      'no-tab',
      'fixme-tag',
      'todo-tag',
      'no-sonar-comments',
      'array-constructor',
      'no-function-declaration-in-block',
      'no-inconsistent-returns',
      'no-same-line-conditional',
      'no-nested-assignment',
      'no-nested-incdec',
      'no-useless-increment',
      'class-name',
      'max-lines',
      'nested-control-flow',
      'max-lines-per-function',
      'no-duplicate-string',
      'no-empty-group',
      'no-empty-alternatives',
      'no-regex-spaces',
      'no-control-regex',
      'single-char-in-character-classes',
    ]);
    expect(typeof plugin.rules['no-nested-template-literals']).toBe('object');
    expect(typeof plugin.rules['no-nested-switch']).toBe('object');
    expect(typeof plugin.rules['no-nested-conditional']).toBe('object');
    expect(typeof plugin.rules['no-collapsible-if']).toBe('object');
    expect(typeof plugin.rules['no-redundant-boolean']).toBe('object');
    expect(typeof plugin.rules['comma-or-logical-or-case']).toBe('object');
    expect(typeof plugin.rules['no-duplicate-in-composite']).toBe('object');
    expect(typeof plugin.rules['non-existent-operator']).toBe('object');
    expect(typeof plugin.rules['no-identical-conditions']).toBe('object');
    expect(typeof plugin.rules['no-all-duplicated-branches']).toBe('object');
    expect(typeof plugin.rules['no-identical-expressions']).toBe('object');
    expect(typeof plugin.rules['arguments-usage']).toBe('object');
    expect(typeof plugin.rules['no-labels']).toBe('object');
    expect(typeof plugin.rules['no-delete-var']).toBe('object');
    expect(typeof plugin.rules['constructor-for-side-effects']).toBe('object');
    expect(typeof plugin.rules['no-empty-character-class']).toBe('object');
    expect(typeof plugin.rules['generator-without-yield']).toBe('object');
    expect(typeof plugin.rules['no-exclusive-tests']).toBe('object');
    expect(typeof plugin.rules['no-built-in-override']).toBe('object');
    expect(typeof plugin.rules['class-prototype']).toBe('object');
    expect(typeof plugin.rules['max-switch-cases']).toBe('object');
    expect(typeof plugin.rules['max-union-size']).toBe('object');
    expect(typeof plugin.rules['elseif-without-else']).toBe('object');
    expect(typeof plugin.rules['no-case-label-in-switch']).toBe('object');
    expect(typeof plugin.rules['for-in']).toBe('object');
    expect(typeof plugin.rules['prefer-while']).toBe('object');
    expect(typeof plugin.rules['no-small-switch']).toBe('object');
    expect(typeof plugin.rules['prefer-default-last']).toBe('object');
    expect(typeof plugin.rules['no-inverted-boolean-check']).toBe('object');
    expect(typeof plugin.rules['no-useless-catch']).toBe('object');
    expect(typeof plugin.rules['no-redundant-optional']).toBe('object');
    expect(typeof plugin.rules['prefer-immediate-return']).toBe('object');
    expect(typeof plugin.rules['no-redundant-jump']).toBe('object');
    expect(typeof plugin.rules['no-primitive-wrappers']).toBe('object');
    expect(typeof plugin.rules['no-skipped-tests']).toBe('object');
    expect(typeof plugin.rules['prefer-single-boolean-return']).toBe('object');
    expect(typeof plugin.rules['no-unthrown-error']).toBe('object');
    expect(typeof plugin.rules['no-tab']).toBe('object');
    expect(typeof plugin.rules['fixme-tag']).toBe('object');
    expect(typeof plugin.rules['todo-tag']).toBe('object');
    expect(typeof plugin.rules['no-sonar-comments']).toBe('object');
    expect(typeof plugin.rules['array-constructor']).toBe('object');
    expect(typeof plugin.rules['no-function-declaration-in-block']).toBe('object');
    expect(typeof plugin.rules['no-inconsistent-returns']).toBe('object');
    expect(typeof plugin.rules['no-same-line-conditional']).toBe('object');
    expect(typeof plugin.rules['no-nested-assignment']).toBe('object');
    expect(typeof plugin.rules['no-nested-incdec']).toBe('object');
    expect(typeof plugin.rules['no-useless-increment']).toBe('object');
    expect(typeof plugin.rules['class-name']).toBe('object');
    expect(typeof plugin.rules['max-lines']).toBe('object');
    expect(typeof plugin.rules['nested-control-flow']).toBe('object');
    expect(typeof plugin.rules['max-lines-per-function']).toBe('object');
    expect(typeof plugin.rules['no-duplicate-string']).toBe('object');
    expect(typeof plugin.rules['no-empty-group']).toBe('object');
    expect(typeof plugin.rules['no-empty-alternatives']).toBe('object');
    expect(typeof plugin.rules['no-regex-spaces']).toBe('object');
    expect(typeof plugin.rules['no-control-regex']).toBe('object');
    expect(typeof plugin.rules['single-char-in-character-classes']).toBe('object');
    expect(Object.keys(plugin.configs)).toEqual(['recommended']);
    expect(plugin.configs.recommended.rules['sonarjs/no-nested-template-literals']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-nested-switch']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-nested-conditional']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-collapsible-if']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-redundant-boolean']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/comma-or-logical-or-case']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-duplicate-in-composite']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/non-existent-operator']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-identical-conditions']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-all-duplicated-branches']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-identical-expressions']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/arguments-usage']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-labels']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-delete-var']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/constructor-for-side-effects']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-empty-character-class']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/generator-without-yield']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-exclusive-tests']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-built-in-override']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/class-prototype']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/max-switch-cases']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/max-union-size']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/elseif-without-else']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-case-label-in-switch']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/for-in']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/prefer-while']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-small-switch']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/prefer-default-last']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-inverted-boolean-check']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-useless-catch']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-redundant-optional']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/prefer-immediate-return']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-redundant-jump']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-primitive-wrappers']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-skipped-tests']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/prefer-single-boolean-return']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-unthrown-error']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-tab']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/fixme-tag']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/todo-tag']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-sonar-comments']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/array-constructor']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-function-declaration-in-block']).toBe(
      'error',
    );
    expect(plugin.configs.recommended.rules['sonarjs/no-inconsistent-returns']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-same-line-conditional']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-nested-assignment']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-nested-incdec']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-useless-increment']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/class-name']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/max-lines']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/nested-control-flow']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/max-lines-per-function']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-duplicate-string']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-empty-group']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-empty-alternatives']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-regex-spaces']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-control-regex']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/single-char-in-character-classes']).toBe(
      'error',
    );
  });
});

describe('sonarjs rules through direct adapter harness', () => {
  it('reports nested template literals', () => {
    const reports = runRule('no-nested-template-literals', 'const x = `outer ${`inner`} end`;');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('nestedTemplateLiteral');
  });

  it('does not report flat template literals', () => {
    const reports = runRule('no-nested-template-literals', 'const x = `value ${y}`;');
    expect(reports).toHaveLength(0);
  });

  it('reports nested switch statements', () => {
    const reports = runRule(
      'no-nested-switch',
      'switch (a) {\n  case 1:\n    switch (b) {\n      default:\n        break;\n    }\n}',
    );
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('nestedSwitch');
  });

  it('reports a nested conditional expression', () => {
    const reports = runRule('no-nested-conditional', 'const x = a ? b : (c ? d : e);');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('nestedConditional');
  });

  it('reports a collapsible if through the adapter', () => {
    const reports = runRule('no-collapsible-if', 'if (a) { if (b) {} }');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('collapsibleIf');
  });

  it('reports a redundant boolean literal through the adapter', () => {
    const reports = runRule('no-redundant-boolean', 'x === true');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('redundantBoolean');
  });

  it('reports a logical-OR case label through the adapter', () => {
    const reports = runRule('comma-or-logical-or-case', 'switch (x) { case 1 || 2: break; }');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('commaOrLogicalOrInCase');
  });

  it('reports a duplicate type in a union through the adapter', () => {
    const reports = runRule('no-duplicate-in-composite', 'type T = A | B | A;', {
      filename: 'sample.ts',
    });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('duplicateType');
  });

  it('reports non-existent-operator through the adapter', () => {
    const source = 'let x = 0; x =- 1;';
    const reports = runRule('non-existent-operator', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('nonExistentOperator');
  });

  it('reports no-identical-conditions through the adapter', () => {
    const source = 'if (a) {} else if (b) {} else if (a) {}';
    const reports = runRule('no-identical-conditions', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('identicalConditions');
  });

  it('reports no-all-duplicated-branches through the adapter', () => {
    const source = 'if (a) { f(); } else { f(); }';
    const reports = runRule('no-all-duplicated-branches', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('allDuplicatedBranches');
  });

  it('reports no-identical-expressions through the adapter', () => {
    const source = 'a === a';
    const reports = runRule('no-identical-expressions', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('identicalExpressions');
  });

  it('reports arguments-usage through the adapter', () => {
    const source = 'function f() { return arguments[0]; }';
    const reports = runRule('arguments-usage', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('argumentsUsage');
  });

  it('reports no-labels through the adapter', () => {
    const source = 'loop: for (;;) { break loop; }';
    const reports = runRule('no-labels', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noLabels');
  });

  it('reports no-delete-var through the adapter', () => {
    const source = 'delete x;';
    const reports = runRule('no-delete-var', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noDeleteVar');
  });

  it('reports constructor-for-side-effects through the adapter', () => {
    const source = 'new Foo();';
    const reports = runRule('constructor-for-side-effects', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('constructorForSideEffects');
  });

  it('reports no-empty-character-class through the adapter', () => {
    const source = 'const r = /a[]b/;';
    const reports = runRule('no-empty-character-class', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyCharacterClass');
  });

  it('reports generator-without-yield through the adapter', () => {
    const source = 'function* g() { return 1; }';
    const reports = runRule('generator-without-yield', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('generatorWithoutYield');
  });

  it('reports no-exclusive-tests for describe.only through the adapter', () => {
    const source = "describe.only('x', () => {});";
    const reports = runRule('no-exclusive-tests', source, { filename: 'sample.js' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noExclusiveTests');
  });

  it('reports no-built-in-override for a let declaration that shadows Object through the adapter', () => {
    const source = 'let Object = 1;';
    const reports = runRule('no-built-in-override', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noBuiltInOverride');
  });

  it('reports class-prototype for Foo.prototype.bar = function () {} through the adapter', () => {
    const source = 'Foo.prototype.bar = function () {};';
    const reports = runRule('class-prototype', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('classPrototype');
  });

  it('reports max-switch-cases for a switch with 31 cases through the adapter', () => {
    const big =
      'switch (x) {' + Array.from({ length: 31 }, (_, i) => `case ${i}: break;`).join('') + '}';
    const reports = runRule('max-switch-cases', big);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('maxSwitchCases');
  });

  it('reports max-union-size for a union type with 4 members through the adapter', () => {
    const source = 'type T = A | B | C | D;';
    const reports = runRule('max-union-size', source, { filename: 'sample.ts' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('maxUnionSize');
  });

  it('reports for-in when body is a block with no if statement through the adapter', () => {
    const source = 'for (const k in o) { doStuff(k); }';
    const reports = runRule('for-in', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('forIn');
  });

  it('reports no-tab for a line with a leading tab through the adapter', () => {
    const source = '\tconst x = 1;';
    const reports = runRule('no-tab', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noTab');
  });

  it('reports no-duplicate-string through the adapter with custom threshold', () => {
    // "hello wrld" = 10 chars, has a space → qualifies; appears twice; threshold 2
    const source = 'const a = "hello wrld"; const b = "hello wrld";';
    const reports = runRule('no-duplicate-string', source, { options: [{ threshold: 2 }] });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('duplicateString');
  });
});

describe('sonarjs rules through oxlint jsPlugins', () => {
  it('reports no-nested-template-literals through the CLI', () => {
    const result = runOxlint('no-nested-template-literals', 'const x = `outer ${`inner`} end`;');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-nested-template-literals)');
  });

  it('reports no-nested-switch through the CLI', () => {
    const result = runOxlint(
      'no-nested-switch',
      'switch (a) {\n  case 1:\n    switch (b) {\n      default:\n        break;\n    }\n}',
    );

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-nested-switch)');
  });

  it('reports no-nested-conditional through the CLI', () => {
    const result = runOxlint('no-nested-conditional', 'const x = a ? b : (c ? d : e);');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-nested-conditional)');
  });

  it('reports no-collapsible-if through the CLI', () => {
    const result = runOxlint('no-collapsible-if', 'if (a) { if (b) {} }');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-collapsible-if)');
  });

  it('reports no-redundant-boolean through the CLI', () => {
    const result = runOxlint('no-redundant-boolean', 'x === true');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-redundant-boolean)');
  });

  it('reports comma-or-logical-or-case through the CLI', () => {
    const result = runOxlint('comma-or-logical-or-case', 'switch (x) { case 1 || 2: break; }');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(comma-or-logical-or-case)');
  });

  it('reports no-duplicate-in-composite through the CLI', () => {
    const result = runOxlint('no-duplicate-in-composite', 'type T = A | B | A;', 'sample.ts');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-duplicate-in-composite)');
  });

  it('reports non-existent-operator through the CLI', () => {
    const source = 'let x = 0; x =- 1;';
    const result = runOxlint('non-existent-operator', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(non-existent-operator)');
  });

  it('reports no-identical-conditions through the CLI', () => {
    const source = 'if (a) {} else if (b) {} else if (a) {}';
    const result = runOxlint('no-identical-conditions', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-identical-conditions)');
  });

  it('reports no-all-duplicated-branches through the CLI', () => {
    const source = 'if (a) { f(); } else { f(); }';
    const result = runOxlint('no-all-duplicated-branches', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-all-duplicated-branches)');
  });

  it('reports no-identical-expressions through the CLI', () => {
    const source = 'a === a';
    const result = runOxlint('no-identical-expressions', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-identical-expressions)');
  });

  it('reports arguments-usage through the CLI', () => {
    const source = 'function f() { return arguments[0]; }';
    const result = runOxlint('arguments-usage', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(arguments-usage)');
  });

  it('reports no-labels through the CLI', () => {
    const source = 'loop: for (;;) { break loop; }';
    const result = runOxlint('no-labels', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-labels)');
  });

  it('reports no-delete-var through the CLI', () => {
    const source = 'delete x;';
    const result = runOxlint('no-delete-var', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-delete-var)');
  });

  it('reports constructor-for-side-effects through the CLI', () => {
    const source = 'new Foo();';
    const result = runOxlint('constructor-for-side-effects', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(constructor-for-side-effects)');
  });

  it('reports no-empty-character-class through the CLI', () => {
    const source = 'const r = /a[]b/;';
    const result = runOxlint('no-empty-character-class', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-empty-character-class)');
  });

  it('reports generator-without-yield through the CLI', () => {
    const source = 'function* g() { return 1; }';
    const result = runOxlint('generator-without-yield', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(generator-without-yield)');
  });

  it('reports no-exclusive-tests through the CLI', () => {
    const source = "describe.only('x', () => {});";
    const result = runOxlint('no-exclusive-tests', source, 'sample.js');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-exclusive-tests)');
  });

  it('reports no-built-in-override through the CLI', () => {
    const source = 'let Object = 1;';
    const result = runOxlint('no-built-in-override', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-built-in-override)');
  });

  it('reports class-prototype through the CLI', () => {
    const source = 'Foo.prototype.bar = function () {};';
    const result = runOxlint('class-prototype', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(class-prototype)');
  });

  it('reports max-switch-cases through the CLI', () => {
    const big =
      'switch (x) {' + Array.from({ length: 31 }, (_, i) => `case ${i}: break;`).join('') + '}';
    const result = runOxlint('max-switch-cases', big);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(max-switch-cases)');
  });

  it('reports max-union-size through the CLI', () => {
    const source = 'type T = A | B | C | D;';
    const result = runOxlint('max-union-size', source, 'sample.ts');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(max-union-size)');
  });

  it('reports elseif-without-else through the adapter', () => {
    const source = 'if (a) {} else if (b) {}';
    const reports = runRule('elseif-without-else', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('elseifWithoutElse');
  });

  it('reports elseif-without-else through the CLI', () => {
    const source = 'if (a) {} else if (b) {}';
    const result = runOxlint('elseif-without-else', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(elseif-without-else)');
  });

  it('reports no-case-label-in-switch through the adapter', () => {
    const source = 'switch (x) { case 1: foo(); lbl: bar(); break; }';
    const reports = runRule('no-case-label-in-switch', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('caseLabelInSwitch');
  });

  it('reports no-case-label-in-switch through the CLI', () => {
    const source = 'switch (x) { case 1: foo(); lbl: bar(); break; }';
    const result = runOxlint('no-case-label-in-switch', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-case-label-in-switch)');
  });

  it('reports for-in through the CLI', () => {
    const source = 'for (const k in o) { doStuff(k); }';
    const result = runOxlint('for-in', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(for-in)');
  });

  it('reports prefer-while through the adapter', () => {
    const source = 'for (; i < 10;) { i++; }';
    const reports = runRule('prefer-while', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferWhile');
  });

  it('reports prefer-while through the CLI', () => {
    const source = 'for (;;) {}';
    const result = runOxlint('prefer-while', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(prefer-while)');
  });

  it('reports no-small-switch for a switch with one case through the adapter', () => {
    const source = 'switch (x) { case 1: break; }';
    const reports = runRule('no-small-switch', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('smallSwitch');
  });

  it('reports no-small-switch through the CLI', () => {
    const source = 'switch (x) { case 1: break; }';
    const result = runOxlint('no-small-switch', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-small-switch)');
  });

  it('reports prefer-default-last when default is not the last clause through the adapter', () => {
    const source = 'switch (x) { default: break; case 1: break; }';
    const reports = runRule('prefer-default-last', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('defaultLast');
  });

  it('reports prefer-default-last through the CLI', () => {
    const source = 'switch (x) { default: break; case 1: break; }';
    const result = runOxlint('prefer-default-last', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(prefer-default-last)');
  });

  it('reports no-inverted-boolean-check for !(a === b) through the adapter', () => {
    const source = 'const r = !(a === b);';
    const reports = runRule('no-inverted-boolean-check', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('invertedBooleanCheck');
  });

  it('reports no-inverted-boolean-check through the CLI', () => {
    const source = 'const r = !(a === b);';
    const result = runOxlint('no-inverted-boolean-check', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-inverted-boolean-check)');
  });

  it('reports no-useless-catch for catch that only rethrows through the adapter', () => {
    const source = 'try { f(); } catch (e) { throw e; }';
    const reports = runRule('no-useless-catch', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('uselessCatch');
  });

  it('reports no-useless-catch through the CLI', () => {
    const source = 'try { f(); } catch (e) { throw e; }';
    const result = runOxlint('no-useless-catch', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-useless-catch)');
  });

  it('reports no-redundant-optional through the adapter', () => {
    const source = 'interface I { a?: string | undefined; }';
    const reports = runRule('no-redundant-optional', source, { filename: 'sample.ts' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('redundantOptional');
  });

  it('reports no-redundant-optional through the CLI', () => {
    const source = 'interface I { a?: string | undefined; }';
    const result = runOxlint('no-redundant-optional', source, 'sample.ts');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-redundant-optional)');
  });

  it('reports prefer-immediate-return through the adapter', () => {
    const source = 'function f() { const x = compute(); return x; }';
    const reports = runRule('prefer-immediate-return', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferImmediateReturn');
  });

  it('reports prefer-immediate-return through the CLI', () => {
    const source = 'function f() { const x = compute(); return x; }';
    const result = runOxlint('prefer-immediate-return', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(prefer-immediate-return)');
  });

  it('reports no-redundant-jump for trailing continue through the adapter', () => {
    const source = 'for (;;) { foo(); continue; }';
    const reports = runRule('no-redundant-jump', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('redundantJump');
  });

  it('reports no-redundant-jump through the CLI', () => {
    const source = 'function f() { foo(); return; }';
    const result = runOxlint('no-redundant-jump', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-redundant-jump)');
  });

  it('reports no-primitive-wrappers for new Number(1) through the adapter', () => {
    const source = 'const n = new Number(1);';
    const reports = runRule('no-primitive-wrappers', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('primitiveWrapper');
  });

  it('reports no-primitive-wrappers through the CLI', () => {
    const source = 'const n = new Number(1);';
    const result = runOxlint('no-primitive-wrappers', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-primitive-wrappers)');
  });

  it('reports no-skipped-tests for describe.skip through the adapter', () => {
    const source = "describe.skip('x', () => {});";
    const reports = runRule('no-skipped-tests', source, { filename: 'sample.js' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('skippedTest');
  });

  it('reports no-skipped-tests through the CLI', () => {
    const source = "xit('x', () => {});";
    const result = runOxlint('no-skipped-tests', source, 'sample.js');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-skipped-tests)');
  });

  it('reports prefer-single-boolean-return through the adapter', () => {
    const source = 'function f() { if (c) { return true; } else { return false; } }';
    const reports = runRule('prefer-single-boolean-return', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferSingleBooleanReturn');
  });

  it('reports prefer-single-boolean-return through the CLI', () => {
    const source = 'function f() { if (c) { return true; } else { return false; } }';
    const result = runOxlint('prefer-single-boolean-return', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(prefer-single-boolean-return)');
  });

  it('reports no-unthrown-error for new Error as a bare statement through the adapter', () => {
    const source = "new Error('boom');";
    const reports = runRule('no-unthrown-error', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('unthrownError');
  });

  it('reports no-unthrown-error through the CLI', () => {
    const source = "new TypeError('x');";
    const result = runOxlint('no-unthrown-error', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-unthrown-error)');
  });

  it('reports no-tab through the CLI', () => {
    const source = '\tconst x = 1;';
    const result = runOxlint('no-tab', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-tab)');
  });

  it('reports fixme-tag for a line comment containing FIXME through the adapter', () => {
    const source = '// FIXME do x';
    const reports = runRule('fixme-tag', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('fixmeTag');
  });

  it('reports fixme-tag through the CLI', () => {
    const source = '// FIXME do x';
    const result = runOxlint('fixme-tag', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(fixme-tag)');
  });

  it('reports todo-tag for a line comment containing TODO through the adapter', () => {
    const source = '// TODO do x';
    const reports = runRule('todo-tag', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('todoTag');
  });

  it('reports todo-tag through the CLI', () => {
    const source = '// TODO do x';
    const result = runOxlint('todo-tag', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(todo-tag)');
  });

  it('reports no-sonar-comments for a NOSONAR comment through the adapter', () => {
    const source = '// NOSONAR suppress this';
    const reports = runRule('no-sonar-comments', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noSonarComments');
  });

  it('reports no-sonar-comments through the CLI', () => {
    const source = '// NOSONAR suppress this';
    const result = runOxlint('no-sonar-comments', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-sonar-comments)');
  });

  it('reports array-constructor for a multi-argument call through the adapter', () => {
    const source = 'const a = Array(1, 2, 3);';
    const reports = runRule('array-constructor', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('arrayConstructor');
  });

  it('reports array-constructor through the CLI', () => {
    const source = 'const a = new Array(1, 2, 3);';
    const result = runOxlint('array-constructor', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(array-constructor)');
  });

  it('reports no-function-declaration-in-block through the adapter', () => {
    const source = 'if (cond) {\n  function f() {}\n}';
    const reports = runRule('no-function-declaration-in-block', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noFunctionDeclarationInBlock');
  });

  it('reports no-function-declaration-in-block through the CLI', () => {
    const source = 'if (cond) {\n  function f() {}\n}';
    const result = runOxlint('no-function-declaration-in-block', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-function-declaration-in-block)');
  });

  it('reports no-inconsistent-returns through the adapter', () => {
    const source = 'function f(x) {\n  if (!x) return;\n  return x.value;\n}';
    const reports = runRule('no-inconsistent-returns', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('inconsistentReturns');
  });

  it('reports no-inconsistent-returns through the CLI', () => {
    const source = 'function f(x) {\n  if (!x) return;\n  return x.value;\n}';
    const result = runOxlint('no-inconsistent-returns', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-inconsistent-returns)');
  });

  it('reports no-same-line-conditional through the adapter', () => {
    const source = 'if (a) {\n  doA();\n} if (b) {\n  doB();\n}';
    const reports = runRule('no-same-line-conditional', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('sameLineConditional');
  });

  it('reports no-same-line-conditional through the CLI', () => {
    const source = 'if (a) {\n  doA();\n} if (b) {\n  doB();\n}';
    const result = runOxlint('no-same-line-conditional', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-same-line-conditional)');
  });

  it('reports no-nested-assignment through the adapter', () => {
    const source = 'if (x = compute()) {\n  use(x);\n}';
    const reports = runRule('no-nested-assignment', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('nestedAssignment');
  });

  it('reports no-nested-assignment through the CLI', () => {
    const source = 'if (x = compute()) {\n  use(x);\n}';
    const result = runOxlint('no-nested-assignment', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-nested-assignment)');
  });

  it('reports no-nested-incdec through the adapter', () => {
    const source = 'foo(i++);';
    const reports = runRule('no-nested-incdec', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('nestedIncDec');
  });

  it('reports no-nested-incdec through the CLI', () => {
    const source = 'foo(i++);';
    const result = runOxlint('no-nested-incdec', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-nested-incdec)');
  });

  it('reports no-useless-increment through the adapter', () => {
    const source = 'i = i++;';
    const reports = runRule('no-useless-increment', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('uselessIncrement');
  });

  it('reports no-useless-increment through the CLI', () => {
    const source = 'i = i++;';
    const result = runOxlint('no-useless-increment', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-useless-increment)');
  });

  it('reports class-name through the adapter', () => {
    const source = 'class myClass {}';
    const reports = runRule('class-name', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('className');
  });

  it('reports class-name through the CLI', () => {
    const source = 'class myClass {}';
    const result = runOxlint('class-name', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(class-name)');
  });

  it('honors the max-switch-cases "maximum" option', () => {
    const source = 'switch (x) { case 1: break; case 2: break; case 3: break; }';
    expect(runRule('max-switch-cases', source, { options: [{ maximum: 2 }] })).toHaveLength(1);
    expect(runRule('max-switch-cases', source, { options: [{ maximum: 3 }] })).toHaveLength(0);
  });

  it('uses the default max-switch-cases threshold when no option is given', () => {
    const source = 'switch (x) { case 1: break; case 2: break; case 3: break; }';
    expect(runRule('max-switch-cases', source)).toHaveLength(0);
  });

  it('honors the max-union-size "threshold" option', () => {
    const source = 'type T = A | B | C;';
    expect(runRule('max-union-size', source, { options: [{ threshold: 2 }] })).toHaveLength(1);
    expect(runRule('max-union-size', source, { options: [{ threshold: 3 }] })).toHaveLength(0);
  });

  it('exposes the configurable options in each rule schema', () => {
    expect(plugin.rules['max-switch-cases'].meta.schema).toEqual([
      { type: 'object', properties: { maximum: { type: 'integer' } }, additionalProperties: false },
    ]);
    expect(plugin.rules['max-union-size'].meta.schema).toEqual([
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ]);
    expect(plugin.rules['no-collapsible-if'].meta.schema).toEqual([]);
  });

  it('honors the max-lines "maximum" option through the adapter', () => {
    const source = 'const a = 1;\nconst b = 2;\nconst c = 3;';
    expect(runRule('max-lines', source, { options: [{ maximum: 2 }] })).toHaveLength(1);
    expect(runRule('max-lines', source, { options: [{ maximum: 3 }] })).toHaveLength(0);
  });

  it('reports max-lines-per-function when function exceeds threshold', () => {
    const source =
      'function f() {\n  const a = 1;\n  const b = 2;\n  const c = 3;\n  return a + b + c;\n}';
    const reports = runRule('max-lines-per-function', source, { options: [{ maximum: 3 }] });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('maxLinesPerFunction');
  });

  it('does not report max-lines-per-function when function is within threshold', () => {
    // 5 code lines (signature + 3 body + closing brace), exactly at the threshold
    const source = 'function f() {\n  const a = 1;\n  const b = 2;\n  return a + b;\n}';
    const reports = runRule('max-lines-per-function', source, { options: [{ maximum: 5 }] });
    expect(reports).toHaveLength(0);
  });

  it('reports max-lines through the CLI', () => {
    const source = 'const a = 1;\nconst b = 2;\nconst c = 3;';
    const result = runOxlint('max-lines', source);
    // default threshold is 1000; three code lines must NOT be flagged
    expect(result.status).toBe(0);
    expect(result.diagnostics).toHaveLength(0);
  });

  it('honors the nested-control-flow "maximumNestingLevel" option through the adapter', () => {
    const source = 'if (a) { for (let i = 0; i < 10; i++) { while (b) {} } }';
    expect(
      runRule('nested-control-flow', source, { options: [{ maximumNestingLevel: 2 }] }),
    ).toHaveLength(1);
    expect(
      runRule('nested-control-flow', source, { options: [{ maximumNestingLevel: 3 }] }),
    ).toHaveLength(0);
  });

  it('reports nested-control-flow through the CLI', () => {
    const source = 'if (a) { for (let i = 0; i < 10; i++) { while (b) { if (c) {} } } }';
    const result = runOxlint('nested-control-flow', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(nested-control-flow)');
  });

  it('does not report max-lines-per-function for a short function (default 200)', () => {
    const source = 'function f() { return 1; }';
    const { diagnostics } = runOxlint('max-lines-per-function', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-duplicate-string through the CLI', () => {
    // "hello wrld" = 10 chars, has a space → qualifies; appears 3× (default threshold 3)
    const src = 'const a = "hello wrld"; const b = "hello wrld"; const c = "hello wrld";';
    const result = runOxlint('no-duplicate-string', src);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-duplicate-string)');
  });

  it('reports no-empty-group for an empty non-capturing group through the adapter', () => {
    const source = 'const r = /(?:)/;';
    const reports = runRule('no-empty-group', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyGroup');
  });

  it('reports no-empty-group through the CLI', () => {
    const result = runOxlint('no-empty-group', 'const r = /(?:)/;');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-empty-group)');
  });

  it('reports no-empty-alternatives for a trailing empty alternative through the adapter', () => {
    const source = 'const r = /a|/;';
    const reports = runRule('no-empty-alternatives', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyAlternative');
  });

  it('reports no-empty-alternatives through the CLI', () => {
    const result = runOxlint('no-empty-alternatives', 'const r = /a|/;');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-empty-alternatives)');
  });

  it('reports no-regex-spaces for two consecutive spaces through the adapter', () => {
    const source = 'const r = /a  b/;';
    const reports = runRule('no-regex-spaces', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('multipleSpaces');
  });

  it('reports no-regex-spaces through the CLI', () => {
    const result = runOxlint('no-regex-spaces', 'const r = /a  b/;');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-regex-spaces)');
  });

  it('reports no-control-regex for a hex escape control character through the adapter', () => {
    const source = 'const r = /\\x1f/;';
    const reports = runRule('no-control-regex', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('controlCharacter');
  });

  it('reports no-control-regex through the CLI', () => {
    const result = runOxlint('no-control-regex', 'const r = /\\x1f/;');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-control-regex)');
  });

  it('reports single-char-in-character-classes through the adapter', () => {
    const source = 'const r = /[a]/;';
    const reports = runRule('single-char-in-character-classes', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('singleCharInCharacterClass');
  });

  it('reports single-char-in-character-classes through the CLI', () => {
    const result = runOxlint('single-char-in-character-classes', 'const r = /[a]/;');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(single-char-in-character-classes)');
  });
});
