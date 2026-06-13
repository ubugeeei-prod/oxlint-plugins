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

function applyReportFix(sourceText, report) {
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

function runOxlint(ruleName, code, filename = 'sample.ts') {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'e18e-plugin-'));

  try {
    const sourcePath = join(temp, filename);
    const configPath = join(temp, 'oxlint.config.jsonc');

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'e18e',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`e18e/${ruleName}`]: 'error',
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

describe('e18e plugin shape', () => {
  it('exposes all rules and configs', () => {
    expect(plugin.meta?.name).toBe('e18e');
    expect(plugin.implementedE18eRuleNames).toHaveLength(25);
    expect(Object.keys(plugin.configs)).toEqual([
      'modernization',
      'module-replacements',
      'performance-improvements',
      'recommended',
    ]);
    expect(plugin.configs.recommended.rules['e18e/prefer-array-at']).toBe('error');
    expect(plugin.configs.recommended.rules['e18e/ban-dependencies']).toBe('error');
  });
});

describe('e18e rules through direct adapter harness', () => {
  it('reports and fixes prefer-array-from-map', () => {
    const code = 'const out = [...items].map(x => x.id);';
    const reports = runRule('prefer-array-from-map', code);

    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('preferArrayFrom');
    expect(applyReportFix(code, reports[0])).toBe('const out = Array.from(items, x => x.id);');
  });

  it('uses ban-dependencies options and allowed modules', () => {
    const code = 'import bad from "left-pad"; import ok from "lodash.merge";';
    const reports = runRule('ban-dependencies', code, {
      filename: 'sample.js',
      options: [{ modules: ['left-pad', 'lodash.merge'], allowed: ['lodash.merge'] }],
    });

    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('removalReplacement');
    expect(reports[0].data.name).toBe('left-pad');
  });
});

describe('e18e rules through oxlint jsPlugins', () => {
  it('reports prefer-object-has-own through the CLI', () => {
    const result = runOxlint(
      'prefer-object-has-own',
      'Object.prototype.hasOwnProperty.call(obj, key);',
    );

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('e18e(prefer-object-has-own)');
  });
});
