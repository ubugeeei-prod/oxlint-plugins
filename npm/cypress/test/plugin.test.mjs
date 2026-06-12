import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

const expectedRuleNames = [
  'assertion-before-screenshot',
  'no-and',
  'no-assigning-return-values',
  'no-async-before',
  'no-async-tests',
  'no-chained-get',
  'no-debug',
  'no-force',
  'no-pause',
  'no-unnecessary-waiting',
  'no-xpath',
  'require-data-selectors',
  'unsafe-to-chain-command',
];

const validCases = [
  [
    'assertion-before-screenshot',
    'assertion in previous statement',
    'cy.get("[data-cy=button]").should("be.visible");\ncy.screenshot();\n',
  ],
  [
    'assertion-before-screenshot',
    'assertion in chain',
    'cy.get("[data-cy=button]").screenshot();\n',
  ],
  ['no-and', 'and after should', 'cy.get("[data-cy=button]").should("exist").and("be.visible");\n'],
  ['no-and', 'and after contains', 'cy.contains("Submit").and("be.visible");\n'],
  ['no-assigning-return-values', 'allowed cy.stub assignment', 'const stub = cy.stub();\n'],
  [
    'no-assigning-return-values',
    'allowed cy.state assignment',
    'const state = cy.state("window");\n',
  ],
  [
    'no-async-before',
    'async before without Cypress usage',
    'beforeEach("setup", async () => {});\n',
  ],
  ['no-async-tests', 'async test without Cypress usage', 'it("works", async () => {});\n'],
  ['no-chained-get', 'single cy.get call', 'cy.get("[data-cy=button]").find("span");\n'],
  ['no-debug', 'debug on non-cy receiver', 'logger.debug();\n'],
  ['no-force', 'action without force option', 'cy.get("[data-cy=button]").click();\n'],
  ['no-pause', 'pause on non-cy receiver', 'player.pause();\n'],
  ['no-unnecessary-waiting', 'wait for alias', 'cy.wait("@saveUser");\n'],
  [
    'no-unnecessary-waiting',
    'wait for imported value',
    'import { WAIT } from "./constants";\ncy.wait(WAIT);\n',
  ],
  ['no-xpath', 'xpath on non-cy receiver', 'subject.xpath("//button");\n'],
  ['require-data-selectors', 'data selector string', 'cy.get("[data-cy=submit]");\n'],
  ['require-data-selectors', 'alias selector string', 'cy.get("@saveUser");\n'],
  [
    'require-data-selectors',
    'data selector variable',
    'const selector = "[data-cy=submit]";\ncy.get(selector);\n',
  ],
  [
    'require-data-selectors',
    'data selector conditional',
    'cy.get(active ? "[data-cy=active]" : "@inactive");\n',
  ],
  ['unsafe-to-chain-command', 'action at end of chain', 'cy.get("[data-cy=button]").click();\n'],
];

const invalidCases = [
  [
    'assertion-before-screenshot',
    'screenshot after non-assertion command',
    'cy.visit("/home");\ncy.screenshot();\n',
    ['Make an assertion on the page state before taking a screenshot'],
  ],
  [
    'no-and',
    'and starts assertion chain',
    'cy.get("é").and("be.visible");\n',
    ['Use .should() here; .and() is only allowed after .should(), .and(), or .contains().'],
  ],
  [
    'no-assigning-return-values',
    'assigns cy command',
    'const button = cy.get("[data-cy=button]");\n',
    ['Do not assign the return value of a Cypress command'],
  ],
  [
    'no-async-before',
    'async before contains cy',
    'beforeEach("setup", async () => { cy.get("[data-cy=button]"); });\n',
    ['Avoid using async functions with Cypress before / beforeEach functions'],
  ],
  [
    'no-async-tests',
    'async test contains Cypress',
    'it("works", async () => { Cypress.env("key"); });\n',
    ['Avoid using async functions with Cypress tests'],
  ],
  [
    'no-chained-get',
    'nested cy.get chain',
    'cy.get("[data-cy=list]").get(".item");\n',
    ['Avoid chaining multiple cy.get() calls'],
  ],
  ['no-debug', 'cy.debug', 'cy.debug();\n', ['Do not use cy.debug command']],
  [
    'no-force',
    'force option on action',
    'cy.get("[data-cy=button]").click({ force: true });\n',
    ['Do not use force on click and type calls'],
  ],
  ['no-pause', 'cy.pause', 'cy.pause();\n', ['Do not use cy.pause command']],
  [
    'no-unnecessary-waiting',
    'numeric wait literal',
    'cy.wait(100);\n',
    ['Do not wait for arbitrary time periods'],
  ],
  [
    'no-unnecessary-waiting',
    'numeric wait variable',
    'const waitTime = 100;\ncy.wait(waitTime);\n',
    ['Do not wait for arbitrary time periods'],
  ],
  [
    'no-unnecessary-waiting',
    'numeric default parameter',
    'function waitFor(ms = 100) { cy.wait(ms); }\n',
    ['Do not wait for arbitrary time periods'],
  ],
  [
    'no-xpath',
    'cy.xpath',
    'cy.xpath("//button");\n',
    [
      'cy.xpath() is deprecated and unsupported. Consider using cy.get() with appropriate selectors instead.',
    ],
  ],
  [
    'require-data-selectors',
    'class selector',
    'cy.get(".submit");\n',
    ['use data-* attribute selectors instead of classes or tag names'],
  ],
  [
    'require-data-selectors',
    'conditional with non-data branch',
    'cy.get(active ? "[data-cy=active]" : ".inactive");\n',
    ['use data-* attribute selectors instead of classes or tag names'],
  ],
  [
    'unsafe-to-chain-command',
    'unsafe action in middle of chain',
    'cy.get("[data-cy=button]").click().should("be.visible");\n',
    [
      'It is unsafe to chain further commands that rely on the subject after this command. It is best to split the chain, chaining again from `cy.` in a next command line.',
    ],
  ],
  [
    'unsafe-to-chain-command',
    'custom unsafe method option',
    'cy.get("[data-cy=todo]").customType("todo").should("have.class", "active");\n',
    [
      'It is unsafe to chain further commands that rely on the subject after this command. It is best to split the chain, chaining again from `cy.` in a next command line.',
    ],
    [{ methods: ['customType'] }],
  ],
];

