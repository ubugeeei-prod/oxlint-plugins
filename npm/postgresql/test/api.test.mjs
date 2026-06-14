// Sanity checks for the native NAPI surface (api.js -> native.js), independent
// of the ESLint-compat adapter. Confirms the libpg_query-backed scanner is
// wired and returns the expected diagnostic shape.

import { describe, expect, it } from 'vitest';

import { implementedPostgresqlRuleNames, scanPostgresql } from '../api.js';

describe('native scanPostgresql', () => {
  it('flags SELECT *', () => {
    const diagnostics = scanPostgresql('SELECT * FROM users', { ruleNames: ['no-select-star'] });
    expect(diagnostics.length).toBe(1);
    expect(diagnostics[0].ruleName).toBe('no-select-star');
    expect(diagnostics[0].messageId).toBe('noSelectStar');
    expect(diagnostics[0].loc.startLine).toBe(1);
    expect(diagnostics[0].loc.startColumn).toBe(7);
  });

  it('allows explicit columns', () => {
    const diagnostics = scanPostgresql('SELECT id, name FROM users', {
      ruleNames: ['no-select-star'],
    });
    expect(diagnostics).toEqual([]);
  });

  it('returns nothing for a disabled rule', () => {
    const diagnostics = scanPostgresql('SELECT * FROM users', { ruleNames: [] });
    expect(diagnostics).toEqual([]);
  });

  it('does not throw on a syntax error', () => {
    expect(() =>
      scanPostgresql('SELECT FROM WHERE )(', { ruleNames: ['no-select-star'] }),
    ).not.toThrow();
  });

  it('exposes the implemented rule names', () => {
    expect(implementedPostgresqlRuleNames()).toContain('no-select-star');
  });
});
