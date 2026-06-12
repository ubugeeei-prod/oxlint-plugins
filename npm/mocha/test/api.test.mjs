import { describe, expect, it } from 'vitest';

import { implementedMochaRuleNames, scanMocha } from '../api.js';

const expectedRuleNames = [
  'consistent-interface',
  'consistent-spacing-between-blocks',
  'handle-done-callback',
  'max-top-level-suites',
  'no-async-suite',
  'no-empty-title',
  'no-exclusive-tests',
  'no-exports',
  'no-global-tests',
  'no-hooks',
  'no-hooks-for-single-case',
  'no-identical-title',
  'no-mocha-arrows',
  'no-nested-tests',
  'no-pending-tests',
  'no-return-and-callback',
  'no-return-from-async',
  'no-setup-in-describe',
  'no-sibling-hooks',
  'no-synchronous-tests',
  'no-top-level-hooks',
  'prefer-arrow-callback',
  'valid-suite-title',
  'valid-test-title',
];

describe('mocha native API', () => {
  it('exposes all eslint-plugin-mocha rule names', () => {
    expect(implementedMochaRuleNames()).toEqual(expectedRuleNames);
  });

  it('scans multiple Mocha rules through one native call', () => {
    const diagnostics = scanMocha(
      [
        'before(function () {});',
        'it("global", function () {});',
        'describe.only("", async function () {',
        '  before(function (done) {});',
        '  before(function () {});',
        '  it("works", function (done) { return fetch("/"); });',
        '  it("works", () => {});',
        '  it.skip("later");',
        '  it("async return", async function () { return fetch("/"); });',
        '  it("nested", function () { it("bad", function () {}); });',
        '  helper();',
        '});',
        'suite("tdd", function () { test("bad", function () {}); });',
        'export const value = 1;',
      ].join('\n'),
      'fixture.test.js',
    );

    expect(new Set(diagnostics.map((diagnostic) => diagnostic.ruleName))).toEqual(
      new Set([
        'consistent-spacing-between-blocks',
        'consistent-interface',
        'handle-done-callback',
        'max-top-level-suites',
        'no-async-suite',
        'no-empty-title',
        'no-exclusive-tests',
        'no-exports',
        'no-global-tests',
        'no-hooks',
        'no-identical-title',
        'no-mocha-arrows',
        'no-nested-tests',
        'no-pending-tests',
        'no-return-and-callback',
        'no-return-from-async',
        'no-setup-in-describe',
        'no-sibling-hooks',
        'no-synchronous-tests',
        'no-top-level-hooks',
        'prefer-arrow-callback',
      ]),
    );
  });

  it('passes rule options to Rust', () => {
    const diagnostics = scanMocha(
      [
        'describe("bad suite", function () {',
        '  before(function () {});',
        '  it("bad test", function () { this.timeout(1000); });',
        '});',
      ].join('\n'),
      'fixture.test.js',
      {
        noHooksAllowed: ['before'],
        validSuiteTitlePattern: '^Suite',
        validTestTitlePattern: '^should',
      },
    );

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toContain('valid-suite-title');
    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toContain('valid-test-title');
    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).not.toContain('no-hooks');
  });

  it('honors prefer-arrow allowUnboundThis', () => {
    const diagnostics = scanMocha(
      'it("bad test", function () { this.timeout(1000); });\n',
      'fixture.test.js',
      {
        preferArrowAllowUnboundThis: true,
      },
    );

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).not.toContain(
      'prefer-arrow-callback',
    );
  });
});
