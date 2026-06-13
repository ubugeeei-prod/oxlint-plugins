import { describe, expect, it } from 'vitest';

import { implementedTestingLibraryRuleNames, scanTestingLibrary } from '../api.js';

const expectedRuleNames = [
  'await-async-events',
  'await-async-queries',
  'await-async-utils',
  'consistent-data-testid',
  'no-await-sync-events',
  'no-await-sync-queries',
  'no-container',
  'no-debugging-utils',
  'no-dom-import',
  'no-global-regexp-flag-in-query',
  'no-manual-cleanup',
  'no-node-access',
  'no-promise-in-fire-event',
  'no-render-in-lifecycle',
  'no-test-id-queries',
  'no-unnecessary-act',
  'no-wait-for-multiple-assertions',
  'no-wait-for-side-effects',
  'no-wait-for-snapshot',
  'prefer-explicit-assert',
  'prefer-find-by',
  'prefer-implicit-assert',
  'prefer-presence-queries',
  'prefer-query-by-disappearance',
  'prefer-query-matchers',
  'prefer-screen-queries',
  'prefer-user-event',
  'prefer-user-event-setup',
  'render-result-naming-convention',
];

const focusedRuleCases = [
  ['await-async-events', 'userEvent.click(button);'],
  ['await-async-queries', 'screen.findByText("Save");'],
  ['await-async-utils', 'waitFor(() => screen.getByText("Save"));'],
  ['consistent-data-testid', 'render(<button data-testid="BadId" />);'],
  ['no-await-sync-events', 'await fireEvent.click(button);'],
  ['no-await-sync-queries', 'await screen.getByText("Save");'],
  ['no-container', 'container.querySelector(".save");'],
  ['no-debugging-utils', 'screen.debug();'],
  ['no-dom-import', "import { fireEvent } from '@testing-library/dom';"],
  ['no-global-regexp-flag-in-query', 'screen.getByText(/Save/g);'],
  ['no-manual-cleanup', 'cleanup();'],
  ['no-node-access', 'screen.getByText("Save").firstChild;'],
  ['no-promise-in-fire-event', 'fireEvent.click(await button());'],
  ['no-render-in-lifecycle', 'beforeEach(() => { render(<App />); });'],
  ['no-test-id-queries', 'screen.getByTestId("save");'],
  ['no-unnecessary-act', 'act(() => render(<App />));'],
  ['no-wait-for-multiple-assertions', 'waitFor(() => { expect(a).toBe(1); expect(b).toBe(2); });'],
  ['no-wait-for-side-effects', 'waitFor(() => { fireEvent.click(button); });'],
  ['no-wait-for-snapshot', 'waitFor(() => expect(container).toMatchSnapshot());'],
  ['prefer-explicit-assert', 'screen.getByText("Save");'],
  ['prefer-find-by', 'waitFor(() => screen.getByText("Save"));'],
  ['prefer-implicit-assert', 'expect(screen.getByText("Save")).toBeInTheDocument();'],
  ['prefer-presence-queries', 'expect(screen.queryByText("Save")).toBeInTheDocument();'],
  ['prefer-query-by-disappearance', 'waitForElementToBeRemoved(() => screen.getByText("Gone"));'],
  ['prefer-query-matchers', 'expect(screen.queryByText("Gone")).toBeNull();'],
  ['prefer-screen-queries', 'const { getByText } = render(<App />);'],
  ['prefer-user-event', 'fireEvent.click(button);'],
  ['prefer-user-event-setup', 'userEvent.click(button);'],
  ['render-result-naming-convention', 'const result = render(<App />);'],
];

describe('testing-library native API', () => {
  it('exposes all eslint-plugin-testing-library rule names', () => {
    expect(implementedTestingLibraryRuleNames()).toEqual(expectedRuleNames);
  });

  it('reports each ported rule with a focused fixture', () => {
    for (const [ruleName, source] of focusedRuleCases) {
      const diagnostics = scanTestingLibrary(source, 'fixture.test.tsx', { ruleNames: [ruleName] });

      expect(
        diagnostics.map((diagnostic) => diagnostic.ruleName),
        ruleName,
      ).toContain(ruleName);
    }
  });

  it('scans representative Testing Library anti-patterns together', () => {
    const diagnostics = scanTestingLibrary(
      [
        "import { fireEvent } from '@testing-library/dom';",
        'const { getByText } = render(<Button data-testid="BadId" />);',
        'userEvent.click(button);',
        'await fireEvent.click(button);',
        'screen.getByText(/Save/g);',
        'screen.getByTestId("save");',
        'cleanup();',
        'container.querySelector(".button");',
        'waitFor(() => { expect(a).toBe(1); expect(b).toBe(2); fireEvent.click(button); expect(screen.getByText("x")).toBeInTheDocument(); });',
        'waitForElementToBeRemoved(() => screen.getByText("gone"));',
        'const result = render(<Button />);',
      ].join('\n'),
      'fixture.test.tsx',
    );

    expect(new Set(diagnostics.map((diagnostic) => diagnostic.ruleName))).toEqual(
      new Set([
        'await-async-events',
        'await-async-utils',
        'consistent-data-testid',
        'no-await-sync-events',
        'no-container',
        'no-dom-import',
        'no-global-regexp-flag-in-query',
        'no-manual-cleanup',
        'no-node-access',
        'no-test-id-queries',
        'no-wait-for-multiple-assertions',
        'no-wait-for-side-effects',
        'prefer-explicit-assert',
        'prefer-find-by',
        'prefer-implicit-assert',
        'prefer-query-by-disappearance',
        'prefer-screen-queries',
        'prefer-user-event',
        'prefer-user-event-setup',
        'render-result-naming-convention',
      ]),
    );
  });

  it('filters rule names and accepts awaited async interactions', () => {
    expect(
      scanTestingLibrary('await userEvent.click(button);', 'fixture.test.tsx', {
        ruleNames: ['await-async-events'],
      }),
    ).toEqual([]);
  });
});
