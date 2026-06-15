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
      'label-position',
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
      'function-name',
      'max-lines',
      'nested-control-flow',
      'max-lines-per-function',
      'no-duplicate-string',
      'no-empty-group',
      'no-empty-alternatives',
      'no-regex-spaces',
      'no-control-regex',
      'single-char-in-character-classes',
      'duplicates-in-character-class',
      'anchor-precedence',
      'cyclomatic-complexity',
      'no-collection-size-mischeck',
      'index-of-compare-to-positive-number',
      'no-nested-functions',
      'too-many-break-or-continue-in-loop',
      'code-eval',
      'void-use',
      'prefer-promise-shorthand',
      'pseudo-random',
      'hashing',
      'no-clear-text-protocols',
      'no-hardcoded-ip',
      'no-global-this',
      'single-character-alternation',
      'empty-string-repetition',
      'no-misleading-array-reverse',
      'no-alphabetical-sort',
      'no-for-in-iterable',
      'no-associative-arrays',
      'bitwise-operators',
      'no-same-argument-assert',
      'inverted-assertion-arguments',
      'for-loop-increment-sign',
      'no-equals-in-for-termination',
      'reduce-initial-value',
      'no-parameter-reassignment',
      'array-callback-without-return',
      'declarations-in-global-scope',
      'no-wildcard-import',
      'updated-loop-counter',
      'misplaced-loop-counter',
      'no-array-delete',
      'no-literal-call',
      'shorthand-property-grouping',
      'process-argv',
      'standard-input',
      'no-code-after-done',
      'function-inside-loop',
      'no-useless-intersection',
      'use-type-alias',
      'public-static-readonly',
      'call-argument-line',
      'prefer-object-literal',
      'no-undefined-argument',
      'no-identical-functions',
      'no-in-misuse',
      'no-require-or-define',
      'no-invalid-regexp',
      'no-invariant-returns',
      'no-extra-arguments',
      'link-with-target-blank',
      'no-weak-cipher',
      'no-hardcoded-passwords',
      'no-ignored-exceptions',
      'no-unused-function-argument',
      'object-alt-content',
      'no-use-of-empty-return-value',
      'no-duplicated-branches',
      'block-scoped-var',
      'no-variable-usage-before-declaration',
      'arguments-order',
      'updated-const-var',
      'unicode-aware-regex',
      'no-undefined-assignment',
      'no-empty-after-reluctant',
      'no-ignored-return',
      'file-name-differ-from-class',
      'no-unenclosed-multiline-block',
      'inconsistent-function-call',
      'new-operator-misuse',
      'no-empty-test-file',
      'deprecation',
      'cognitive-complexity',
      'expression-complexity',
      'prefer-regexp-exec',
      'no-fallthrough',
      'no-commented-code',
      'no-incomplete-assertions',
      'destructuring-assignment-syntax',
      'no-element-overwrite',
      'no-redundant-assignments',
      'no-unused-collection',
      'no-empty-collection',
      'no-redundant-parentheses',
      'bool-param-default',
      'post-message',
      'in-operator-type-error',
      'different-types-comparison',
      'operation-returning-nan',
      'production-debug',
      'no-hardcoded-secrets',
      'concise-regex',
      'no-misleading-character-class',
      'slow-regex',
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
    expect(typeof plugin.rules['label-position']).toBe('object');
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
    expect(typeof plugin.rules['function-name']).toBe('object');
    expect(typeof plugin.rules['max-lines']).toBe('object');
    expect(typeof plugin.rules['nested-control-flow']).toBe('object');
    expect(typeof plugin.rules['max-lines-per-function']).toBe('object');
    expect(typeof plugin.rules['no-duplicate-string']).toBe('object');
    expect(typeof plugin.rules['no-empty-group']).toBe('object');
    expect(typeof plugin.rules['no-empty-alternatives']).toBe('object');
    expect(typeof plugin.rules['no-regex-spaces']).toBe('object');
    expect(typeof plugin.rules['no-control-regex']).toBe('object');
    expect(typeof plugin.rules['single-char-in-character-classes']).toBe('object');
    expect(typeof plugin.rules['duplicates-in-character-class']).toBe('object');
    expect(typeof plugin.rules['anchor-precedence']).toBe('object');
    expect(typeof plugin.rules['cyclomatic-complexity']).toBe('object');
    expect(typeof plugin.rules['no-collection-size-mischeck']).toBe('object');
    expect(typeof plugin.rules['index-of-compare-to-positive-number']).toBe('object');
    expect(typeof plugin.rules['no-nested-functions']).toBe('object');
    expect(typeof plugin.rules['too-many-break-or-continue-in-loop']).toBe('object');
    expect(typeof plugin.rules['code-eval']).toBe('object');
    expect(typeof plugin.rules['prefer-promise-shorthand']).toBe('object');
    expect(typeof plugin.rules['pseudo-random']).toBe('object');
    expect(typeof plugin.rules['hashing']).toBe('object');
    expect(typeof plugin.rules['no-clear-text-protocols']).toBe('object');
    expect(typeof plugin.rules['no-hardcoded-ip']).toBe('object');
    expect(typeof plugin.rules['no-global-this']).toBe('object');
    expect(typeof plugin.rules['single-character-alternation']).toBe('object');
    expect(typeof plugin.rules['empty-string-repetition']).toBe('object');
    expect(typeof plugin.rules['no-misleading-array-reverse']).toBe('object');
    expect(typeof plugin.rules['no-alphabetical-sort']).toBe('object');
    expect(typeof plugin.rules['no-for-in-iterable']).toBe('object');
    expect(typeof plugin.rules['no-associative-arrays']).toBe('object');
    expect(typeof plugin.rules['bitwise-operators']).toBe('object');
    expect(typeof plugin.rules['no-same-argument-assert']).toBe('object');
    expect(typeof plugin.rules['inverted-assertion-arguments']).toBe('object');
    expect(typeof plugin.rules['for-loop-increment-sign']).toBe('object');
    expect(typeof plugin.rules['no-equals-in-for-termination']).toBe('object');
    expect(typeof plugin.rules['reduce-initial-value']).toBe('object');
    expect(typeof plugin.rules['no-parameter-reassignment']).toBe('object');
    expect(typeof plugin.rules['array-callback-without-return']).toBe('object');
    expect(typeof plugin.rules['declarations-in-global-scope']).toBe('object');
    expect(typeof plugin.rules['no-wildcard-import']).toBe('object');
    expect(typeof plugin.rules['updated-loop-counter']).toBe('object');
    expect(typeof plugin.rules['misplaced-loop-counter']).toBe('object');
    expect(typeof plugin.rules['no-array-delete']).toBe('object');
    expect(typeof plugin.rules['no-literal-call']).toBe('object');
    expect(typeof plugin.rules['shorthand-property-grouping']).toBe('object');
    expect(typeof plugin.rules['process-argv']).toBe('object');
    expect(typeof plugin.rules['standard-input']).toBe('object');
    expect(typeof plugin.rules['no-code-after-done']).toBe('object');
    expect(typeof plugin.rules['function-inside-loop']).toBe('object');
    expect(typeof plugin.rules['no-useless-intersection']).toBe('object');
    expect(typeof plugin.rules['use-type-alias']).toBe('object');
    expect(typeof plugin.rules['public-static-readonly']).toBe('object');
    expect(typeof plugin.rules['call-argument-line']).toBe('object');
    expect(typeof plugin.rules['prefer-object-literal']).toBe('object');
    expect(typeof plugin.rules['no-undefined-argument']).toBe('object');
    expect(typeof plugin.rules['no-identical-functions']).toBe('object');
    expect(typeof plugin.rules['no-in-misuse']).toBe('object');
    expect(typeof plugin.rules['no-require-or-define']).toBe('object');
    expect(typeof plugin.rules['no-invalid-regexp']).toBe('object');
    expect(typeof plugin.rules['no-invariant-returns']).toBe('object');
    expect(typeof plugin.rules['no-extra-arguments']).toBe('object');
    expect(typeof plugin.rules['link-with-target-blank']).toBe('object');
    expect(typeof plugin.rules['no-weak-cipher']).toBe('object');
    expect(typeof plugin.rules['no-hardcoded-passwords']).toBe('object');
    expect(typeof plugin.rules['no-ignored-exceptions']).toBe('object');
    expect(typeof plugin.rules['no-unused-function-argument']).toBe('object');
    expect(typeof plugin.rules['object-alt-content']).toBe('object');
    expect(typeof plugin.rules['no-use-of-empty-return-value']).toBe('object');
    expect(typeof plugin.rules['no-duplicated-branches']).toBe('object');
    expect(typeof plugin.rules['block-scoped-var']).toBe('object');
    expect(typeof plugin.rules['no-variable-usage-before-declaration']).toBe('object');
    expect(typeof plugin.rules['arguments-order']).toBe('object');
    expect(typeof plugin.rules['updated-const-var']).toBe('object');
    expect(typeof plugin.rules['unicode-aware-regex']).toBe('object');
    expect(typeof plugin.rules['no-undefined-assignment']).toBe('object');
    expect(typeof plugin.rules['no-empty-after-reluctant']).toBe('object');
    expect(typeof plugin.rules['no-ignored-return']).toBe('object');
    expect(typeof plugin.rules['file-name-differ-from-class']).toBe('object');
    expect(typeof plugin.rules['no-unenclosed-multiline-block']).toBe('object');
    expect(typeof plugin.rules['inconsistent-function-call']).toBe('object');
    expect(typeof plugin.rules['new-operator-misuse']).toBe('object');
    expect(typeof plugin.rules['no-empty-test-file']).toBe('object');
    expect(typeof plugin.rules['deprecation']).toBe('object');
    expect(typeof plugin.rules['cognitive-complexity']).toBe('object');
    expect(typeof plugin.rules['expression-complexity']).toBe('object');
    expect(typeof plugin.rules['no-fallthrough']).toBe('object');
    expect(typeof plugin.rules['no-commented-code']).toBe('object');
    expect(typeof plugin.rules['no-incomplete-assertions']).toBe('object');
    expect(typeof plugin.rules['destructuring-assignment-syntax']).toBe('object');
    expect(typeof plugin.rules['no-element-overwrite']).toBe('object');
    expect(typeof plugin.rules['no-redundant-assignments']).toBe('object');
    expect(typeof plugin.rules['no-unused-collection']).toBe('object');
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
    expect(plugin.configs.recommended.rules['sonarjs/label-position']).toBe('error');
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
    expect(plugin.configs.recommended.rules['sonarjs/function-name']).toBeUndefined();
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
    expect(plugin.configs.recommended.rules['sonarjs/duplicates-in-character-class']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/anchor-precedence']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/cyclomatic-complexity']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-collection-size-mischeck']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/index-of-compare-to-positive-number']).toBe(
      'error',
    );
    expect(plugin.configs.recommended.rules['sonarjs/no-nested-functions']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/too-many-break-or-continue-in-loop']).toBe(
      'error',
    );
    expect(plugin.configs.recommended.rules['sonarjs/code-eval']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/prefer-promise-shorthand']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/pseudo-random']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/hashing']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-clear-text-protocols']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-hardcoded-ip']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-global-this']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/single-character-alternation']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/empty-string-repetition']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-misleading-array-reverse']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-alphabetical-sort']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-for-in-iterable']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-associative-arrays']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/bitwise-operators']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-same-argument-assert']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/inverted-assertion-arguments']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/for-loop-increment-sign']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-equals-in-for-termination']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/reduce-initial-value']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-parameter-reassignment']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/array-callback-without-return']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/declarations-in-global-scope']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-wildcard-import']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/updated-loop-counter']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/misplaced-loop-counter']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-array-delete']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-literal-call']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/shorthand-property-grouping']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/process-argv']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/standard-input']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-code-after-done']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/function-inside-loop']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-useless-intersection']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/use-type-alias']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/public-static-readonly']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/call-argument-line']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/prefer-object-literal']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-undefined-argument']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-identical-functions']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-in-misuse']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-require-or-define']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-invalid-regexp']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-invariant-returns']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-extra-arguments']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/link-with-target-blank']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-weak-cipher']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-hardcoded-passwords']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-ignored-exceptions']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-unused-function-argument']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/object-alt-content']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-use-of-empty-return-value']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-duplicated-branches']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/block-scoped-var']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-variable-usage-before-declaration']).toBe(
      'error',
    );
    expect(plugin.configs.recommended.rules['sonarjs/arguments-order']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/updated-const-var']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/unicode-aware-regex']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-undefined-assignment']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-empty-after-reluctant']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-ignored-return']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/file-name-differ-from-class']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-unenclosed-multiline-block']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/inconsistent-function-call']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/new-operator-misuse']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-empty-test-file']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/deprecation']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/cognitive-complexity']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/expression-complexity']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-fallthrough']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-commented-code']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-incomplete-assertions']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/destructuring-assignment-syntax']).toBe(
      'error',
    );
    expect(plugin.configs.recommended.rules['sonarjs/no-element-overwrite']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-redundant-assignments']).toBe('error');
    expect(plugin.configs.recommended.rules['sonarjs/no-unused-collection']).toBe('error');
  });
});

