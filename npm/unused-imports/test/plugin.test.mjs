import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

const expectedRuleNames = ['no-unused-imports', 'no-unused-vars'];

function runRule(ruleName, sourceText, filename = 'fixture.js') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[ruleName].createOnce({
    filename,
    options: [],
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function applyFix(sourceText, report) {
  const fix = report.fix({
    replaceTextRange(range, replacement) {
      return { range, text: replacement };
    },
  });
  return sourceText.slice(0, fix.range[0]) + fix.text + sourceText.slice(fix.range[1]);
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

function runOxlint(ruleName, code, filename = 'fixture.js') {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'unused-imports-plugin-'));

  try {
    const sourcePath = join(temp, filename);
    const configPath = join(temp, 'oxlint.config.jsonc');

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'unused-imports',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`unused-imports/${ruleName}`]: 'error',
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

describe('unused-imports plugin shape', () => {
  it('exposes all ported rules', () => {
    expect(plugin.meta?.name).toBe('unused-imports');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.implementedUnusedImportsRuleNames).toEqual(expectedRuleNames);
    expect(typeof plugin.scanUnusedImports).toBe('function');
  });

  it('ships recommended config', () => {
    expect(plugin.configs.recommended.rules).toEqual({
      'unused-imports/no-unused-imports': 'error',
      'unused-imports/no-unused-vars': 'off',
    });
  });
});

describe('unused-imports rules through direct adapter harness', () => {
  it('reports and fixes no-unused-imports', () => {
    const code = ['import { a, b } from "./utils";', 'console.log(b);'].join('\n');
    const reports = runRule('no-unused-imports', code);

    expect(reports).toHaveLength(1);
    expect(reports[0].data.message).toBe("'a' is defined but never used.");
    expect(applyFix(code, reports[0])).toBe(
      ['import { b } from "./utils";', 'console.log(b);'].join('\n'),
    );
  });

  it('reports no-unused-vars without a fixer', () => {
    const code = ['const used = 1;', 'const unused = 2;', 'console.log(used);'].join('\n');
    const reports = runRule('no-unused-vars', code);

    expect(reports).toHaveLength(1);
    expect(reports[0].data.message).toBe("'unused' is defined but never used.");
    expect(reports[0].fix).toBeUndefined();
  });
});

describe('unused-imports rules through oxlint jsPlugins', () => {
  it('reports no-unused-imports through the CLI', () => {
    const result = runOxlint(
      'no-unused-imports',
      ['import { a, b } from "./utils";', 'console.log(b);'].join('\n'),
    );

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('unused-imports(no-unused-imports)');
  });
});
