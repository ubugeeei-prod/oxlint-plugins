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
  readFileSync(join(packageRoot, 'test', 'fixtures', 'astro-v3.0.1.json'), 'utf8'),
);
const expectedRuleNames = [
  'no-deprecated-astro-canonicalurl',
  'no-deprecated-astro-fetchcontent',
  'no-deprecated-getentrybyslug',
];

function runRule(ruleName, sourceText, filename = 'fixture.astro') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[ruleName].createOnce({
    filename,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function renderedMessages(ruleName, reports) {
  return reports.map((report) => plugin.rules[ruleName].meta.messages[report.messageId]);
}

function applyReports(sourceText, reports) {
  const edits = [];
  for (const report of reports) {
    report.fix?.({
      replaceTextRange(range, replacementText) {
        edits.push({ range, replacementText });
      },
    });
  }
  return edits
    .sort((left, right) => right.range[0] - left.range[0])
    .reduce(
      (output, edit) =>
        output.slice(0, edit.range[0]) + edit.replacementText + output.slice(edit.range[1]),
      sourceText,
    );
}

function findOxlintCli() {
  const store = join(workspaceRoot, 'node_modules', '.pnpm');
  const candidates = readdirSync(store)
    .filter((entry) => entry.startsWith('oxlint@'))
    .map((entry) => join(store, entry, 'node_modules', 'oxlint', 'bin', 'oxlint'))
    .filter((candidate) => existsSync(candidate))
    .sort((left, right) => left.localeCompare(right));
  if (candidates.length === 0) {
    throw new Error('Could not find oxlint CLI in node_modules/.pnpm.');
  }
  return candidates[candidates.length - 1];
}

function runOxlint(ruleName, code, fix = false) {
  const temporary = mkdtempSync(join(tmpdir(), 'astro-plugin-'));
  try {
    const sourcePath = join(temporary, 'fixture.astro');
    const configPath = join(temporary, 'oxlint.config.jsonc');
    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [{ name: 'astro', specifier: join(packageRoot, 'index.js') }],
        rules: { [`astro/${ruleName}`]: 'error' },
      }),
    );
    const args = ['-c', configPath, '--quiet', '--format', 'json'];
    if (fix) args.push('--fix');
    args.push(sourcePath);
    const result = spawnSync(findOxlintCli(), args, { encoding: 'utf8' });
    const payload = result.stdout.trim() === '' ? { diagnostics: [] } : JSON.parse(result.stdout);
    return {
      diagnostics: payload.diagnostics ?? [],
      output: readFileSync(sourcePath, 'utf8'),
      status: result.status,
      stderr: result.stderr,
    };
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

describe('astro plugin shape', () => {
  it('exports only the selected Astro rules', () => {
    expect(plugin.meta?.name).toBe('astro');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.implementedAstroRuleNames).toEqual(expectedRuleNames);
    expect(typeof plugin.scanAstro).toBe('function');
  });

  it('enables all selected rules in recommended for Astro files', () => {
    expect(plugin.configs.recommended.files).toEqual(['**/*.astro']);
    expect(plugin.configs.recommended.rules).toEqual(
      Object.fromEntries(expectedRuleNames.map((name) => [`astro/${name}`, 'error'])),
    );
  });

  it('keeps every schema empty and every rule recommended', () => {
    for (const rule of Object.values(plugin.rules)) {
      expect(rule.meta.schema).toEqual([]);
      expect(rule.meta.docs.recommended).toBe(true);
      expect(rule.meta.type).toBe('problem');
    }
  });

  it('marks only fetchContent as fixable', () => {
    expect(plugin.rules['no-deprecated-astro-fetchcontent'].meta.fixable).toBe('code');
    expect(plugin.rules['no-deprecated-astro-canonicalurl'].meta.fixable).toBeUndefined();
    expect(plugin.rules['no-deprecated-getentrybyslug'].meta.fixable).toBeUndefined();
  });
});