describe('too-many-break-or-continue-in-loop rule', () => {
  it('reports when a loop has two break statements targeting it', () => {
    const src = 'while (a) { if (b) break; if (c) break; }';
    const reports = runRule('too-many-break-or-continue-in-loop', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('tooManyBreakContinue');
  });

  it('does not report a loop with only one break', () => {
    const src = 'while (a) { if (b) break; }';
    const reports = runRule('too-many-break-or-continue-in-loop', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report breaks that target a nested switch, not the loop', () => {
    const src = 'while (a) { switch (x) { case 1: break; case 2: break; } }';
    const reports = runRule('too-many-break-or-continue-in-loop', src);
    expect(reports).toHaveLength(0);
  });

  it('reports too-many-break-or-continue-in-loop through the CLI', () => {
    const src = 'while (a) { if (b) break; if (c) break; }';
    const result = runOxlint('too-many-break-or-continue-in-loop', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(too-many-break-or-continue-in-loop)');
  });
});

describe('no-code-after-done rule', () => {
  it('reports a statement after a done() call in a test callback', () => {
    const src = "it('t', function (done) { done(); foo(); });";
    const reports = runRule('no-code-after-done', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noCodeAfterDone');
  });

  it('reports a statement after done() in an arrow hook callback', () => {
    const src = 'beforeEach((done) => { done(); cleanup(); });';
    const reports = runRule('no-code-after-done', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noCodeAfterDone');
  });

  it('does not report when done() is the last statement', () => {
    const src = "it('t', function (done) { foo(); done(); });";
    const reports = runRule('no-code-after-done', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report a trailing bare return after done()', () => {
    const src = "it('t', function (done) { done(); return; });";
    const reports = runRule('no-code-after-done', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report a callback without a done parameter', () => {
    const src = "it('t', function () { foo(); });";
    const reports = runRule('no-code-after-done', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the call is not a Mocha construct', () => {
    const src = 'register(function (done) { done(); foo(); });';
    const reports = runRule('no-code-after-done', src);
    expect(reports).toHaveLength(0);
  });

  it('reports no-code-after-done through the CLI', () => {
    const src = "it('t', function (done) { done(); foo(); });";
    const result = runOxlint('no-code-after-done', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-code-after-done)');
  });
});

describe('function-inside-loop rule', () => {
  it('reports an arrow function defined inside a for loop', () => {
    const src = 'for (let i = 0; i < 10; i++) { const f = () => i; }';
    const reports = runRule('function-inside-loop', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noFunctionInLoop');
  });

  it('reports a function declaration inside a while loop', () => {
    const src = 'while (x) { function g() {} }';
    const reports = runRule('function-inside-loop', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noFunctionInLoop');
  });

  it('reports a function expression inside a for-of loop', () => {
    const src = 'for (const x of xs) { const h = function () {}; }';
    const reports = runRule('function-inside-loop', src);
    expect(reports).toHaveLength(1);
  });

  it('does not report a function at top level', () => {
    const src = 'function h() {}';
    const reports = runRule('function-inside-loop', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report a loop with no function inside', () => {
    const src = 'for (const x of xs) { use(x); }';
    const reports = runRule('function-inside-loop', src);
    expect(reports).toHaveLength(0);
  });

  it('reports only the outer function nested directly in the loop', () => {
    const src = 'for (;;) { const a = () => { const b = () => {}; }; }';
    const reports = runRule('function-inside-loop', src);
    expect(reports).toHaveLength(1);
  });

  it('does not report an immediately invoked function expression', () => {
    const src = 'for (let i = 0; i < 10; i++) { (() => use(i))(); }';
    const reports = runRule('function-inside-loop', src);
    expect(reports).toHaveLength(0);
  });

  it('reports function-inside-loop through the CLI', () => {
    const src = 'for (let i = 0; i < 10; i++) { const f = () => i; }';
    const result = runOxlint('function-inside-loop', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(function-inside-loop)');
  });
});

describe('no-useless-intersection rule', () => {
  it('reports an "any" member in an intersection', () => {
    const reports = runRule('no-useless-intersection', 'type T = string & any;');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('uselessIntersection');
  });

  it('reports a "never" member in an intersection', () => {
    const reports = runRule('no-useless-intersection', 'type T = number & never;');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('uselessIntersection');
  });

  it('reports an "unknown" member in an intersection', () => {
    const reports = runRule('no-useless-intersection', 'type T = string & unknown;');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('uselessIntersection');
  });

  it('does not report an intersection without keyword members', () => {
    const reports = runRule('no-useless-intersection', 'type T = A & B;');
    expect(reports).toHaveLength(0);
  });

  it('does not report a union containing "any"', () => {
    const reports = runRule('no-useless-intersection', 'type T = string | any;');
    expect(reports).toHaveLength(0);
  });

  it('reports no-useless-intersection through the CLI', () => {
    const result = runOxlint('no-useless-intersection', 'type T = string & any;', 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-useless-intersection)');
  });
});

describe('use-type-alias rule', () => {
  it('reports a union type repeated three times', () => {
    const src = 'let a: string | number;\nlet b: string | number;\nlet c: string | number;';
    const reports = runRule('use-type-alias', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('useTypeAlias');
  });

  it('reports an intersection type repeated three times', () => {
    const src = 'let a: A & B;\nlet b: A & B;\nlet c: A & B;';
    const reports = runRule('use-type-alias', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('useTypeAlias');
  });

  it('does not report a union repeated only twice', () => {
    const src = 'let a: string | number;\nlet b: string | number;';
    const reports = runRule('use-type-alias', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report distinct unions each used once', () => {
    const src = 'let a: string | number;\nlet b: boolean | null;\nlet c: number | boolean;';
    const reports = runRule('use-type-alias', src);
    expect(reports).toHaveLength(0);
  });

  it('reports use-type-alias through the CLI', () => {
    const src = 'let a: string | number;\nlet b: string | number;\nlet c: string | number;';
    const result = runOxlint('use-type-alias', src, 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(use-type-alias)');
  });
});

describe('public-static-readonly rule', () => {
  it('reports a public-by-default static field through the adapter', () => {
    const reports = runRule('public-static-readonly', 'class C { static x = 1; }');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('publicStaticReadonly');
  });

  it('reports an explicit public static field through the adapter', () => {
    const reports = runRule('public-static-readonly', 'class C { public static x = 1; }');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('publicStaticReadonly');
  });

  it('does not report a static readonly field', () => {
    const reports = runRule('public-static-readonly', 'class C { static readonly x = 1; }');
    expect(reports).toHaveLength(0);
  });

  it('does not report a private static field', () => {
    const reports = runRule('public-static-readonly', 'class C { private static x = 1; }');
    expect(reports).toHaveLength(0);
  });

  it('does not report a protected static field', () => {
    const reports = runRule('public-static-readonly', 'class C { protected static x = 1; }');
    expect(reports).toHaveLength(0);
  });

  it('does not report a non-static field', () => {
    const reports = runRule('public-static-readonly', 'class C { x = 1; }');
    expect(reports).toHaveLength(0);
  });

  it('does not report a static #private field', () => {
    const reports = runRule('public-static-readonly', 'class C { static #x = 1; }');
    expect(reports).toHaveLength(0);
  });

  it('reports public-static-readonly through the CLI', () => {
    const result = runOxlint('public-static-readonly', 'class C { static x = 1; }', 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(public-static-readonly)');
  });
});

describe('call-argument-line rule', () => {
  it('reports a call whose open paren starts on a new line through the adapter', () => {
    const reports = runRule('call-argument-line', 'foo\n(arg);');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('sameLineAsCallee');
  });

  it('reports a zero-argument call whose paren starts on a new line', () => {
    const reports = runRule('call-argument-line', 'foo\n();');
    expect(reports).toHaveLength(1);
  });

  it('does not report a call with the paren on the callee line', () => {
    const reports = runRule('call-argument-line', 'foo(arg);');
    expect(reports).toHaveLength(0);
  });

  it('does not report arguments wrapped across lines with the paren on the callee line', () => {
    const reports = runRule('call-argument-line', 'foo(\n  a,\n  b\n);');
    expect(reports).toHaveLength(0);
  });

  it('does not report a member call with the paren on the callee line', () => {
    const reports = runRule('call-argument-line', 'obj.method(x);');
    expect(reports).toHaveLength(0);
  });

  it('reports call-argument-line through the CLI', () => {
    const result = runOxlint('call-argument-line', 'foo\n(arg);', 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(call-argument-line)');
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

  it('reports label-position through the adapter', () => {
    const source = 'unused: doWork();';
    const reports = runRule('label-position', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('removeLabel');
  });

  it('allows direct loop and switch labels through the adapter', () => {
    const source = `
      labelled_for: for (;;) { break labelled_for; }
      labelled_switch: switch (value) { case 1: break labelled_switch; }
    `;
    const reports = runRule('label-position', source);
    expect(reports).toHaveLength(0);
  });

  it('reports only the outer nested label through the adapter', () => {
    const source = 'outer: inner: for (;;) { break outer; }';
    const reports = runRule('label-position', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('removeLabel');
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

  it('reports label-position through the CLI', () => {
    const source = 'unused: doWork();';
    const result = runOxlint('label-position', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(label-position)');
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

  it('reports function-name through the adapter', () => {
    const source = 'function Bad_name() {} const Bad_name2 = () => {};';
    const reports = runRule('function-name', source);
    expect(reports).toHaveLength(2);
    expect(reports[0].messageId).toBe('renameFunction');
    expect(reports[0].data).toEqual({
      value: 'Bad_name',
      format: '^[_a-z][a-zA-Z0-9]*$',
    });
  });

  it('honors the function-name "format" option', () => {
    const source = 'function goodName() {} function GoodName() {} const goodName2 = () => {};';
    const reports = runRule('function-name', source, {
      options: [{ format: '^[A-Z][A-Za-z0-9]*$' }],
    });
    expect(reports).toHaveLength(2);
    expect(reports.map((report) => report.data.value)).toEqual(['goodName', 'goodName2']);
  });

  it('reports function-name through the CLI', () => {
    const source = 'function Bad_name() {}';
    const result = runOxlint('function-name', source);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(function-name)');
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

  it('reports duplicates-in-character-class through the adapter', () => {
    const source = 'const r = /[aa]/;';
    const reports = runRule('duplicates-in-character-class', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('duplicateCharacter');
  });

  it('reports duplicates-in-character-class through the CLI', () => {
    const result = runOxlint('duplicates-in-character-class', 'const r = /[aa]/;');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(duplicates-in-character-class)');
  });

  it('reports anchor-precedence through the adapter', () => {
    const source = 'const r = /^a|b|c$/;';
    const reports = runRule('anchor-precedence', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('anchorPrecedence');
  });

  it('reports anchor-precedence through the CLI', () => {
    const result = runOxlint('anchor-precedence', 'const r = /^a|b|c$/;');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(anchor-precedence)');
  });

  it('reports cyclomatic-complexity through the adapter with custom threshold', () => {
    // base 1 + 4 ifs = 5 > threshold 3 → 1 report
    const source = 'function f(a,b,c,d){if(a){}if(b){}if(c){}if(d){}}';
    const reports = runRule('cyclomatic-complexity', source, { options: [{ threshold: 3 }] });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('cyclomaticComplexity');
  });

  it('does not report cyclomatic-complexity when function is within the threshold', () => {
    // base 1 + 3 ifs = 4, threshold 4: 4 is not > 4 → 0 reports
    const source = 'function f(a,b,c){if(a){}if(b){}if(c){}}';
    const reports = runRule('cyclomatic-complexity', source, { options: [{ threshold: 4 }] });
    expect(reports).toHaveLength(0);
  });

  it('reports cyclomatic-complexity through the CLI', () => {
    // 11 ifs + base 1 = 12 > default threshold 10 → reported by CLI
    const src =
      'function f(a,b,c,d,e,f2,g,h,i,j,k)' +
      '{if(a){}if(b){}if(c){}if(d){}if(e){}' +
      'if(f2){}if(g){}if(h){}if(i){}if(j){}if(k){}}';
    const result = runOxlint('cyclomatic-complexity', src);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(cyclomatic-complexity)');
  });

  it('exposes the cyclomatic-complexity threshold option in the rule schema', () => {
    expect(plugin.rules['cyclomatic-complexity'].meta.schema).toEqual([
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ]);
  });

  it('reports no-collection-size-mischeck through the adapter', () => {
    const reports = runRule('no-collection-size-mischeck', 'const b = x.length < 0;');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('collectionSizeMischeck');
  });

  it('does not report no-collection-size-mischeck for x.length > 0', () => {
    const reports = runRule('no-collection-size-mischeck', 'const b = x.length > 0;');
    expect(reports).toHaveLength(0);
  });

  it('reports no-collection-size-mischeck through the CLI', () => {
    const result = runOxlint('no-collection-size-mischeck', 'const b = x.length < 0;');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-collection-size-mischeck)');
  });

  it('reports index-of-compare-to-positive-number through the adapter', () => {
    const reports = runRule('index-of-compare-to-positive-number', 'const b = a.indexOf(x) > 0;');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('indexOfPositive');
  });

  it('does not report index-of-compare-to-positive-number for indexOf >= 0', () => {
    const reports = runRule('index-of-compare-to-positive-number', 'const b = a.indexOf(x) >= 0;');
    expect(reports).toHaveLength(0);
  });

  it('reports index-of-compare-to-positive-number through the CLI', () => {
    const result = runOxlint('index-of-compare-to-positive-number', 'const b = a.indexOf(x) > 0;');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(index-of-compare-to-positive-number)');
  });

  it('reports no-nested-functions when depth 5 exceeds default threshold of 4', () => {
    const src = 'function a(){function b(){function c(){function d(){function e(){}}}}}';
    const reports = runRule('no-nested-functions', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noNestedFunctions');
  });

  it('does not report no-nested-functions when depth is exactly 4 (at threshold)', () => {
    const src = 'function a(){function b(){function c(){function d(){}}}}';
    const reports = runRule('no-nested-functions', src);
    expect(reports).toHaveLength(0);
  });

  it('honors the no-nested-functions "threshold" option through the adapter', () => {
    const src = 'function a(){function b(){function c(){}}}';
    expect(runRule('no-nested-functions', src, { options: [{ threshold: 2 }] })).toHaveLength(1);
    expect(runRule('no-nested-functions', src, { options: [{ threshold: 3 }] })).toHaveLength(0);
  });

  it('reports no-nested-functions through the CLI', () => {
    const src = 'function a(){function b(){function c(){function d(){function e(){}}}}}';
    const result = runOxlint('no-nested-functions', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-nested-functions)');
  });

  it('exposes the no-nested-functions threshold option in the rule schema', () => {
    expect(plugin.rules['no-nested-functions'].meta.schema).toEqual([
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ]);
  });
});

describe('code-eval rule', () => {
  it('reports code-eval for a bare eval call through the adapter', () => {
    const source = 'eval("x + 1");';
    const reports = runRule('code-eval', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('codeEval');
  });

  it('reports code-eval for new Function(...) through the adapter', () => {
    const source = 'const f = new Function("a", "return a");';
    const reports = runRule('code-eval', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('codeEval');
  });

  it('does not report code-eval for member-access eval call', () => {
    const source = 'window.eval("x");';
    const reports = runRule('code-eval', source);
    expect(reports).toHaveLength(0);
  });

  it('reports code-eval for bare eval call through the CLI', () => {
    const source = 'eval("x + 1");';
    const result = runOxlint('code-eval', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(code-eval)');
  });
});

describe('void-use rule', () => {
  it('reports void-use for void applied to a function call through the adapter', () => {
    const source = 'void foo();';
    const reports = runRule('void-use', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('voidUse');
  });

  it('does not report void-use for void 0 through the adapter', () => {
    const source = 'void 0;';
    const reports = runRule('void-use', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report void-use for void (0) through the adapter', () => {
    const source = 'void (0);';
    const reports = runRule('void-use', source);
    expect(reports).toHaveLength(0);
  });

  it('reports void-use for void applied to a function call through the CLI', () => {
    const source = 'void foo();';
    const result = runOxlint('void-use', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(void-use)');
  });
});

describe('prefer-promise-shorthand rule', () => {
  it('reports prefer-promise-shorthand for arrow expression body calling resolve', () => {
    const source = 'const p = new Promise((resolve) => resolve(42));';
    const reports = runRule('prefer-promise-shorthand', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferShorthand');
  });

  it('reports prefer-promise-shorthand for arrow expression body calling resolve with no arg', () => {
    const source = 'const p = new Promise((resolve) => resolve());';
    const reports = runRule('prefer-promise-shorthand', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferShorthand');
  });

  it('reports prefer-promise-shorthand for two-param arrow calling reject', () => {
    const source = 'const p = new Promise((resolve, reject) => reject(err));';
    const reports = runRule('prefer-promise-shorthand', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferShorthand');
  });

  it('reports prefer-promise-shorthand for function expression block body', () => {
    const source = 'const p = new Promise(function (resolve) { resolve(1); });';
    const reports = runRule('prefer-promise-shorthand', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferShorthand');
  });

  it('does not report when the executor has multiple statements', () => {
    const source = 'const p = new Promise((resolve, reject) => { doStuff(); resolve(1); });';
    const reports = runRule('prefer-promise-shorthand', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the call is not to resolve or reject', () => {
    const source = 'const p = new Promise((resolve) => setTimeout(resolve, 100));';
    const reports = runRule('prefer-promise-shorthand', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the executor is not an inline function', () => {
    const source = 'const p = new Promise(executor);';
    const reports = runRule('prefer-promise-shorthand', source);
    expect(reports).toHaveLength(0);
  });

  it('reports prefer-promise-shorthand through the CLI', () => {
    const source = 'const p = new Promise((resolve) => resolve(42));';
    const result = runOxlint('prefer-promise-shorthand', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(prefer-promise-shorthand)');
  });
});

describe('pseudo-random rule', () => {
  it('reports pseudo-random for Math.random() through the adapter', () => {
    const source = 'const x = Math.random();';
    const reports = runRule('pseudo-random', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('pseudoRandom');
  });

  it('reports pseudo-random for a bare Math.random() call through the adapter', () => {
    const source = 'Math.random();';
    const reports = runRule('pseudo-random', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('pseudoRandom');
  });

  it('does not report pseudo-random for Math.floor()', () => {
    const source = 'Math.floor(1.5);';
    const reports = runRule('pseudo-random', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report pseudo-random for foo.random()', () => {
    const source = 'foo.random();';
    const reports = runRule('pseudo-random', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report pseudo-random for a bare Math.random reference', () => {
    const source = 'const f = Math.random;';
    const reports = runRule('pseudo-random', source);
    expect(reports).toHaveLength(0);
  });

  it('reports pseudo-random through the CLI', () => {
    const source = 'const x = Math.random();';
    const result = runOxlint('pseudo-random', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(pseudo-random)');
  });
});

describe('no-hardcoded-ip rule', () => {
  it('reports no-hardcoded-ip for a private IPv4 address through the adapter', () => {
    const source = 'const ip = "192.168.1.1";';
    const reports = runRule('no-hardcoded-ip', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('hardcodedIp');
  });

  it('reports no-hardcoded-ip for an IPv4 address in a URL string', () => {
    const source = 'const url = "http://10.0.0.1/api";';
    const reports = runRule('no-hardcoded-ip', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('hardcodedIp');
  });

  it('does not report no-hardcoded-ip for the loopback address 127.0.0.1', () => {
    const source = 'const ip = "127.0.0.1";';
    const reports = runRule('no-hardcoded-ip', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for broadcast 255.255.255.255', () => {
    const source = 'const ip = "255.255.255.255";';
    const reports = runRule('no-hardcoded-ip', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for unspecified 0.0.0.0', () => {
    const source = 'const ip = "0.0.0.0";';
    const reports = runRule('no-hardcoded-ip', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-hardcoded-ip through the CLI', () => {
    const source = 'const ip = "192.168.1.1";';
    const result = runOxlint('no-hardcoded-ip', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-hardcoded-ip)');
  });
});

describe('hashing rule', () => {
  it('reports MD5 hashing through the adapter', () => {
    const reports = runRule('hashing', 'const h = crypto.createHash("md5");');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('weakHash');
  });

  it('reports SHA-1 WebCrypto digest through the adapter', () => {
    const reports = runRule('hashing', 'crypto.subtle.digest("SHA-1", data);');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('weakHash');
  });

  it('does not report strong hashing algorithms', () => {
    const reports = runRule('hashing', 'const h = crypto.createHash("sha256");');
    expect(reports).toHaveLength(0);
  });

  it('does not report dynamic hashing algorithms', () => {
    const reports = runRule('hashing', 'const h = crypto.createHash(algorithm);');
    expect(reports).toHaveLength(0);
  });

  it('reports hashing through the CLI', () => {
    const result = runOxlint('hashing', 'const h = crypto.createHash("sha1");');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(hashing)');
  });
});

describe('no-clear-text-protocols rule', () => {
  it('reports an HTTP URL through the adapter', () => {
    const reports = runRule('no-clear-text-protocols', 'const url = "http://example.com";');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('clearTextProtocol');
  });

  it('reports a clear-text WebSocket URL through the adapter', () => {
    const reports = runRule('no-clear-text-protocols', 'const url = "ws://example.com/socket";');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('clearTextProtocol');
  });

  it('does not report encrypted URL protocols', () => {
    const reports = runRule(
      'no-clear-text-protocols',
      'const a = "https://example.com"; const b = "wss://example.com/socket";',
    );
    expect(reports).toHaveLength(0);
  });

  it('does not report a protocol-like label without URL authority', () => {
    const reports = runRule('no-clear-text-protocols', 'const label = "http: status";');
    expect(reports).toHaveLength(0);
  });

  it('reports no-clear-text-protocols through the CLI', () => {
    const result = runOxlint('no-clear-text-protocols', 'const url = "ftp://example.com/file";');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-clear-text-protocols)');
  });
});

describe('no-global-this rule', () => {
  it('reports no-global-this for a top-level this expression through the adapter', () => {
    const source = 'this.foo = 1;';
    const reports = runRule('no-global-this', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noGlobalThis');
  });

  it('reports no-global-this for this inside a top-level arrow through the adapter', () => {
    const source = 'const f = () => this.x;';
    const reports = runRule('no-global-this', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noGlobalThis');
  });

  it('does not report no-global-this for this inside a regular function', () => {
    const source = 'function f() { return this.x; }';
    const reports = runRule('no-global-this', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report no-global-this for this inside a class field initializer', () => {
    const source = 'class C { x = this.y; }';
    const reports = runRule('no-global-this', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report no-global-this for this inside a class static block', () => {
    const source = 'class C { static { this.z(); } }';
    const reports = runRule('no-global-this', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-global-this through the CLI', () => {
    const source = 'this.foo = 1;';
    const result = runOxlint('no-global-this', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-global-this)');
  });
});

describe('single-character-alternation rule', () => {
  it('reports a top-level single-character alternation /a|b|c/', () => {
    const source = 'const re = /a|b|c/;';
    const reports = runRule('single-character-alternation', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('singleCharAlternation');
  });

  it('reports a single-character alternation inside a group /(a|b|c)/', () => {
    const source = 'const re = /(a|b|c)/;';
    const reports = runRule('single-character-alternation', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('singleCharAlternation');
  });

  it('does not report when an alternative is multi-char /ab|c/', () => {
    const source = 'const re = /ab|c/;';
    const reports = runRule('single-character-alternation', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when an alternative is a class escape /\\d|x/', () => {
    const source = 'const re = /\\d|x/;';
    const reports = runRule('single-character-alternation', source);
    expect(reports).toHaveLength(0);
  });

  it('reports single-character-alternation through the CLI', () => {
    const source = 'const re = /a|b|c/;';
    const result = runOxlint('single-character-alternation', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(single-character-alternation)');
  });
});

describe('empty-string-repetition rule', () => {
  it('reports * applied to a group containing a* (body matches empty)', () => {
    const source = 'const re = /(a*)*/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyStringRepetition');
  });

  it('reports + applied to a group containing a? (body matches empty)', () => {
    const source = 'const re = /(a?)+/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyStringRepetition');
  });

  it('reports * applied to an empty non-capturing group', () => {
    const source = 'const re = /(?:)*/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyStringRepetition');
  });

  it('reports + applied to an empty capturing group', () => {
    const source = 'const re = /()+/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyStringRepetition');
  });

  it('reports * applied to a disjunction with an empty alternative', () => {
    const source = 'const re = /(?:|a)*/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyStringRepetition');
  });

  it('does not report * when body is a literal character /a*/', () => {
    const source = 'const re = /a*/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report * when inner group body cannot match empty /(a+)*/', () => {
    const source = 'const re = /(a+)*/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report for /(?:)?/ — ? quantifier is not a repetition even on empty body', () => {
    const source = 'const re = /(?:)?/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report for /a?/ — ? quantifier is not a repetition', () => {
    const source = 'const re = /a?/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report for /(abc)+/ — body always consumes characters', () => {
    const source = 'const re = /(abc)+/;';
    const reports = runRule('empty-string-repetition', source);
    expect(reports).toHaveLength(0);
  });

  it('reports empty-string-repetition through the CLI', () => {
    const source = 'const re = /(a?)*/;';
    const result = runOxlint('empty-string-repetition', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(empty-string-repetition)');
  });
});

describe('no-misleading-array-reverse rule', () => {
  it('reports assigning the result of reverse() on a known array variable', () => {
    const source = 'const a = [3, 1, 2];\nconst b = a.reverse();';
    const reports = runRule('no-misleading-array-reverse', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('misleadingReverse');
  });

  it('reports assigning the result of sort() on a known array variable', () => {
    const source = 'const a = [3, 1, 2];\nlet b;\nb = a.sort();';
    const reports = runRule('no-misleading-array-reverse', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('misleadingReverse');
  });

  it('does not report a bare reverse() statement', () => {
    const source = 'const a = [3, 1, 2];\na.reverse();';
    const reports = runRule('no-misleading-array-reverse', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report reverse() on a fresh array literal', () => {
    const source = 'const b = [3, 1, 2].reverse();';
    const reports = runRule('no-misleading-array-reverse', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report reverse() on an unresolvable receiver', () => {
    const source = 'function f(a) {\n  return a.reverse();\n}';
    const reports = runRule('no-misleading-array-reverse', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-misleading-array-reverse through the CLI', () => {
    const source = 'const a = [3, 1, 2];\nconst b = a.reverse();';
    const result = runOxlint('no-misleading-array-reverse', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-misleading-array-reverse)');
  });
});

describe('no-alphabetical-sort rule', () => {
  it('reports sort() with no compare function on an array literal', () => {
    const source = '[3, 1, 2].sort();';
    const reports = runRule('no-alphabetical-sort', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('provideCompareFunction');
  });

  it('reports sort() with no compare function on a known array variable', () => {
    const source = 'const a = [3, 1, 2];\na.sort();';
    const reports = runRule('no-alphabetical-sort', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('provideCompareFunction');
  });

  it('reports toSorted() with no compare function on an array literal', () => {
    const source = '[3, 1, 2].toSorted();';
    const reports = runRule('no-alphabetical-sort', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('provideCompareFunction');
  });

  it('does not report sort() with a compare function', () => {
    const source = '[3, 1, 2].sort((x, y) => x - y);';
    const reports = runRule('no-alphabetical-sort', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report sort() on a non-array receiver', () => {
    const source = 'const obj = { sort() {} };\nobj.sort();';
    const reports = runRule('no-alphabetical-sort', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-alphabetical-sort through the CLI', () => {
    const source = '[3, 1, 2].sort();';
    const result = runOxlint('no-alphabetical-sort', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-alphabetical-sort)');
  });
});

describe('no-for-in-iterable rule', () => {
  it('reports a for...in loop over an array literal', () => {
    const source = 'for (const i in [1, 2, 3]) {\n}';
    const reports = runRule('no-for-in-iterable', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noForInIterable');
  });

  it('reports a for...in loop over a known array variable', () => {
    const source = 'const a = [1, 2, 3];\nfor (const i in a) {\n}';
    const reports = runRule('no-for-in-iterable', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noForInIterable');
  });

  it('does not report a for...in loop over an object literal', () => {
    const source = 'const obj = { a: 1 };\nfor (const k in obj) {\n}';
    const reports = runRule('no-for-in-iterable', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a for...of loop over an array literal', () => {
    const source = 'for (const x of [1, 2, 3]) {\n}';
    const reports = runRule('no-for-in-iterable', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-for-in-iterable through the CLI', () => {
    const source = 'for (const i in [1, 2, 3]) {\n}';
    const result = runOxlint('no-for-in-iterable', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-for-in-iterable)');
  });
});

describe('no-associative-arrays rule', () => {
  it('reports a computed non-numeric string-key write on an array variable', () => {
    const source = "const a = [];\na['key'] = 1;";
    const reports = runRule('no-associative-arrays', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noAssociativeArray');
  });

  it('reports a static non-numeric key write on an array variable', () => {
    const source = 'const a = [];\na.foo = 1;';
    const reports = runRule('no-associative-arrays', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noAssociativeArray');
  });

  it('does not report a numeric index write', () => {
    const source = 'const a = [];\na[0] = 1;';
    const reports = runRule('no-associative-arrays', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a length write', () => {
    const source = 'const a = [];\na.length = 0;';
    const reports = runRule('no-associative-arrays', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a write on a non-array object', () => {
    const source = 'const o = {};\no.foo = 1;';
    const reports = runRule('no-associative-arrays', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-associative-arrays through the CLI', () => {
    const source = "const a = [];\na['key'] = 1;";
    const result = runOxlint('no-associative-arrays', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-associative-arrays)');
  });
});

describe('bitwise-operators rule', () => {
  it('reports a bitwise & when an operand is a comparison', () => {
    const source = 'if (a < 1 & b > 2) {\n}';
    const reports = runRule('bitwise-operators', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('bitwiseOperator');
  });

  it('reports a bitwise | when an operand is an equality check', () => {
    const source = 'const x = (a === b) | c;';
    const reports = runRule('bitwise-operators', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('bitwiseOperator');
  });

  it('does not report a bitwise & on numeric/identifier operands', () => {
    const source = 'const y = flags & MASK;';
    const reports = runRule('bitwise-operators', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a logical && expression', () => {
    const source = 'if (a < 1 && b > 2) {\n}';
    const reports = runRule('bitwise-operators', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a bitwise ^ even with a comparison operand', () => {
    const source = 'const z = (a === b) ^ c;';
    const reports = runRule('bitwise-operators', source);
    expect(reports).toHaveLength(0);
  });

  it('reports bitwise-operators through the CLI', () => {
    const source = 'if (a < 1 & b > 2) {\n}';
    const result = runOxlint('bitwise-operators', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(bitwise-operators)');
  });
});

describe('no-same-argument-assert rule', () => {
  it('reports assert.equal called with the same argument twice', () => {
    const source = 'assert.equal(x, x);';
    const reports = runRule('no-same-argument-assert', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('sameArgumentAssert');
  });

  it('reports assert.strictEqual called with the same member expression twice', () => {
    const source = 'assert.strictEqual(foo.bar, foo.bar);';
    const reports = runRule('no-same-argument-assert', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('sameArgumentAssert');
  });

  it('does not report assert.equal called with different arguments', () => {
    const source = 'assert.equal(x, y);';
    const reports = runRule('no-same-argument-assert', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a generic call with repeated arguments', () => {
    const source = 'foo(x, x);';
    const reports = runRule('no-same-argument-assert', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an assertion with a single argument', () => {
    const source = 'assert.ok(x);';
    const reports = runRule('no-same-argument-assert', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-same-argument-assert through the CLI', () => {
    const source = 'assert.equal(x, x);';
    const result = runOxlint('no-same-argument-assert', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-same-argument-assert)');
  });
});

describe('inverted-assertion-arguments rule', () => {
  it('reports assert.equal called with a numeric literal first', () => {
    const source = 'assert.equal(42, x);';
    const reports = runRule('inverted-assertion-arguments', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('invertedArguments');
  });

  it('reports assert.strictEqual called with a string literal first', () => {
    const source = "assert.strictEqual('foo', bar);";
    const reports = runRule('inverted-assertion-arguments', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('invertedArguments');
  });

  it('does not report arguments already in actual/expected order', () => {
    const source = 'assert.equal(x, 42);';
    const reports = runRule('inverted-assertion-arguments', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when both arguments are literals', () => {
    const source = 'assert.equal(1, 2);';
    const reports = runRule('inverted-assertion-arguments', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when neither argument is a literal', () => {
    const source = 'assert.equal(x, y);';
    const reports = runRule('inverted-assertion-arguments', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a generic non-assert call', () => {
    const source = 'foo(42, x);';
    const reports = runRule('inverted-assertion-arguments', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an assertion with a single argument', () => {
    const source = 'assert.ok(x);';
    const reports = runRule('inverted-assertion-arguments', source);
    expect(reports).toHaveLength(0);
  });

  it('reports inverted-assertion-arguments through the CLI', () => {
    const source = 'assert.equal(42, x);';
    const result = runOxlint('inverted-assertion-arguments', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(inverted-assertion-arguments)');
  });
});

describe('for-loop-increment-sign rule', () => {
  it('reports an increasing condition with a decrementing update', () => {
    const source = 'for (let i = 0; i < 10; i--) {}';
    const reports = runRule('for-loop-increment-sign', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('wrongDirection');
  });

  it('reports a decreasing condition with an incrementing update', () => {
    const source = 'for (let i = 10; i > 0; i++) {}';
    const reports = runRule('for-loop-increment-sign', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('wrongDirection');
  });

  it('reports a compound subtract-assign against an increasing condition', () => {
    const source = 'for (let i = 0; i <= 10; i -= 1) {}';
    const reports = runRule('for-loop-increment-sign', source);
    expect(reports).toHaveLength(1);
  });

  it('does not report a correctly increasing loop', () => {
    const source = 'for (let i = 0; i < 10; i++) {}';
    const reports = runRule('for-loop-increment-sign', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a correctly decreasing loop', () => {
    const source = 'for (let i = 10; i > 0; i--) {}';
    const reports = runRule('for-loop-increment-sign', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an equality condition with no direction', () => {
    const source = 'for (let i = 0; i != 10; i++) {}';
    const reports = runRule('for-loop-increment-sign', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the update variable differs from the counter', () => {
    const source = 'for (let i = 0, j = 0; i < 10; j++) {}';
    const reports = runRule('for-loop-increment-sign', source);
    expect(reports).toHaveLength(0);
  });

  it('reports for-loop-increment-sign through the CLI', () => {
    const source = 'for (let i = 0; i < 10; i--) {}';
    const result = runOxlint('for-loop-increment-sign', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(for-loop-increment-sign)');
  });
});

describe('no-equals-in-for-termination rule', () => {
  it('reports an inequality condition with a non-unit compound step', () => {
    const source = 'for (let i = 0; i != 10; i += 2) {}';
    const reports = runRule('no-equals-in-for-termination', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noEqualsInForTermination');
  });

  it('reports a strict-inequality condition with a non-unit plain assignment', () => {
    const source = 'for (let i = 0; i !== 10; i = i + 2) {}';
    const reports = runRule('no-equals-in-for-termination', source);
    expect(reports).toHaveLength(1);
  });

  it('does not report a unit increment', () => {
    const source = 'for (let i = 0; i != 10; i++) {}';
    const reports = runRule('no-equals-in-for-termination', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a relational condition', () => {
    const source = 'for (let i = 0; i < 10; i += 2) {}';
    const reports = runRule('no-equals-in-for-termination', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the update variable differs from the counter', () => {
    const source = 'for (let i = 0, j = 0; i != 10; j += 2) {}';
    const reports = runRule('no-equals-in-for-termination', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-equals-in-for-termination through the CLI', () => {
    const source = 'for (let i = 0; i != 10; i += 2) {}';
    const result = runOxlint('no-equals-in-for-termination', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-equals-in-for-termination)');
  });
});

describe('reduce-initial-value rule', () => {
  it('reports reduce() with no initial value on an array literal', () => {
    const source = '[1, 2, 3].reduce((a, b) => a + b);';
    const reports = runRule('reduce-initial-value', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('provideInitialValue');
  });

  it('reports reduce() with no initial value on a known array variable', () => {
    const source = 'const a = [1, 2];\na.reduce((x, y) => x + y);';
    const reports = runRule('reduce-initial-value', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('provideInitialValue');
  });

  it('reports reduceRight() with no initial value on an array literal', () => {
    const source = '[1, 2, 3].reduceRight((a, b) => a + b);';
    const reports = runRule('reduce-initial-value', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('provideInitialValue');
  });

  it('does not report reduce() with an initial value', () => {
    const source = '[1, 2, 3].reduce((a, b) => a + b, 0);';
    const reports = runRule('reduce-initial-value', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report reduce() on a non-array receiver', () => {
    const source = 'const obj = { reduce() {} };\nobj.reduce(fn);';
    const reports = runRule('reduce-initial-value', source);
    expect(reports).toHaveLength(0);
  });

  it('reports reduce-initial-value through the CLI', () => {
    const source = '[1, 2, 3].reduce((a, b) => a + b);';
    const result = runOxlint('reduce-initial-value', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(reduce-initial-value)');
  });
});

describe('no-parameter-reassignment rule', () => {
  it('reports reassigning a function parameter', () => {
    const source = 'function f(p) { p = 1; }';
    const reports = runRule('no-parameter-reassignment', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noParameterReassignment');
  });

  it('reports incrementing a function parameter', () => {
    const source = 'function f(p) { p++; }';
    const reports = runRule('no-parameter-reassignment', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noParameterReassignment');
  });

  it('reports compound-assigning an arrow parameter', () => {
    const source = 'const g = (a) => { a += 2; };';
    const reports = runRule('no-parameter-reassignment', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noParameterReassignment');
  });

  it('reports reassigning a caught exception', () => {
    const source = 'try {} catch (e) { e = null; }';
    const reports = runRule('no-parameter-reassignment', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noParameterReassignment');
  });

  it('reports reassigning a for-of loop variable', () => {
    const source = 'for (const x of xs) { x = 0; }';
    const reports = runRule('no-parameter-reassignment', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noParameterReassignment');
  });

  it('does not report writing to a parameter property', () => {
    const source = 'function f(p) { p.x = 1; }';
    const reports = runRule('no-parameter-reassignment', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report reassigning a local variable copy', () => {
    const source = 'function f(p) { let q = p; q = 2; }';
    const reports = runRule('no-parameter-reassignment', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report reassigning a module-scope variable', () => {
    const source = 'let x = 1; x = 2;';
    const reports = runRule('no-parameter-reassignment', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a classic for-loop counter', () => {
    const source = 'function f() { for (let i = 0; i < 3; i++) { i = 2; } }';
    const reports = runRule('no-parameter-reassignment', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-parameter-reassignment through the CLI', () => {
    const source = 'function f(p) { p = 1; }';
    const result = runOxlint('no-parameter-reassignment', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-parameter-reassignment)');
  });
});

describe('array-callback-without-return rule', () => {
  it('reports a map callback (function expression) that never returns', () => {
    const source = '[1, 2].map(function (x) { console.log(x); });';
    const reports = runRule('array-callback-without-return', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('addReturn');
  });

  it('reports a filter arrow with a block body and no return', () => {
    const source = 'arr.filter((x) => { doStuff(x); });';
    const reports = runRule('array-callback-without-return', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('addReturn');
  });

  it('reports a sort comparator that only throws', () => {
    const source = 'arr.sort((a, b) => { throw new Error("x"); });';
    const reports = runRule('array-callback-without-return', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('addReturn');
  });

  it('does not report an arrow with an expression body', () => {
    const source = '[1, 2].map((x) => x + 1);';
    const reports = runRule('array-callback-without-return', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a callback that returns a value', () => {
    const source = 'arr.filter(function (x) { return x > 0; });';
    const reports = runRule('array-callback-without-return', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a value return nested in control flow', () => {
    const source = 'arr.map((x) => { if (x) { return x; } });';
    const reports = runRule('array-callback-without-return', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a forEach callback (method not covered)', () => {
    const source = 'arr.forEach((x) => { log(x); });';
    const reports = runRule('array-callback-without-return', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the callback is an identifier', () => {
    const source = 'arr.map(fn);';
    const reports = runRule('array-callback-without-return', source);
    expect(reports).toHaveLength(0);
  });

  it('reports array-callback-without-return through the CLI', () => {
    const source = '[1, 2].map(function (x) { console.log(x); });';
    const result = runOxlint('array-callback-without-return', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(array-callback-without-return)');
  });
});

describe('declarations-in-global-scope rule', () => {
  it('reports a top-level function declaration', () => {
    const reports = runRule('declarations-in-global-scope', 'function f() {}');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('defineLocally');
  });

  it('reports named exported function declarations', () => {
    const source = 'export function f() {}\nexport default function g() {}';
    const reports = runRule('declarations-in-global-scope', source);
    expect(reports).toHaveLength(2);
  });

  it('does not report anonymous default function exports', () => {
    const reports = runRule('declarations-in-global-scope', 'export default function () {}');
    expect(reports).toHaveLength(0);
  });

  it('reports every non-require top-level var declarator', () => {
    const source = "var fs = require('fs'), value = 1, other = 2;";
    const reports = runRule('declarations-in-global-scope', source);
    expect(reports).toHaveLength(2);
  });

  it('reports var declarations nested in top-level blocks', () => {
    const reports = runRule('declarations-in-global-scope', 'if (enabled) { var leaked = 1; }');
    expect(reports).toHaveLength(1);
  });

  it('only reports the outer function when other declarations are local or allowed', () => {
    const source = `
      let a = 1;
      const b = 2;
      var fs = require('fs');
      function outer() { var local = 1; function inner() {} }
      class C { static { var staticLocal = 1; } }
    `;
    const reports = runRule('declarations-in-global-scope', source);
    expect(reports).toHaveLength(1);
  });

  it('reports declarations-in-global-scope through the CLI', () => {
    const result = runOxlint('declarations-in-global-scope', 'var leaked = 1;');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(declarations-in-global-scope)');
  });
});

describe('no-wildcard-import rule', () => {
  it('reports a wildcard namespace import', () => {
    const source = "import * as ns from 'mod';";
    const reports = runRule('no-wildcard-import', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noWildcardImport');
  });

  it('reports a combined default and wildcard import', () => {
    const source = "import def, * as ns from 'mod';";
    const reports = runRule('no-wildcard-import', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noWildcardImport');
  });

  it('does not report a named import', () => {
    const source = "import { a, b } from 'mod';";
    const reports = runRule('no-wildcard-import', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a default import', () => {
    const source = "import def from 'mod';";
    const reports = runRule('no-wildcard-import', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a side-effect import', () => {
    const source = "import 'mod';";
    const reports = runRule('no-wildcard-import', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a re-export (export *)', () => {
    const source = "export * from 'mod';";
    const reports = runRule('no-wildcard-import', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-wildcard-import through the CLI', () => {
    const source = "import * as ns from 'mod';";
    const result = runOxlint('no-wildcard-import', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-wildcard-import)');
  });
});

describe('updated-loop-counter rule', () => {
  it('reports reassigning the loop counter in the body', () => {
    const source = 'for (let i = 0; i < 10; i++) { i = 5; }';
    const reports = runRule('updated-loop-counter', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noCounterUpdate');
  });

  it('reports compound-assigning the loop counter in the body', () => {
    const source = 'for (let i = 0; i < 10; i++) { i += 2; }';
    const reports = runRule('updated-loop-counter', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noCounterUpdate');
  });

  it('reports decrementing the loop counter inside a branch', () => {
    const source = 'for (let i = 0; i < 10; i++) { if (x) i--; }';
    const reports = runRule('updated-loop-counter', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noCounterUpdate');
  });

  it('does not report a counter touched only by the update clause', () => {
    const source = 'for (let i = 0; i < 10; i++) {}';
    const reports = runRule('updated-loop-counter', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a shadowing local of the same name', () => {
    const source = 'for (let i = 0; i < 10; i++) { let i = 0; i = 5; }';
    const reports = runRule('updated-loop-counter', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report writing to a different variable', () => {
    const source = 'for (let i = 0; i < 10; i++) { j = 5; }';
    const reports = runRule('updated-loop-counter', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a property write on the counter', () => {
    const source = 'for (let i = 0; i < 10; i++) { i.x = 5; }';
    const reports = runRule('updated-loop-counter', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report reassigning a for-of loop variable', () => {
    const source = 'for (const x of xs) { x = 1; }';
    const reports = runRule('updated-loop-counter', source);
    expect(reports).toHaveLength(0);
  });

  it('reports updated-loop-counter through the CLI', () => {
    const source = 'for (let i = 0; i < 10; i++) { i = 5; }';
    const result = runOxlint('updated-loop-counter', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(updated-loop-counter)');
  });
});

describe('misplaced-loop-counter rule', () => {
  it('reports an update that increments a variable absent from the condition', () => {
    const source = 'for (let i = 0; i < 10; j++) {}';
    const reports = runRule('misplaced-loop-counter', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('misplacedCounter');
  });

  it('reports a compound assignment to a non-condition variable', () => {
    const source = 'for (let i = 0; i < 10; k += 1) {}';
    const reports = runRule('misplaced-loop-counter', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('misplacedCounter');
  });

  it('does not report when the update advances the condition counter', () => {
    const source = 'for (let i = 0; i < 10; i++) {}';
    const reports = runRule('misplaced-loop-counter', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a comma update overlapping the condition', () => {
    const source = 'for (let i = 0, j = 0; i < 10 && j < 5; i++, j++) {}';
    const reports = runRule('misplaced-loop-counter', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the condition uses the counter inside a member access', () => {
    const source = 'for (let i = 0; arr[i] < 10; i++) {}';
    const reports = runRule('misplaced-loop-counter', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a loop with no test or update', () => {
    const source = 'for (;;) {}';
    const reports = runRule('misplaced-loop-counter', source);
    expect(reports).toHaveLength(0);
  });

  it('reports misplaced-loop-counter through the CLI', () => {
    const source = 'for (let i = 0; i < 10; j++) {}';
    const result = runOxlint('misplaced-loop-counter', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(misplaced-loop-counter)');
  });
});

describe('no-array-delete rule', () => {
  it('reports delete on a resolved array variable element', () => {
    const source = 'const a = [1, 2, 3];\ndelete a[0];';
    const reports = runRule('no-array-delete', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noArrayDelete');
  });

  it('reports delete on a direct array-literal element', () => {
    const source = 'delete [1, 2][0];';
    const reports = runRule('no-array-delete', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noArrayDelete');
  });

  it('does not report delete on an object property', () => {
    const source = 'const o = { x: 1 };\ndelete o.x;';
    const reports = runRule('no-array-delete', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report delete on a static array member', () => {
    const source = 'const a = [1, 2, 3];\ndelete a.foo;';
    const reports = runRule('no-array-delete', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report delete on an unprovable receiver', () => {
    const source = 'function f(p) {\n  delete p[0];\n}';
    const reports = runRule('no-array-delete', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-array-delete through the CLI', () => {
    const source = 'const a = [1, 2, 3];\ndelete a[0];';
    const result = runOxlint('no-array-delete', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-array-delete)');
  });
});

describe('no-literal-call rule', () => {
  it('reports a boolean literal called as a function', () => {
    const source = 'true();';
    const reports = runRule('no-literal-call', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noLiteralCall');
  });

  it('reports a number literal called as a function', () => {
    const source = '(42)();';
    const reports = runRule('no-literal-call', source);
    expect(reports).toHaveLength(1);
  });

  it('reports a literal used as a tagged-template tag', () => {
    const source = 'true`text`;';
    const reports = runRule('no-literal-call', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noLiteralCall');
  });

  it('does not report an ordinary function call', () => {
    const source = 'foo();';
    const reports = runRule('no-literal-call', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a callable tagged template', () => {
    const source = 'foo`text`;';
    const reports = runRule('no-literal-call', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-literal-call through the CLI', () => {
    const source = 'true();';
    const result = runOxlint('no-literal-call', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-literal-call)');
  });
});

describe('shorthand-property-grouping rule', () => {
  it('reports shorthand properties split by a regular property', () => {
    const source = 'const o = { a, x: 1, b };';
    const reports = runRule('shorthand-property-grouping', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('groupShorthand');
  });

  it('reports a lone shorthand property in the middle', () => {
    const source = 'const o = { x: 1, a, y: 2 };';
    const reports = runRule('shorthand-property-grouping', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('groupShorthand');
  });

  it('does not report shorthand grouped at the beginning', () => {
    const source = 'const o = { a, b, x: 1 };';
    const reports = runRule('shorthand-property-grouping', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an all-shorthand object', () => {
    const source = 'const o = { a, b };';
    const reports = runRule('shorthand-property-grouping', source);
    expect(reports).toHaveLength(0);
  });

  it('reports shorthand-property-grouping through the CLI', () => {
    const source = 'const o = { a, x: 1, b };';
    const result = runOxlint('shorthand-property-grouping', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(shorthand-property-grouping)');
  });
});

describe('process-argv rule', () => {
  it('reports process-argv for a direct process.argv access through the adapter', () => {
    const source = 'const a = process.argv;';
    const reports = runRule('process-argv', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('processArgv');
  });

  it('reports process-argv once for process.argv[2] through the adapter', () => {
    const source = 'process.argv[2];';
    const reports = runRule('process-argv', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('processArgv');
  });

  it('reports process-argv once for process.argv.slice(2) through the adapter', () => {
    const source = 'process.argv.slice(2);';
    const reports = runRule('process-argv', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('processArgv');
  });

  it('does not report process-argv for process.env', () => {
    const source = 'process.env.PATH;';
    const reports = runRule('process-argv', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report process-argv for foo.argv', () => {
    const source = 'foo.argv;';
    const reports = runRule('process-argv', source);
    expect(reports).toHaveLength(0);
  });

  it('reports process-argv through the CLI', () => {
    const source = 'const a = process.argv;';
    const result = runOxlint('process-argv', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(process-argv)');
  });
});

describe('standard-input rule', () => {
  it('reports standard-input for a direct process.stdin access through the adapter', () => {
    const source = 'const x = process.stdin;';
    const reports = runRule('standard-input', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('standardInput');
  });

  it('reports standard-input once for process.stdin.on through the adapter', () => {
    const source = "process.stdin.on('data', cb);";
    const reports = runRule('standard-input', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('standardInput');
  });

  it('reports standard-input once for process.stdin.read() through the adapter', () => {
    const source = 'process.stdin.read();';
    const reports = runRule('standard-input', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('standardInput');
  });

  it('does not report standard-input for process.stdout', () => {
    const source = 'process.stdout;';
    const reports = runRule('standard-input', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report standard-input for foo.stdin', () => {
    const source = 'foo.stdin;';
    const reports = runRule('standard-input', source);
    expect(reports).toHaveLength(0);
  });

  it('reports standard-input through the CLI', () => {
    const source = 'const x = process.stdin;';
    const result = runOxlint('standard-input', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(standard-input)');
  });
});

describe('prefer-object-literal rule', () => {
  it('reports an empty object literal followed by a property assignment', () => {
    const source = 'let person = {};\nperson.name = "John";';
    const reports = runRule('prefer-object-literal', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferObjectLiteral');
  });

  it('reports an empty object literal followed by a computed property assignment', () => {
    const source = 'let p = {};\np["name"] = "John";';
    const reports = runRule('prefer-object-literal', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferObjectLiteral');
  });

  it('does not report a non-empty object literal', () => {
    const source = 'let person = { name: "John" };\nperson.age = 42;';
    const reports = runRule('prefer-object-literal', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the next statement reads the variable', () => {
    const source = 'let p = {};\nfoo(p);';
    const reports = runRule('prefer-object-literal', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an empty object declaration with no following statement', () => {
    const source = 'let p = {};';
    const reports = runRule('prefer-object-literal', source);
    expect(reports).toHaveLength(0);
  });

  it('reports prefer-object-literal through the CLI', () => {
    const source = 'let person = {};\nperson.name = "John";';
    const result = runOxlint('prefer-object-literal', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(prefer-object-literal)');
  });
});

describe('no-undefined-argument rule', () => {
  it('reports a sole undefined argument in a call expression', () => {
    const reports = runRule('no-undefined-argument', 'foo(undefined);');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('removeUndefined');
  });

  it('reports a trailing undefined after other arguments', () => {
    const reports = runRule('no-undefined-argument', 'foo(1, undefined);');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('removeUndefined');
  });

  it('reports a sole undefined argument in a new expression', () => {
    const reports = runRule('no-undefined-argument', 'new Foo(undefined);');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('removeUndefined');
  });

  it('does not report when undefined is not the last argument', () => {
    const reports = runRule('no-undefined-argument', 'foo(undefined, 1);');
    expect(reports).toHaveLength(0);
  });

  it('does not report a call with no arguments', () => {
    const reports = runRule('no-undefined-argument', 'foo();');
    expect(reports).toHaveLength(0);
  });

  it('does not report a call with no undefined arguments', () => {
    const reports = runRule('no-undefined-argument', 'foo(1, 2);');
    expect(reports).toHaveLength(0);
  });

  it('reports no-undefined-argument through the CLI', () => {
    const result = runOxlint('no-undefined-argument', 'foo(undefined);');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-undefined-argument)');
  });
});

describe('no-identical-functions rule', () => {
  const twoIdentical = [
    'function a(x) {',
    '  if (x > 0) return x;',
    '  return -x;',
    '}',
    'function b(x) {',
    '  if (x > 0) return x;',
    '  return -x;',
    '}',
  ].join('\n');

  it('reports the second of two identical functions', () => {
    const reports = runRule('no-identical-functions', twoIdentical);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('identicalFunctions');
  });

  it('does not report two functions with different bodies', () => {
    const src = [
      'function a(x) {',
      '  if (x > 0) return x;',
      '  return -x;',
      '}',
      'function b(x) {',
      '  if (x > 0) return x + 1;',
      '  return -x;',
      '}',
    ].join('\n');
    const reports = runRule('no-identical-functions', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report expression-bodied arrow functions', () => {
    const src = 'const f = x => x + 1;\nconst g = x => x + 1;';
    const reports = runRule('no-identical-functions', src);
    expect(reports).toHaveLength(0);
  });

  it('reports no-identical-functions through the CLI', () => {
    const result = runOxlint('no-identical-functions', twoIdentical);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-identical-functions)');
  });
});

describe('no-in-misuse rule', () => {
  it('reports a string value used with in against a direct array literal', () => {
    const src = 'const found = "apple" in ["apple", "banana"];';
    const reports = runRule('no-in-misuse', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('inMisuse');
  });

  it('reports a string value used with in against a const array identifier', () => {
    const src = 'const fruits = ["apple", "banana"]; const found = "apple" in fruits;';
    const reports = runRule('no-in-misuse', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('inMisuse');
  });

  it('does not report when the left operand is a numeric index string', () => {
    const src = 'const found = "0" in ["apple", "banana"];';
    const reports = runRule('no-in-misuse', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the left operand is an Array.prototype member name', () => {
    const src = 'const hasLength = "length" in ["apple"];';
    const reports = runRule('no-in-misuse', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the left operand is not a string literal', () => {
    const src = 'const k = "apple"; const found = k in ["apple", "banana"];';
    const reports = runRule('no-in-misuse', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the right operand is not a provable array', () => {
    const src = 'const found = "apple" in someObject;';
    const reports = runRule('no-in-misuse', src);
    expect(reports).toHaveLength(0);
  });

  it('reports no-in-misuse through the CLI', () => {
    const src = 'const found = "apple" in ["apple", "banana"];';
    const result = runOxlint('no-in-misuse', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-in-misuse)');
  });
});

describe('no-require-or-define rule', () => {
  it('flags a bare require() call', () => {
    const reports = runRule('no-require-or-define', "require('fs');");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noRequireOrDefine');
  });

  it('flags require() used in a variable declaration', () => {
    const reports = runRule('no-require-or-define', "const x = require('./utils');");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noRequireOrDefine');
  });

  it('flags a bare define() call', () => {
    const reports = runRule('no-require-or-define', "define(['dep'], function(dep) {});");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noRequireOrDefine');
  });

  it('does not flag a member-expression require', () => {
    const reports = runRule('no-require-or-define', "foo.require('x');");
    expect(reports).toHaveLength(0);
  });

  it('does not flag an ES import statement', () => {
    const reports = runRule('no-require-or-define', "import x from 'fs';");
    expect(reports).toHaveLength(0);
  });

  it('does not flag a function whose name only contains require as a substring', () => {
    const reports = runRule('no-require-or-define', 'function f() { requireSomething(); }');
    expect(reports).toHaveLength(0);
  });

  it('reports no-require-or-define through the CLI', () => {
    const result = runOxlint('no-require-or-define', "require('fs');");
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-require-or-define)');
  });
});

describe('no-invalid-regexp rule', () => {
  it('reports an unclosed bracket passed to new RegExp', () => {
    const reports = runRule('no-invalid-regexp', "new RegExp('[');");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('invalidRegExp');
  });

  it('reports an unclosed group passed to RegExp call', () => {
    const reports = runRule('no-invalid-regexp', "RegExp('(');");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('invalidRegExp');
  });

  it('reports an invalid flag', () => {
    const reports = runRule('no-invalid-regexp', "new RegExp('a', 'z');");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('invalidRegExp');
  });

  it('does not report a valid pattern', () => {
    const reports = runRule('no-invalid-regexp', "new RegExp('abc');");
    expect(reports).toHaveLength(0);
  });

  it('does not report when the argument is a variable', () => {
    const reports = runRule('no-invalid-regexp', 'new RegExp(somePattern);');
    expect(reports).toHaveLength(0);
  });

  it('does not report a valid digit escape (cooked value is valid)', () => {
    // JS source: new RegExp('\\d+') — the cooked string value is \d+ which is valid
    const reports = runRule('no-invalid-regexp', "new RegExp('\\\\d+');");
    expect(reports).toHaveLength(0);
  });

  it('reports no-invalid-regexp through the CLI', () => {
    const result = runOxlint('no-invalid-regexp', "new RegExp('[');");
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-invalid-regexp)');
  });
});

describe('no-invariant-returns rule', () => {
  it('reports a function that always returns the same value', () => {
    const source = 'function f(x) {\n  if (x > 0) return 42;\n  return 42;\n}';
    const reports = runRule('no-invariant-returns', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('invariantReturn');
  });

  it('does not report when return values differ', () => {
    const source = 'function f(x) {\n  if (x > 0) return 1;\n  return 2;\n}';
    const reports = runRule('no-invariant-returns', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when a bare return is present', () => {
    const source = 'function f(x) {\n  if (!x) return;\n  return 42;\n}';
    const reports = runRule('no-invariant-returns', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-invariant-returns through the CLI', () => {
    const source = 'function f(x) {\n  if (x > 0) return 42;\n  return 42;\n}';
    const result = runOxlint('no-invariant-returns', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-invariant-returns)');
  });
});

describe('no-extra-arguments rule', () => {
  it('reports when a const-assigned function expression is called with too many arguments', () => {
    const src = 'const f = function(a){}; f(1, 2);';
    const reports = runRule('no-extra-arguments', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('extraArguments');
  });

  it('reports when a const-assigned arrow function is called with too many arguments', () => {
    const src = 'const g = (a) => a; g(1, 2, 3);';
    const reports = runRule('no-extra-arguments', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('extraArguments');
  });

  it('does not report when the argument count exactly matches the parameter count', () => {
    const src = 'const f = (a, b) => {}; f(1, 2);';
    const reports = runRule('no-extra-arguments', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when fewer arguments than parameters are passed', () => {
    const src = 'const f = (a) => {}; f();';
    const reports = runRule('no-extra-arguments', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the function has a rest parameter', () => {
    const src = 'const f = (...args) => {}; f(1, 2, 3);';
    const reports = runRule('no-extra-arguments', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the call contains a spread argument', () => {
    const src = 'const f = (a) => {}; f(...arr);';
    const reports = runRule('no-extra-arguments', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the function body references the arguments object', () => {
    const src = 'const f = function(a){ return arguments.length; }; f(1, 2);';
    const reports = runRule('no-extra-arguments', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the callee is an unresolved identifier', () => {
    const src = 'g(1, 2);';
    const reports = runRule('no-extra-arguments', src);
    expect(reports).toHaveLength(0);
  });

  it('reports no-extra-arguments through the CLI', () => {
    const src = 'const f = function(a){}; f(1, 2);';
    const result = runOxlint('no-extra-arguments', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-extra-arguments)');
  });
});

describe('link-with-target-blank rule', () => {
  it('reports <a target="_blank"> with no rel attribute', () => {
    const src = '<a target="_blank">link</a>';
    const reports = runRule('link-with-target-blank', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('targetBlankNoOpener');
  });

  it('reports <a target="_blank"> with rel lacking noopener/noreferrer', () => {
    const src = '<a target="_blank" rel="nofollow">link</a>';
    const reports = runRule('link-with-target-blank', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('targetBlankNoOpener');
  });

  it('does not report <a target="_blank" rel="noopener">', () => {
    const src = '<a target="_blank" rel="noopener">link</a>';
    const reports = runRule('link-with-target-blank', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report <a target="_blank" rel="noreferrer">', () => {
    const src = '<a target="_blank" rel="noreferrer">link</a>';
    const reports = runRule('link-with-target-blank', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report <a> with no target attribute', () => {
    const src = '<a href="/x">link</a>';
    const reports = runRule('link-with-target-blank', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report <a target="_self">', () => {
    const src = '<a target="_self">link</a>';
    const reports = runRule('link-with-target-blank', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report <a {...props} target="_blank"> (spread attribute)', () => {
    const src = '<a {...props} target="_blank">link</a>';
    const reports = runRule('link-with-target-blank', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report <a target="_blank" rel={dyn}> (dynamic rel)', () => {
    const src = '<a target="_blank" rel={dyn}>link</a>';
    const reports = runRule('link-with-target-blank', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('reports link-with-target-blank through the CLI', () => {
    const src = '<a target="_blank">link</a>';
    const result = runOxlint('link-with-target-blank', src, 'sample.tsx');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(link-with-target-blank)');
  });
});

describe('no-hardcoded-passwords rule', () => {
  it('reports a hardcoded password in a variable declaration', () => {
    const reports = runRule('no-hardcoded-passwords', "const password = 's3cr3t-value';");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('hardcodedPassword');
  });

  it('reports a hardcoded password in an object property', () => {
    const reports = runRule('no-hardcoded-passwords', "const config = { password: 'hunter2abc' };");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('hardcodedPassword');
  });

  it('reports a hardcoded password in a member assignment', () => {
    const reports = runRule('no-hardcoded-passwords', "obj.passwd = 'realSecret123';");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('hardcodedPassword');
  });

  it('reports a hardcoded passphrase in a variable declaration', () => {
    const reports = runRule('no-hardcoded-passwords', "const passphrase = 'my-secret-phrase!';");
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('hardcodedPassword');
  });

  it('does not report an empty string value', () => {
    const reports = runRule('no-hardcoded-passwords', "const password = '';");
    expect(reports).toHaveLength(0);
  });

  it('does not report when the value equals the target name', () => {
    const reports = runRule('no-hardcoded-passwords', "const password = 'password';");
    expect(reports).toHaveLength(0);
  });

  it('does not report a non-credential identifier', () => {
    const reports = runRule('no-hardcoded-passwords', "const username = 'admin';");
    expect(reports).toHaveLength(0);
  });

  it('does not report when the init is not a string literal', () => {
    const reports = runRule('no-hardcoded-passwords', 'const password = getSecret();');
    expect(reports).toHaveLength(0);
  });

  it('does not report an identifier whose name only contains the word', () => {
    const reports = runRule('no-hardcoded-passwords', "const passwordHint = 'some hint text';");
    expect(reports).toHaveLength(0);
  });

  it('does not report a well-known placeholder value', () => {
    const reports = runRule('no-hardcoded-passwords', "const pwd = 'changeit';");
    expect(reports).toHaveLength(0);
  });

  it('reports no-hardcoded-passwords through the CLI', () => {
    const result = runOxlint('no-hardcoded-passwords', "const password = 's3cr3t-value';");
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-hardcoded-passwords)');
  });
});

describe('no-ignored-exceptions rule', () => {
  it('reports an empty catch block with a binding parameter', () => {
    const src = 'try { foo(); } catch (e) {}';
    const reports = runRule('no-ignored-exceptions', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('ignoredException');
  });

  it('reports an empty catch block with optional binding (no parameter)', () => {
    const src = 'try { foo(); } catch {}';
    const reports = runRule('no-ignored-exceptions', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('ignoredException');
  });

  it('does not report a catch block that logs the exception', () => {
    const src = 'try { foo(); } catch (e) { log(e); }';
    const reports = runRule('no-ignored-exceptions', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report an empty catch block that contains a comment', () => {
    const src = 'try { foo(); } catch (e) { /* ignore on purpose */ }';
    const reports = runRule('no-ignored-exceptions', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report a catch block that rethrows the exception', () => {
    const src = 'try { foo(); } catch (e) { throw e; }';
    const reports = runRule('no-ignored-exceptions', src);
    expect(reports).toHaveLength(0);
  });

  it('reports no-ignored-exceptions through the CLI', () => {
    const result = runOxlint('no-ignored-exceptions', 'try { foo(); } catch (e) {}');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-ignored-exceptions)');
  });
});

describe('no-unused-function-argument rule', () => {
  it('reports a trailing unused parameter in a function declaration', () => {
    const src = 'function f(a, b) { return a; }';
    const reports = runRule('no-unused-function-argument', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('unusedFunctionArgument');
  });

  it('reports a trailing unused parameter in an arrow function', () => {
    const src = 'const g = (x, y, z) => x + y;';
    const reports = runRule('no-unused-function-argument', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('unusedFunctionArgument');
  });

  it('does not report when all parameters are used', () => {
    const src = 'function f(a, b) { return a + b; }';
    const reports = runRule('no-unused-function-argument', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report an earlier unused param when the trailing one is used', () => {
    const src = 'function f(a, b) { return b; }';
    const reports = runRule('no-unused-function-argument', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report an underscore-prefixed parameter', () => {
    const src = 'function f(_unused) {}';
    const reports = runRule('no-unused-function-argument', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the param is used inside a nested function', () => {
    const src = 'function f(a) { return inner(); function inner() { return a; } }';
    const reports = runRule('no-unused-function-argument', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report a function with a rest parameter', () => {
    const src = 'function f(a, ...rest) {}';
    const reports = runRule('no-unused-function-argument', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report a function with a destructuring parameter', () => {
    const src = 'function f({ x }) {}';
    const reports = runRule('no-unused-function-argument', src);
    expect(reports).toHaveLength(0);
  });

  it('reports no-unused-function-argument through the CLI', () => {
    const src = 'function f(a, b) { return a; }';
    const result = runOxlint('no-unused-function-argument', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-unused-function-argument)');
  });
});

describe('no-weak-cipher rule', () => {
  it('reports DES cipher creation through the adapter', () => {
    const reports = runRule(
      'no-weak-cipher',
      'const c = crypto.createCipheriv("des-cbc", key, iv);',
    );
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('weakCipher');
  });

  it('reports RC4 cipher creation through the adapter', () => {
    const reports = runRule('no-weak-cipher', 'const c = createCipher("rc4", password);');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('weakCipher');
  });

  it('does not report a modern authenticated cipher', () => {
    const reports = runRule(
      'no-weak-cipher',
      'const c = crypto.createCipheriv("aes-256-gcm", key, iv);',
    );
    expect(reports).toHaveLength(0);
  });

  it('does not report dynamic cipher algorithms', () => {
    const reports = runRule(
      'no-weak-cipher',
      'const c = crypto.createCipheriv(algorithm, key, iv);',
    );
    expect(reports).toHaveLength(0);
  });

  it('reports no-weak-cipher through the CLI', () => {
    const result = runOxlint('no-weak-cipher', 'const c = crypto.createCipheriv("des", k, i);');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-weak-cipher)');
  });
});

describe('object-alt-content rule', () => {
  it('reports a self-closing <object> element with no attributes', () => {
    const src = '<object data="video.swf" />';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('objectAltContent');
  });

  it('reports an <object> element with empty children', () => {
    const src = '<object data="video.swf"></object>';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('objectAltContent');
  });

  it('reports an <object> element with whitespace-only text child', () => {
    const src = '<object data="video.swf">   </object>';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('objectAltContent');
  });

  it('does not report an <object> element with meaningful text child', () => {
    const src = '<object data="video.swf">Fallback text for assistive technologies.</object>';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report an <object> element with a child element', () => {
    const src = '<object data="video.swf"><img src="fallback.png" alt="Embedded video" /></object>';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report an <object> element with a child expression', () => {
    const src = '<object data={src}>{fallback}</object>';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report an <object> element with aria-label', () => {
    const src = '<object data="video.swf" aria-label="Embedded video" />';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report an <object> element with aria-labelledby', () => {
    const src = '<object data="video.swf" aria-labelledby="label-id" />';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report an <object> element with a title attribute', () => {
    const src = '<object data="video.swf" title="Embedded video" />';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report an <object> element with aria-hidden="true"', () => {
    const src = '<object data="video.swf" aria-hidden="true" />';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report an <object> element with a spread attribute', () => {
    const src = '<object {...props} />';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('does not report a non-object JSX element', () => {
    const src = '<video src="clip.mp4" />';
    const reports = runRule('object-alt-content', src, { filename: 'sample.tsx' });
    expect(reports).toHaveLength(0);
  });

  it('reports object-alt-content through the CLI', () => {
    const src = '<object data="video.swf" />';
    const result = runOxlint('object-alt-content', src, 'sample.tsx');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(object-alt-content)');
  });
});

describe('no-use-of-empty-return-value rule', () => {
  it('reports when a void function result is assigned to a variable', () => {
    const src = 'function voidFn() { console.log("x"); } const x = voidFn();';
    const reports = runRule('no-use-of-empty-return-value', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('useOfEmptyReturnValue');
  });

  it('reports when a void function result is assigned via plain assignment', () => {
    const src = 'function voidFn() {} let x; x = voidFn();';
    const reports = runRule('no-use-of-empty-return-value', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('useOfEmptyReturnValue');
  });

  it('reports when a void function result is returned from another function', () => {
    const src = 'function voidFn() {} function outer() { return voidFn(); }';
    const reports = runRule('no-use-of-empty-return-value', src);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('useOfEmptyReturnValue');
  });

  it('does not report when the void function is called as a bare statement', () => {
    const src = 'function voidFn() {} voidFn();';
    const reports = runRule('no-use-of-empty-return-value', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the called function does return a value', () => {
    const src = 'function valued() { return 42; } const x = valued();';
    const reports = runRule('no-use-of-empty-return-value', src);
    expect(reports).toHaveLength(0);
  });

  it('does not report an async function whose result is assigned', () => {
    const src = 'async function asyncFn() {} const p = asyncFn();';
    const reports = runRule('no-use-of-empty-return-value', src);
    expect(reports).toHaveLength(0);
  });

  it('reports no-use-of-empty-return-value through the CLI', () => {
    const src = 'function voidFn() { console.log("x"); } const x = voidFn();';
    const result = runOxlint('no-use-of-empty-return-value', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-use-of-empty-return-value)');
  });
});

describe('no-duplicated-branches rule', () => {
  it('reports an else branch identical to the if branch', () => {
    const source = 'if (a) { doWork(); } else { doWork(); }';
    const reports = runRule('no-duplicated-branches', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('duplicatedBranch');
  });

  it('reports the duplicate else-if branch in a three-branch chain', () => {
    const source = 'if (a) { doWork(); } else if (b) { other(); } else if (c) { doWork(); }';
    const reports = runRule('no-duplicated-branches', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('duplicatedBranch');
  });

  it('does not report when all branches differ', () => {
    const source = 'if (a) { one(); } else if (b) { two(); } else { three(); }';
    const reports = runRule('no-duplicated-branches', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a chain with only the if branch and no else', () => {
    const source = 'if (a) { doWork(); }';
    const reports = runRule('no-duplicated-branches', source);
    expect(reports).toHaveLength(0);
  });

  it('reports a duplicate case in a switch statement', () => {
    const source = 'switch (x) { case 1: doWork(); break; case 2: doWork(); break; }';
    const reports = runRule('no-duplicated-branches', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('duplicatedBranch');
  });

  it('does not report a switch where all case bodies differ', () => {
    const source = 'switch (x) { case 1: one(); break; case 2: two(); break; }';
    const reports = runRule('no-duplicated-branches', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report fall-through cases with empty consequents', () => {
    const source = 'switch (x) { case 1: case 2: doWork(); break; }';
    const reports = runRule('no-duplicated-branches', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-duplicated-branches through the CLI', () => {
    const source = 'if (a) { doWork(); } else { doWork(); }';
    const result = runOxlint('no-duplicated-branches', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-duplicated-branches)');
  });
});

describe('block-scoped-var rule', () => {
  it('reports a var declared inside an if-block and used after it', () => {
    const source = 'function f(c) { if (c) { var x = 1; } return x; }';
    const reports = runRule('block-scoped-var', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('blockScopedVar');
  });

  it('does not report a var used only inside the block where it is declared', () => {
    const source = 'function f(c) { if (c) { var x = 1; return x; } }';
    const reports = runRule('block-scoped-var', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a var declared at function top level', () => {
    const source = 'function f() { var x = 1; return x; }';
    const reports = runRule('block-scoped-var', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report let or const declared inside a block', () => {
    const source = 'function f(c) { if (c) { let y = 1; } }';
    const reports = runRule('block-scoped-var', source);
    expect(reports).toHaveLength(0);
  });

  it('reports block-scoped-var through the CLI', () => {
    const source = 'function f(c) { if (c) { var x = 1; } return x; }';
    const result = runOxlint('block-scoped-var', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(block-scoped-var)');
  });
});

describe('no-variable-usage-before-declaration rule', () => {
  it('reports a var variable used before its declaration at module level', () => {
    const source = 'console.log(x); var x = 5;';
    const reports = runRule('no-variable-usage-before-declaration', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('usedBeforeDeclaration');
  });

  it('reports a let variable used before its declaration at module level', () => {
    const source = 'console.log(y); let y = 10;';
    const reports = runRule('no-variable-usage-before-declaration', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('usedBeforeDeclaration');
  });

  it('reports a var variable used before its declaration inside a function', () => {
    const source = 'function f() { console.log(z); var z = 1; }';
    const reports = runRule('no-variable-usage-before-declaration', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('usedBeforeDeclaration');
  });

  it('does not report a variable used after its declaration', () => {
    const source = 'var x = 5; console.log(x);';
    const reports = runRule('no-variable-usage-before-declaration', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a function called before its declaration', () => {
    const source = 'foo(); function foo() {}';
    const reports = runRule('no-variable-usage-before-declaration', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a reference inside a nested function defined before the var', () => {
    const source = 'function outer() { function cb() { console.log(val); } var val = 3; cb(); }';
    const reports = runRule('no-variable-usage-before-declaration', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a reference inside an arrow defined before the var', () => {
    const source = 'function outer() { const cb = () => console.log(v); var v = 7; cb(); }';
    const reports = runRule('no-variable-usage-before-declaration', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-variable-usage-before-declaration through the CLI', () => {
    const source = 'console.log(x); var x = 5;';
    const result = runOxlint('no-variable-usage-before-declaration', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-variable-usage-before-declaration)');
  });
});

describe('arguments-order rule', () => {
  it('reports swapped arguments matching parameter names', () => {
    const source = 'function f(a, b) {} const a = 1, b = 2; f(b, a);';
    const reports = runRule('arguments-order', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('argumentsOrder');
  });

  it('does not report when arguments are in the correct order', () => {
    const source = 'function f(a, b) {} const a = 1, b = 2; f(a, b);';
    const reports = runRule('arguments-order', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when argument names do not match parameter names', () => {
    const source = 'function f(a, b) {} const x = 1, y = 2; f(x, y);';
    const reports = runRule('arguments-order', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a single-argument call', () => {
    const source = 'function f(a) {} const a = 1; f(a);';
    const reports = runRule('arguments-order', source);
    expect(reports).toHaveLength(0);
  });

  it('reports arguments-order through the CLI', () => {
    const source = 'function f(a, b) {} const a = 1, b = 2; f(b, a);';
    const result = runOxlint('arguments-order', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(arguments-order)');
  });
});

describe('updated-const-var rule', () => {
  it('reports simple and compound assignments to const bindings', () => {
    const source = 'const x = 1, y = 2; x = 3; y += 4;';
    const reports = runRule('updated-const-var', source);
    expect(reports).toHaveLength(2);
    expect(reports.map((report) => report.messageId)).toEqual(['updateConst', 'updateConst']);
  });

  it('reports update expressions against const bindings', () => {
    const reports = runRule('updated-const-var', 'const x = 1; ++x; x++;');
    expect(reports).toHaveLength(2);
    expect(reports.map((report) => report.messageId)).toEqual(['updateConst', 'updateConst']);
  });

  it('reports destructuring and for-in/of assignment targets', () => {
    const source = 'const x = 1, y = 2, z = 3; ({ x } = obj); [y] = values; for (z of values) {}';
    const reports = runRule('updated-const-var', source);
    expect(reports).toHaveLength(3);
    expect(reports.map((report) => report.messageId)).toEqual([
      'updateConst',
      'updateConst',
      'updateConst',
    ]);
  });

  it('does not report let/var assignments, property writes, or shadowed names', () => {
    const source = [
      'let x = 1; x = 2;',
      'var y = 1; y++;',
      'const obj = {}; obj.x = 1;',
      'const z = 1; function f(z) { z = 2; }',
    ].join('\\n');
    const reports = runRule('updated-const-var', source);
    expect(reports).toHaveLength(0);
  });

  it('reports updated-const-var through the CLI', () => {
    const result = runOxlint('updated-const-var', 'const x = 1; x = 2;');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(updated-const-var)');
  });
});

describe('unicode-aware-regex rule', () => {
  it('reports a \\p{...} property escape without u flag through the adapter', () => {
    const source = 'const r = /\\p{Letter}/;';
    const reports = runRule('unicode-aware-regex', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('unicodeAwareRegex');
  });

  it('reports unicode-aware-regex through the CLI', () => {
    const result = runOxlint('unicode-aware-regex', 'const r = /\\p{Letter}/;');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(unicode-aware-regex)');
  });
});

describe('no-undefined-assignment rule', () => {
  it('reports a plain variable assigned undefined', () => {
    const source = 'x = undefined;';
    const reports = runRule('no-undefined-assignment', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noUndefinedAssignment');
  });

  it('reports a property assigned undefined', () => {
    const source = 'obj.prop = undefined;';
    const reports = runRule('no-undefined-assignment', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noUndefinedAssignment');
  });

  it('does not report assignment of null', () => {
    const source = 'x = null;';
    const reports = runRule('no-undefined-assignment', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report assignment of void 0', () => {
    const source = 'x = void 0;';
    const reports = runRule('no-undefined-assignment', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-undefined-assignment through the CLI', () => {
    const source = 'x = undefined;';
    const result = runOxlint('no-undefined-assignment', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-undefined-assignment)');
  });
});

describe('no-empty-after-reluctant rule', () => {
  it('reports a lazy star with nothing following', () => {
    const source = 'const r = /a*?/;';
    const reports = runRule('no-empty-after-reluctant', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyAfterReluctant');
  });

  it('reports a lazy star followed by an end-of-input boundary', () => {
    const source = 'const r = /a*?$/;';
    const reports = runRule('no-empty-after-reluctant', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyAfterReluctant');
  });

  it('does not report a lazy star followed by a required character', () => {
    const source = 'const r = /a*?b/;';
    const reports = runRule('no-empty-after-reluctant', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a greedy star', () => {
    const source = 'const r = /a*/;';
    const reports = runRule('no-empty-after-reluctant', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a lazy plus (min == 1)', () => {
    const source = 'const r = /a+?/;';
    const reports = runRule('no-empty-after-reluctant', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-empty-after-reluctant through the CLI', () => {
    const source = 'const r = /a*?/;';
    const result = runOxlint('no-empty-after-reluctant', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-empty-after-reluctant)');
  });
});

describe('no-ignored-return rule', () => {
  it('reports a string literal trim() call whose result is discarded', () => {
    const source = '"hello".trim();';
    const reports = runRule('no-ignored-return', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('ignoredReturn');
  });

  it('does not report when the return value is used', () => {
    const source = 'const s = "hello".trim();';
    const reports = runRule('no-ignored-return', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a non-literal receiver', () => {
    const source = 'foo.trim();';
    const reports = runRule('no-ignored-return', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-ignored-return through the CLI', () => {
    const source = '"hello".trim();';
    const result = runOxlint('no-ignored-return', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-ignored-return)');
  });
});

describe('file-name-differ-from-class rule', () => {
  it('reports when the exported class name does not match the filename stem', () => {
    const source = 'export class Foo {}';
    const reports = runRule('file-name-differ-from-class', source, { filename: 'bar.ts' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('fileNameDifferFromClass');
  });

  it('does not report when the class name matches the stem', () => {
    const source = 'export class Foo {}';
    const reports = runRule('file-name-differ-from-class', source, { filename: 'foo.ts' });
    expect(reports).toHaveLength(0);
  });

  it('does not report when PascalCase class matches a kebab-case stem', () => {
    const source = 'export class MyClass {}';
    const reports = runRule('file-name-differ-from-class', source, { filename: 'my-class.ts' });
    expect(reports).toHaveLength(0);
  });

  it('does not report when there is no exported class', () => {
    const source = 'class Foo {} export {};';
    const reports = runRule('file-name-differ-from-class', source, { filename: 'bar.ts' });
    expect(reports).toHaveLength(0);
  });

  it('reports file-name-differ-from-class through the CLI', () => {
    const source = 'export class Foo {}';
    const result = runOxlint('file-name-differ-from-class', source, 'bar.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(file-name-differ-from-class)');
  });
});

describe('no-unenclosed-multiline-block rule', () => {
  it('reports a sibling statement indented as if inside an unbraced if body', () => {
    const source = 'if (c)\n  a();\n  b();';
    const reports = runRule('no-unenclosed-multiline-block', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('unenclosedMultilineBlock');
  });

  it('does not report when the body is braced', () => {
    const source = 'if (c) {\n  a();\n  b();\n}';
    const reports = runRule('no-unenclosed-multiline-block', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the sibling is at the outer indentation level', () => {
    const source = 'if (c)\n  a();\nb();';
    const reports = runRule('no-unenclosed-multiline-block', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-unenclosed-multiline-block through the CLI', () => {
    const source = 'if (c)\n  a();\n  b();';
    const result = runOxlint('no-unenclosed-multiline-block', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-unenclosed-multiline-block)');
  });
});

describe('inconsistent-function-call rule', () => {
  it('reports a function called both as plain call and constructor', () => {
    const source = 'function f() {} f(); new f();';
    const reports = runRule('inconsistent-function-call', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('inconsistentFunctionCall');
  });

  it('does not report a function called only as a plain call', () => {
    const source = 'function f() {} f(); f();';
    const reports = runRule('inconsistent-function-call', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a function called only as a constructor', () => {
    const source = 'function f() {} new f(); new f();';
    const reports = runRule('inconsistent-function-call', source);
    expect(reports).toHaveLength(0);
  });

  it('reports inconsistent-function-call through the CLI', () => {
    const source = 'function f() {} f(); new f();';
    const result = runOxlint('inconsistent-function-call', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(inconsistent-function-call)');
  });
});

describe('new-operator-misuse rule', () => {
  it('reports new on an inline arrow function', () => {
    const source = 'new (() => {})();';
    const reports = runRule('new-operator-misuse', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('newOperatorMisuse');
  });

  it('reports new on an identifier resolving to an arrow function', () => {
    const source = 'const f = () => {};\nnew f();';
    const reports = runRule('new-operator-misuse', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('newOperatorMisuse');
  });

  it('does not report new on a regular function', () => {
    const source = 'function F() {}\nnew F();';
    const reports = runRule('new-operator-misuse', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report new on a class', () => {
    const source = 'class C {}\nnew C();';
    const reports = runRule('new-operator-misuse', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report new on an unresolved identifier', () => {
    const source = 'new Foo();';
    const reports = runRule('new-operator-misuse', source);
    expect(reports).toHaveLength(0);
  });

  it('reports new-operator-misuse through the CLI', () => {
    const source = 'const f = () => {};\nnew f();';
    const result = runOxlint('new-operator-misuse', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(new-operator-misuse)');
  });
});

describe('no-empty-test-file rule', () => {
  it('reports a test file with no it/test calls', () => {
    const source = "import {x} from './x';";
    const reports = runRule('no-empty-test-file', source, { filename: 'foo.test.ts' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyTestFile');
  });

  it('reports a spec file with only describe and no it/test calls', () => {
    const source = "describe('x', () => {});";
    const reports = runRule('no-empty-test-file', source, { filename: 'a.spec.ts' });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyTestFile');
  });

  it('does not report when it() is present in the test file', () => {
    const source = "it('works', () => {});";
    const reports = runRule('no-empty-test-file', source, { filename: 'foo.test.ts' });
    expect(reports).toHaveLength(0);
  });

  it('does not report for a non-test filename', () => {
    const source = "import {x} from './x';";
    const reports = runRule('no-empty-test-file', source, { filename: 'foo.ts' });
    expect(reports).toHaveLength(0);
  });

  it('reports no-empty-test-file through the CLI', () => {
    const source = "import {x} from './x';";
    const result = runOxlint('no-empty-test-file', source, 'foo.test.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-empty-test-file)');
  });
});

describe('deprecation rule', () => {
  it('reports a call to a locally-declared deprecated function', () => {
    const source = '/** @deprecated */ function old() {} old();';
    const reports = runRule('deprecation', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('deprecatedUse');
  });

  it('does not report a call to a function without a @deprecated block comment', () => {
    const source = 'function modern() {} modern();';
    const reports = runRule('deprecation', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the deprecated function is never called', () => {
    const source = '/** @deprecated */ function old() {}';
    const reports = runRule('deprecation', source);
    expect(reports).toHaveLength(0);
  });

  it('reports deprecation through the CLI', () => {
    const source = '/** @deprecated */ function old() {} old();';
    const result = runOxlint('deprecation', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(deprecation)');
  });
});

describe('cognitive-complexity rule', () => {
  it('reports cognitive-complexity through the adapter when score exceeds threshold', () => {
    // if(a&&b){} → 2 > threshold 1 → report
    const source = 'function f(a,b){ if(a&&b){} }';
    const reports = runRule('cognitive-complexity', source, { options: [{ threshold: 1 }] });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('cognitiveComplexity');
  });

  it('does not report cognitive-complexity when score is within threshold', () => {
    // if(a&&b){} → 2; threshold 2: 2 is not > 2 → no report
    const source = 'function f(a,b){ if(a&&b){} }';
    const reports = runRule('cognitive-complexity', source, { options: [{ threshold: 2 }] });
    expect(reports).toHaveLength(0);
  });

  it('reports cognitive-complexity through the CLI', () => {
    // 16 if statements → 16 > default threshold 15 → reported
    const src =
      'function f(a,b,c,d,e,g,h,i,j,k,l,m,n,o,p,q)' +
      '{if(a){}if(b){}if(c){}if(d){}if(e){}if(g){}if(h){}if(i){}' +
      'if(j){}if(k){}if(l){}if(m){}if(n){}if(o){}if(p){}if(q){}}';
    const result = runOxlint('cognitive-complexity', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(cognitive-complexity)');
  });

  it('exposes the cognitive-complexity threshold option in the rule schema', () => {
    expect(plugin.rules['cognitive-complexity'].meta.schema).toEqual([
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ]);
  });
});

describe('expression-complexity rule', () => {
  it('reports expression-complexity when expression exceeds the default threshold of 3', () => {
    // 4 logical && operators: a&&b&&c&&d&&e → 4 > 3 → 1 report
    const source = 'const x = a && b && c && d && e;';
    const reports = runRule('expression-complexity', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('expressionComplexity');
  });

  it('does not report expression-complexity when expression is at the threshold', () => {
    // 3 logical && operators: a&&b&&c&&d → 3 is not > 3 → 0 reports
    const source = 'const x = a && b && c && d;';
    const reports = runRule('expression-complexity', source);
    expect(reports).toHaveLength(0);
  });

  it('reports expression-complexity through the adapter with a custom threshold', () => {
    // 3 operators > threshold 2 → 1 report
    const source = 'const x = a && b && c && d;';
    const reports = runRule('expression-complexity', source, { options: [{ threshold: 2 }] });
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('expressionComplexity');
  });

  it('reports expression-complexity through the CLI', () => {
    // 4 logical && operators → 4 > default threshold 3 → reported by CLI
    const src = 'const x = a && b && c && d && e;';
    const result = runOxlint('expression-complexity', src);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(expression-complexity)');
  });

  it('exposes the expression-complexity threshold option in the rule schema', () => {
    expect(plugin.rules['expression-complexity'].meta.schema).toEqual([
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ]);
  });
});

describe('prefer-regexp-exec rule', () => {
  it('reports String#match with a non-global RegExp literal through the adapter', () => {
    const reports = runRule('prefer-regexp-exec', 'const result = str.match(/foo/u);');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferRegExpExec');
  });

  it('reports a member receiver with a non-global RegExp literal', () => {
    const reports = runRule('prefer-regexp-exec', 'const result = object.value.match(/bar/);');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferRegExpExec');
  });

  it('does not report global RegExp literals', () => {
    const reports = runRule('prefer-regexp-exec', 'const result = str.match(/foo/g);');
    expect(reports).toHaveLength(0);
  });

  it('does not report dynamic match arguments', () => {
    const reports = runRule('prefer-regexp-exec', 'const result = str.match(pattern);');
    expect(reports).toHaveLength(0);
  });

  it('reports prefer-regexp-exec through the CLI', () => {
    const result = runOxlint('prefer-regexp-exec', 'const result = str.match(/foo/u);');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(prefer-regexp-exec)');
  });
});

describe('no-fallthrough rule', () => {
  it('reports a switch case that falls into the next case', () => {
    const source = 'switch (x) { case 1: doWork(); case 2: done(); break; }';
    const reports = runRule('no-fallthrough', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('noFallthrough');
  });

  it('does not report a case that ends with break', () => {
    const source = 'switch (x) { case 1: doWork(); break; case 2: done(); }';
    const reports = runRule('no-fallthrough', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an intentional fallthrough comment', () => {
    const source = 'switch (x) { case 1: doWork(); // fall through\ncase 2: done(); }';
    const reports = runRule('no-fallthrough', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-fallthrough through the CLI', () => {
    const source = 'switch (x) { case 1: doWork(); case 2: done(); break; }';
    const result = runOxlint('no-fallthrough', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-fallthrough)');
  });
});

describe('no-commented-code rule', () => {
  it('reports no-commented-code for a line comment with a variable declaration through the adapter', () => {
    const source = '// const x = 1;';
    const reports = runRule('no-commented-code', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('commentedCode');
  });

  it('reports no-commented-code for a block comment with an if statement through the adapter', () => {
    const source = '/* if (cond) { doSomething(); } */';
    const reports = runRule('no-commented-code', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('commentedCode');
  });

  it('does not report no-commented-code for a plain prose line comment through the adapter', () => {
    const source = '// This returns the user name';
    const reports = runRule('no-commented-code', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-commented-code through the CLI', () => {
    const source = '// const x = 1;';
    const result = runOxlint('no-commented-code', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-commented-code)');
  });
});

describe('no-incomplete-assertions rule', () => {
  it('reports a bare expect(x) statement as incomplete', () => {
    const source = 'expect(x);';
    const reports = runRule('no-incomplete-assertions', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('incompleteAssertion');
  });

  it('reports expect(x).to as a statement (language chain, no terminal)', () => {
    const source = 'expect(x).to;';
    const reports = runRule('no-incomplete-assertions', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('incompleteAssertion');
  });

  it('does not report expect(x).to.be.true (complete assertion)', () => {
    const source = 'expect(x).to.be.true;';
    const reports = runRule('no-incomplete-assertions', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report expect(x).to.equal(42) (method call is terminal)', () => {
    const source = 'expect(x).to.equal(42);';
    const reports = runRule('no-incomplete-assertions', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a non-expect call', () => {
    const source = 'foo(x);';
    const reports = runRule('no-incomplete-assertions', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-incomplete-assertions through the CLI', () => {
    const source = 'expect(x);';
    const result = runOxlint('no-incomplete-assertions', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-incomplete-assertions)');
  });
});

describe('destructuring-assignment-syntax rule', () => {
  it('reports the second consecutive extraction from the same object', () => {
    const source = 'const a = obj.a;\nconst b = obj.b;';
    const reports = runRule('destructuring-assignment-syntax', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('useDestructuring');
  });

  it('reports each declaration from the third onward in a longer group', () => {
    const source = 'const a = obj.a;\nconst b = obj.b;\nconst c = obj.c;';
    const reports = runRule('destructuring-assignment-syntax', source);
    expect(reports).toHaveLength(2);
    expect(reports[0].messageId).toBe('useDestructuring');
    expect(reports[1].messageId).toBe('useDestructuring');
  });

  it('does not report a lone extraction with no consecutive partner', () => {
    const source = 'const a = obj.a;';
    const reports = runRule('destructuring-assignment-syntax', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when binding name differs from property name', () => {
    const source = 'const x = obj.a;\nconst y = obj.b;';
    const reports = runRule('destructuring-assignment-syntax', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when source is a chained member expression', () => {
    const source = 'const a = foo.bar.a;\nconst b = foo.bar.b;';
    const reports = runRule('destructuring-assignment-syntax', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when extractions come from different objects', () => {
    const source = 'const a = foo.a;\nconst b = bar.b;';
    const reports = runRule('destructuring-assignment-syntax', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report non-consecutive matching declarations separated by another statement', () => {
    const source = 'const a = obj.a;\ndoSomething();\nconst b = obj.b;';
    const reports = runRule('destructuring-assignment-syntax', source);
    expect(reports).toHaveLength(0);
  });

  it('reports destructuring-assignment-syntax through the CLI', () => {
    const source = 'const a = obj.a;\nconst b = obj.b;';
    const result = runOxlint('destructuring-assignment-syntax', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(destructuring-assignment-syntax)');
  });
});

describe('no-element-overwrite rule', () => {
  it('reports the first of two consecutive writes to the same numeric-literal index', () => {
    const source = 'var a = [];\na[0] = 1;\na[0] = 2;';
    const reports = runRule('no-element-overwrite', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('elementOverwrite');
  });

  it('reports the first of two consecutive writes to the same string-literal key', () => {
    const source = 'var m = {};\nm["x"] = 1;\nm["x"] = 2;';
    const reports = runRule('no-element-overwrite', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('elementOverwrite');
  });

  it('reports the first of two consecutive writes to the same static property', () => {
    const source = 'var obj = {};\nobj.x = 1;\nobj.x = 2;';
    const reports = runRule('no-element-overwrite', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('elementOverwrite');
  });

  it('does not report writes to different indices', () => {
    const source = 'var a = [];\na[0] = 1;\na[1] = 2;';
    const reports = runRule('no-element-overwrite', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when an intervening statement separates the writes', () => {
    const source = 'var a = [];\na[0] = 1;\nfoo();\na[0] = 2;';
    const reports = runRule('no-element-overwrite', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a read-modify-write where the RHS references the element', () => {
    const source = 'var a = [];\na[0] = 1;\na[0] = a[0] + 1;';
    const reports = runRule('no-element-overwrite', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report writes to different static properties', () => {
    const source = 'var obj = {};\nobj.x = 1;\nobj.y = 2;';
    const reports = runRule('no-element-overwrite', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-element-overwrite through the CLI', () => {
    const source = 'var a = [];\na[0] = 1;\na[0] = 2;';
    const result = runOxlint('no-element-overwrite', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-element-overwrite)');
  });
});

describe('no-redundant-assignments rule', () => {
  it('reports a self-assignment', () => {
    const source = 'x = x;';
    const reports = runRule('no-redundant-assignments', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('redundantAssignment');
  });

  it('reports the first of two adjacent assignments to the same identifier', () => {
    const source = 'let y = 0;\ny = 1;\ny = 2;';
    const reports = runRule('no-redundant-assignments', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('redundantAssignment');
  });

  it('does not report a regular assignment to a different identifier', () => {
    const source = 'x = y;';
    const reports = runRule('no-redundant-assignments', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a read-modify-write', () => {
    const source = 'let x = 1;\nx = x + 1;';
    const reports = runRule('no-redundant-assignments', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when an intervening statement separates the assignments', () => {
    const source = 'let x = 1;\nfoo();\nx = 2;';
    const reports = runRule('no-redundant-assignments', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-redundant-assignments through the CLI', () => {
    const source = 'x = x;';
    const result = runOxlint('no-redundant-assignments', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-redundant-assignments)');
  });
});

describe('no-unused-collection rule', () => {
  it('reports an array that is only written to via push', () => {
    const source = 'const a = [];\na.push(1);\na.push(2);';
    const reports = runRule('no-unused-collection', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('unusedCollection');
  });

  it('reports a Map that is only written to via set', () => {
    const source = "const m = new Map();\nm.set('k', 1);";
    const reports = runRule('no-unused-collection', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('unusedCollection');
  });

  it('does not report when the array is returned', () => {
    const source = 'function f() { const a = [];\na.push(1);\nreturn a; }';
    const reports = runRule('no-unused-collection', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the array is passed to a function', () => {
    const source = 'const a = [];\na.push(1);\nconsole.log(a);';
    const reports = runRule('no-unused-collection', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the length property is read', () => {
    const source = 'const a = [];\na.push(1);\nconst b = a.length;';
    const reports = runRule('no-unused-collection', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-unused-collection through the CLI', () => {
    const source = 'const a = [];\na.push(1);\na.push(2);';
    const result = runOxlint('no-unused-collection', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-unused-collection)');
  });
});

describe('no-empty-collection rule', () => {
  it('reports an array that is read but never populated', () => {
    const source = 'const a = [];\nfunction f() { return a.length; }';
    const reports = runRule('no-empty-collection', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyCollection');
  });

  it('reports a Map that is queried but never populated', () => {
    const source = 'const m = new Map();\nfunction f(k) { return m.has(k); }';
    const reports = runRule('no-empty-collection', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('emptyCollection');
  });

  it('does not report when the array is populated via push', () => {
    const source = 'const a = [];\na.push(1);\nfunction f() { return a.length; }';
    const reports = runRule('no-empty-collection', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when the array is passed to a function', () => {
    const source = 'const a = [];\nfill(a);\nfunction f() { return a.length; }';
    const reports = runRule('no-empty-collection', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report object literals', () => {
    const source = 'const o = {};\nfunction f() { return o.x; }';
    const reports = runRule('no-empty-collection', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-empty-collection through the CLI', () => {
    const source = 'const a = [];\nfunction f() { return a.length; }';
    const result = runOxlint('no-empty-collection', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-empty-collection)');
  });
});

describe('no-redundant-parentheses rule', () => {
  it('reports nested double parentheses', () => {
    const source = 'const x = ((1));';
    const reports = runRule('no-redundant-parentheses', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('redundantParentheses');
  });

  it('reports twice for triple nesting', () => {
    const source = 'const z = (((a)));';
    const reports = runRule('no-redundant-parentheses', source);
    expect(reports).toHaveLength(2);
  });

  it('does not report a single pair', () => {
    const source = 'const x = (1);';
    const reports = runRule('no-redundant-parentheses', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report precedence grouping', () => {
    const source = 'const r = (a + b) * c;';
    const reports = runRule('no-redundant-parentheses', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-redundant-parentheses through the CLI', () => {
    const source = 'const x = ((1));';
    const result = runOxlint('no-redundant-parentheses', source);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-redundant-parentheses)');
  });
});

describe('bool-param-default rule', () => {
  it('reports an optional boolean function parameter without a default', () => {
    const source = 'function f(flag?: boolean) {}';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('boolParamDefault');
  });

  it('reports an optional boolean arrow-function parameter', () => {
    const source = 'const g = (flag?: boolean) => {};';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('boolParamDefault');
  });

  it('reports an optional boolean class-method parameter', () => {
    const source = 'class C { m(flag?: boolean) {} }';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('boolParamDefault');
  });

  it('does not report a required boolean parameter', () => {
    const source = 'function f(flag: boolean) {}';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an optional boolean parameter with a default', () => {
    const source = 'function f(flag: boolean = false) {}';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a union annotation', () => {
    const source = 'function f(flag?: boolean | undefined) {}';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an interface method signature', () => {
    const source = 'interface I { m(flag?: boolean): void; }';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a type-alias function type', () => {
    const source = 'type T = (flag?: boolean) => void;';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a type-literal method signature', () => {
    const source = 'type O = { m(flag?: boolean): void };';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an abstract method', () => {
    const source = 'abstract class C { abstract m(flag?: boolean): void; }';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an overload signature, only its implementation', () => {
    const source = 'function f(flag?: boolean): void; function f(flag?: boolean) { return flag; }';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('boolParamDefault');
  });

  it('does not report an ambient declared function', () => {
    const source = 'declare function f(flag?: boolean): void;';
    const reports = runRule('bool-param-default', source);
    expect(reports).toHaveLength(0);
  });

  it('reports bool-param-default through the CLI', () => {
    const source = 'function f(flag?: boolean) {}';
    const result = runOxlint('bool-param-default', source, 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(bool-param-default)');
  });
});

describe('post-message rule', () => {
  it('reports a cross-document message sent with the "*" wildcard target origin', () => {
    const source = 'win.postMessage(data, "*");';
    const reports = runRule('post-message', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('postMessage');
  });

  it('reports the wildcard origin regardless of the receiver type', () => {
    const source = 'el.postMessage(payload, "*");';
    const reports = runRule('post-message', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('postMessage');
  });

  it('does not report a specific target origin', () => {
    const source = 'win.postMessage(data, "https://example.com");';
    const reports = runRule('post-message', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a single-argument postMessage call', () => {
    const source = 'worker.postMessage(data);';
    const reports = runRule('post-message', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a variable target origin', () => {
    const source = 'win.postMessage(data, origin);';
    const reports = runRule('post-message', source);
    expect(reports).toHaveLength(0);
  });

  it('reports post-message through the CLI', () => {
    const source = 'win.postMessage(data, "*");';
    const result = runOxlint('post-message', source, 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(post-message)');
  });
});

describe('in-operator-type-error rule', () => {
  it('reports a string-literal right operand', () => {
    const source = 'const r = "a" in "s";';
    const reports = runRule('in-operator-type-error', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('inOperatorTypeError');
  });

  it('reports a numeric-literal right operand', () => {
    const source = 'const r = 0 in 5;';
    const reports = runRule('in-operator-type-error', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('inOperatorTypeError');
  });

  it('reports a null-literal right operand', () => {
    const source = 'const r = k in null;';
    const reports = runRule('in-operator-type-error', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('inOperatorTypeError');
  });

  it('does not report an identifier right operand', () => {
    const source = 'const r = "x" in obj;';
    const reports = runRule('in-operator-type-error', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an object-literal right operand', () => {
    const source = 'const r = "x" in {};';
    const reports = runRule('in-operator-type-error', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an array-literal right operand', () => {
    const source = 'const r = "x" in [];';
    const reports = runRule('in-operator-type-error', source);
    expect(reports).toHaveLength(0);
  });

  it('reports in-operator-type-error through the CLI', () => {
    const source = 'const r = "a" in "s";';
    const result = runOxlint('in-operator-type-error', source, 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(in-operator-type-error)');
  });
});

describe('different-types-comparison rule', () => {
  it('reports a strict comparison between a string and a number literal', () => {
    const source = '"a" === 1;';
    const reports = runRule('different-types-comparison', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('differentTypesComparison');
  });

  it('reports a strict comparison between null and a number literal', () => {
    const source = 'null === 0;';
    const reports = runRule('different-types-comparison', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('differentTypesComparison');
  });

  it('reports a strict inequality between a number and a string literal', () => {
    const source = '5 !== "5";';
    const reports = runRule('different-types-comparison', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('differentTypesComparison');
  });

  it('reports a strict comparison between a bigint and a number literal', () => {
    const source = '1n === 1;';
    const reports = runRule('different-types-comparison', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('differentTypesComparison');
  });

  it('does not report a same-kind literal comparison', () => {
    const source = '1 === 2;';
    const reports = runRule('different-types-comparison', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report when an operand is not a literal', () => {
    const source = 'x === 1;';
    const reports = runRule('different-types-comparison', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report loose equality between different-typed literals', () => {
    const source = '1 == "1";';
    const reports = runRule('different-types-comparison', source);
    expect(reports).toHaveLength(0);
  });

  it('reports different-types-comparison through the CLI', () => {
    const source = '"a" === 1;';
    const result = runOxlint('different-types-comparison', source, 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(different-types-comparison)');
  });
});

describe('operation-returning-nan rule', () => {
  it('reports an arrow function operand in a multiplication', () => {
    const source = 'const x = (() => {}) * 2;';
    const reports = runRule('operation-returning-nan', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('operationReturningNan');
  });

  it('reports a function expression operand in a subtraction', () => {
    const source = 'const x = (function(){}) - 1;';
    const reports = runRule('operation-returning-nan', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('operationReturningNan');
  });

  it('reports a plain object literal operand', () => {
    const source = 'const x = ({a:1}) / 2;';
    const reports = runRule('operation-returning-nan', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('operationReturningNan');
  });

  it('does not report an object literal with a custom valueOf', () => {
    const source = 'const x = ({valueOf(){return 5}}) * 2;';
    const reports = runRule('operation-returning-nan', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report an array operand', () => {
    const source = 'const x = [] * 2;';
    const reports = runRule('operation-returning-nan', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report the + operator', () => {
    const source = 'const x = "a" + {};';
    const reports = runRule('operation-returning-nan', source);
    expect(reports).toHaveLength(0);
  });

  it('reports operation-returning-nan through the CLI', () => {
    const source = 'const x = ({}) * 2;';
    const result = runOxlint('operation-returning-nan', source, 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(operation-returning-nan)');
  });
});

describe('production-debug rule', () => {
  it('reports a debugger statement inside a function body', () => {
    const source = 'function f() { debugger; }';
    const reports = runRule('production-debug', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('productionDebug');
  });

  it('reports a debugger statement nested in a block', () => {
    const source = 'if (x) { debugger; }';
    const reports = runRule('production-debug', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('productionDebug');
  });

  it('reports a top-level debugger statement', () => {
    const source = 'debugger;';
    const reports = runRule('production-debug', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('productionDebug');
  });

  it('does not report console.* calls', () => {
    const source = 'console.log(1); console.debug(2);';
    const reports = runRule('production-debug', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report alert/confirm/prompt calls', () => {
    const source = 'alert(1); confirm("x"); prompt("y");';
    const reports = runRule('production-debug', source);
    expect(reports).toHaveLength(0);
  });

  it('reports production-debug through the CLI', () => {
    const source = 'function f() { debugger; }';
    const result = runOxlint('production-debug', source, 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(production-debug)');
  });
});

describe('no-hardcoded-secrets rule', () => {
  it('reports a hardcoded secret assigned to an apiKey variable', () => {
    const source = 'const apiKey = "AKIA1234567890ABCD";';
    const reports = runRule('no-hardcoded-secrets', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('hardcodedSecret');
  });

  it('reports a hardcoded secret in an object property', () => {
    const source = 'const x = { secret: "s3cr3tVal" };';
    const reports = runRule('no-hardcoded-secrets', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('hardcodedSecret');
  });

  it('does not report a partial name match', () => {
    const source = 'const tokenizer = "someValueHere";';
    const reports = runRule('no-hardcoded-secrets', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a non-string-literal initializer', () => {
    const source = 'const apiKey = process.env.KEY;';
    const reports = runRule('no-hardcoded-secrets', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a well-known placeholder value', () => {
    const source = 'const apiKey = "token";';
    const reports = runRule('no-hardcoded-secrets', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-hardcoded-secrets through the CLI', () => {
    const source = 'const token = "ghp_realLongTokenValue123";';
    const result = runOxlint('no-hardcoded-secrets', source, 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-hardcoded-secrets)');
  });
});

describe('concise-regex rule', () => {
  it('reports a verbose [0-9] digit class', () => {
    const reports = runRule('concise-regex', 'const r = /[0-9]/;');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('conciseRegex');
  });

  it('reports a negated [^0-9] digit class', () => {
    const reports = runRule('concise-regex', 'const r = /[^0-9]/;');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('conciseRegex');
  });

  it('reports a verbose [A-Za-z0-9_] word class in any order', () => {
    const reports = runRule('concise-regex', 'const r = /[_0-9a-zA-Z]/;');
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('conciseRegex');
  });

  it('does not report a class with an extra member', () => {
    const reports = runRule('concise-regex', 'const r = /[0-9a]/;');
    expect(reports).toHaveLength(0);
  });

  it('does not report a word class missing the underscore', () => {
    const reports = runRule('concise-regex', 'const r = /[A-Za-z0-9]/;');
    expect(reports).toHaveLength(0);
  });

  it('does not report an already concise shorthand', () => {
    const reports = runRule('concise-regex', 'const r = /\\d/;');
    expect(reports).toHaveLength(0);
  });

  it('reports concise-regex through the CLI', () => {
    const result = runOxlint('concise-regex', 'const r = /[0-9]/;', 'sample.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(concise-regex)');
  });
});

describe('no-misleading-character-class rule', () => {
  it('reports an astral character inside a class without the u flag', () => {
    const source = 'const r = /[\u{1F44D}]/;';
    const reports = runRule('no-misleading-character-class', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('misleadingCharacterClass');
  });

  it('does not report when the u flag is present', () => {
    const source = 'const r = /[\u{1F44D}]/u;';
    const reports = runRule('no-misleading-character-class', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a BMP-only character class', () => {
    const source = 'const r = /[abc]/;';
    const reports = runRule('no-misleading-character-class', source);
    expect(reports).toHaveLength(0);
  });

  it('reports no-misleading-character-class through the CLI', () => {
    const result = runOxlint('no-misleading-character-class', 'const r = /[\u{1F44D}]/;');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(no-misleading-character-class)');
  });
});

describe('slow-regex rule', () => {
  it('reports an unbounded quantifier over a group with an unbounded quantifier', () => {
    const source = 'const r = /(a+)+/;';
    const reports = runRule('slow-regex', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('slowRegex');
  });

  it('reports the (.*)+ shape', () => {
    const source = 'const r = /(.*)+$/;';
    const reports = runRule('slow-regex', source);
    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('slowRegex');
  });

  it('does not report a single quantifier', () => {
    const source = 'const r = /a+/;';
    const reports = runRule('slow-regex', source);
    expect(reports).toHaveLength(0);
  });

  it('does not report a bounded outer quantifier', () => {
    const source = 'const r = /(a+){2,3}/;';
    const reports = runRule('slow-regex', source);
    expect(reports).toHaveLength(0);
  });

  it('reports slow-regex through the CLI', () => {
    const result = runOxlint('slow-regex', 'const r = /(a+)+/;');
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('sonarjs(slow-regex)');
  });
});