function runRule(ruleName, sourceText, options = [], filename = 'fixture.cy.js') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const rule = plugin.rules[ruleName];
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

function renderMessage(ruleName, report) {
  return plugin.rules[ruleName].meta.messages[report.messageId];
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

function runOxlint(ruleName, code, options) {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'cypress-plugin-'));

  try {
    const sourcePath = join(temp, 'fixture.cy.js');
    const configPath = join(temp, 'oxlint.config.jsonc');
    const ruleConfig = options == null ? 'error' : ['error', options];

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'cypress',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`cypress/${ruleName}`]: ruleConfig,
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

describe('cypress plugin shape', () => {
  it('exposes all ported rules', () => {
    expect(plugin.meta?.name).toBe('cypress');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.implementedCypressRuleNames).toEqual(expectedRuleNames);
    expect(typeof plugin.scanCypress).toBe('function');
  });

  it('ships upstream-compatible configs', () => {
    expect(plugin.configs.globals.languageOptions.globals).toMatchObject({
      cy: false,
      Cypress: false,
      beforeEach: false,
      it: false,
    });
    expect(plugin.configs.recommended.rules).toEqual({
      'cypress/no-assigning-return-values': 'error',
      'cypress/no-unnecessary-waiting': 'error',
      'cypress/no-async-tests': 'error',
      'cypress/unsafe-to-chain-command': 'error',
    });
    expect(plugin.configs['recommended-legacy'].rules).toEqual(plugin.configs.recommended.rules);
  });

  it('marks no-and fixable and no-xpath deprecated', () => {
    expect(plugin.rules['no-and'].meta.fixable).toBe('code');
    expect(plugin.rules['no-xpath'].meta.deprecated.message).toContain('@cypress/xpath');
  });
});

describe('cypress rules through direct adapter harness', () => {
  it.each(validCases)('accepts %s: %s', (ruleName, _name, code) => {
    expect(runRule(ruleName, code)).toEqual([]);
  });

  it.each(invalidCases)('reports %s: %s', (ruleName, _name, code, messages, options = []) => {
    const reports = runRule(ruleName, code, options);

    expect(reports.map((report) => renderMessage(ruleName, report))).toEqual(messages);
  });

  it('maps no-and autofix ranges from Rust byte offsets to UTF-16 ranges', () => {
    const code = 'cy.get("é").and("be.visible");\n';
    const [report] = runRule('no-and', code);
    const fixes = [];

    report.fix({
      replaceTextRange(range, replacementText) {
        fixes.push({ range, replacementText });
      },
    });

    expect(fixes).toEqual([
      {
        range: [code.indexOf('and'), code.indexOf('and') + 3],
        replacementText: 'should',
      },
    ]);
  });
});

describe('cypress rules through oxlint jsPlugins', () => {
  it('reports an invalid rule through the CLI', () => {
    const result = runOxlint('no-debug', 'cy.debug();\n');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('cypress(no-debug)');
  });

  it('passes unsafe-to-chain options through the CLI', () => {
    const result = runOxlint(
      'unsafe-to-chain-command',
      'cy.get("[data-cy=todo]").customType("todo").should("have.class", "active");\n',
      { methods: ['customType'] },
    );

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('cypress(unsafe-to-chain-command)');
  });
});
