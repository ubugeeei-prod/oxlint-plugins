import { describe, expect, it } from 'vitest';

import { implementedCypressRuleNames, scanCypress } from '../api.js';

describe('cypress native API', () => {
  it('exposes all eslint-plugin-cypress rule names', () => {
    expect(implementedCypressRuleNames()).toEqual([
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
    ]);
  });

  it('scans multiple Cypress rules through one native call', () => {
    const diagnostics = scanCypress(
      [
        'const button = cy.get(".submit");',
        'beforeEach("setup", async () => { cy.get("[data-cy=button]"); });',
        'it("works", async () => { Cypress.env("key"); });',
        'cy.get(".button").and("be.visible");',
        'cy.wait(100);',
        'cy.get("[data-cy=button]").click({ force: true }).should("exist");',
        'cy.visit("/home");',
        'cy.screenshot();',
      ].join('\n'),
      'fixture.cy.js',
    );

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'no-assigning-return-values',
      'require-data-selectors',
      'no-async-before',
      'no-async-tests',
      'no-and',
      'require-data-selectors',
      'no-unnecessary-waiting',
      'no-force',
      'unsafe-to-chain-command',
      'assertion-before-screenshot',
    ]);
  });

  it('returns LSP-shaped locations and native byte fixes', () => {
    const code = 'cy.get("é").and("be.visible");\n';
    const diagnostics = scanCypress(code, 'fixture.cy.js');
    const diagnostic = diagnostics.find((diagnostic) => diagnostic.ruleName === 'no-and');

    expect(diagnostic).toMatchObject({
      ruleName: 'no-and',
      messageId: 'unexpected',
      loc: {
        startLine: 1,
        startColumn: 0,
        endLine: 1,
        endColumn: 29,
      },
      fix: {
        start: Buffer.byteLength(code.slice(0, code.indexOf('and'))),
        end: Buffer.byteLength(code.slice(0, code.indexOf('and') + 3)),
        replacement: 'should',
      },
    });
  });

  it('passes unsafe-to-chain custom methods to Rust', () => {
    const diagnostics = scanCypress(
      'cy.get("[data-cy=todo]").customType("todo").should("have.class", "active");',
      'fixture.cy.js',
      { unsafeToChainMethods: ['customType'] },
    );

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toContain(
      'unsafe-to-chain-command',
    );
  });
});
