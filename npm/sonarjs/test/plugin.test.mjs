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
});