describe('authored eslint-plugin-astro v3.0.1 fixtures through the adapter', () => {
  it.each(
    Object.entries(fixture.rules).flatMap(([ruleName, cases]) =>
      cases.valid.map((testCase) => [ruleName, testCase]),
    ),
  )('accepts %s valid fixture', (ruleName, testCase) => {
    expect(runRule(ruleName, testCase.code, testCase.filename)).toEqual([]);
  });

  it.each(
    Object.entries(fixture.rules).flatMap(([ruleName, cases]) =>
      cases.invalid.map((testCase) => [ruleName, testCase]),
    ),
  )('matches %s invalid fixture messages and locations', (ruleName, testCase) => {
    const reports = runRule(ruleName, testCase.code, testCase.filename);
    expect(renderedMessages(ruleName, reports)).toEqual(
      testCase.errors.map((error) => error.message),
    );
    expect(
      reports.map((report) => ({
        line: report.loc.start.line,
        column: report.loc.start.column + 1,
      })),
    ).toEqual(testCase.errors.map((error) => ({ line: error.line, column: error.column })));
  });

  it('matches the authored fetchContent output', () => {
    const testCase = fixture.rules['no-deprecated-astro-fetchcontent'].invalid[0];
    expect(
      applyReports(testCase.code, runRule('no-deprecated-astro-fetchcontent', testCase.code)),
    ).toBe(testCase.output);
  });
});

describe('adapter regression coverage', () => {
  it('maps native UTF-8 fix ranges to JavaScript UTF-16 ranges', () => {
    const code = '---\nconst emoji = "😀"; Astro.fetchContent("*.md")\n---\n';
    const reports = runRule('no-deprecated-astro-fetchcontent', code);
    expect(applyReports(code, reports)).toBe('---\nconst emoji = "😀"; Astro.glob("*.md")\n---\n');
  });

  it('does not report a shadowed Astro binding', () => {
    const code = '---\nconst Astro = { canonicalURL: "local" };\nAstro.canonicalURL\n---\n';
    expect(runRule('no-deprecated-astro-canonicalurl', code)).toEqual([]);
  });

  it('does not scan template expressions in the frontmatter slice', () => {
    expect(
      runRule(
        'no-deprecated-astro-canonicalurl',
        '---\nconst title = "page"\n---\n<p>{Astro.canonicalURL}</p>\n',
      ),
    ).toEqual([]);
  });

  it('does not report malformed frontmatter', () => {
    expect(
      runRule('no-deprecated-astro-canonicalurl', '---\nconst = Astro.canonicalURL\n---\n'),
    ).toEqual([]);
  });

  it('does not run against non-Astro filenames', () => {
    expect(
      runRule('no-deprecated-astro-canonicalurl', '---\nAstro.canonicalURL\n---\n', 'fixture.ts'),
    ).toEqual([]);
  });
});

describe('astro rules through real Oxlint jsPlugins', () => {
  it('reports an authored .astro invalid fixture through the CLI', () => {
    const testCase = fixture.rules['no-deprecated-getentrybyslug'].invalid[0];
    const result = runOxlint('no-deprecated-getentrybyslug', testCase.code);
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('astro(no-deprecated-getentrybyslug)');
  });

  it('fixes an authored .astro fixture through the CLI', () => {
    const testCase = fixture.rules['no-deprecated-astro-fetchcontent'].invalid[0];
    const result = runOxlint('no-deprecated-astro-fetchcontent', testCase.code, true);
    expect(result.status).toBe(0);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toEqual([]);
    expect(result.output).toBe(testCase.output);

    const rerun = runOxlint('no-deprecated-astro-fetchcontent', result.output, true);
    expect(rerun.status).toBe(0);
    expect(rerun.stderr).toBe('');
    expect(rerun.diagnostics).toEqual([]);
    expect(rerun.output).toBe(result.output);
  });

  it('keeps rule selection isolated through the CLI', () => {
    const code = '---\nAstro.fetchContent("*.md")\nAstro.canonicalURL\n---\n';
    const result = runOxlint('no-deprecated-astro-canonicalurl', code);
    expect(result.status).toBe(1);
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('astro(no-deprecated-astro-canonicalurl)');
  });
});
