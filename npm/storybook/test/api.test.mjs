import { describe, expect, it } from 'vitest';

import { implementedStorybookRuleNames, scanStorybook } from '../api.js';

const expectedRuleNames = [
  'await-interactions',
  'context-in-play-function',
  'csf-component',
  'default-exports',
  'hierarchy-separator',
  'meta-inline-properties',
  'meta-satisfies-type',
  'no-redundant-story-name',
  'no-renderer-packages',
  'no-stories-of',
  'no-title-property-in-meta',
  'no-uninstalled-addons',
  'prefer-pascal-case',
  'story-exports',
  'use-storybook-expect',
  'use-storybook-testing-library',
];

function scan(ruleName, sourceText, options = {}, filename = 'Button.stories.tsx') {
  return scanStorybook(sourceText, filename, {
    ...options,
    ruleNames: [ruleName],
  });
}

function applyFixes(sourceText, diagnostic) {
  return [...(diagnostic.fixes || [])]
    .sort((a, b) => b.start - a.start)
    .reduce(
      (text, fix) => text.slice(0, fix.start) + fix.replacement + text.slice(fix.end),
      sourceText,
    );
}

describe('storybook native API', () => {
  it('exposes all eslint-plugin-storybook rule names', () => {
    expect(implementedStorybookRuleNames()).toEqual(expectedRuleNames);
  });

  it('reports and fixes await-interactions', () => {
    const source = 'Basic.play = async () => { userEvent.click(button) }';
    const diagnostics = scan('await-interactions', source);

    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]).toMatchObject({
      ruleName: 'await-interactions',
      messageId: 'interactionShouldBeAwaited',
      data: { method: 'userEvent' },
    });
    expect(applyFixes(source, diagnostics[0])).toBe(
      'Basic.play = async () => { await userEvent.click(button) }',
    );
  });

  it('reports context-in-play-function', () => {
    const source =
      'export const SecondStory = { play: async ({ canvasElement }) => { await FirstStory.play({ canvasElement }) } }';
    expect(scan('context-in-play-function', source).map((d) => d.messageId)).toEqual([
      'passContextToPlayFunction',
    ]);
  });

  it('reports csf-component for meta without component', () => {
    expect(scan('csf-component', "export default { title: 'Button' }")).toHaveLength(1);
  });

  it('reports and fixes default-exports with inferred component name', () => {
    const source = "import Button from './Button';\nexport const Primary = {};";
    const diagnostic = scan('default-exports', source, {}, '/tmp/Button.stories.tsx')[0];

    expect(diagnostic.messageId).toBe('shouldHaveDefaultExport');
    expect(applyFixes(source, diagnostic)).toBe(
      "import Button from './Button';\nexport default { component: Button }\nexport const Primary = {};",
    );
  });

  it('reports and fixes hierarchy-separator', () => {
    const source = "export default { title: 'Atoms|Button', component: Button }";
    const diagnostic = scan('hierarchy-separator', source)[0];

    expect(diagnostic.data.metaTitle).toBe("'Atoms|Button'");
    expect(applyFixes(source, diagnostic)).toBe(
      "export default { title: 'Atoms/Button', component: Button }",
    );
  });

  it('reports meta-inline-properties for dynamic title and args', () => {
    const source =
      "const title = 'Button';\nconst args = {};\nexport default { title, args, component: Button }";
    expect(scan('meta-inline-properties', source).map((d) => d.data.property)).toEqual([
      'title',
      'args',
    ]);
  });

  it('reports and fixes meta-satisfies-type for variable annotations', () => {
    const source = 'const meta: Meta<typeof Button> = { component: Button };\nexport default meta;';
    const diagnostic = scan('meta-satisfies-type', source)[0];

    expect(diagnostic.messageId).toBe('metaShouldSatisfyType');
    expect(applyFixes(source, diagnostic)).toBe(
      'const meta = { component: Button } satisfies Meta<typeof Button>;\nexport default meta;',
    );
  });

  it('reports and fixes no-redundant-story-name', () => {
    const source = "export const PrimaryButton = { name: 'Primary Button', args: {} }";
    const diagnostic = scan('no-redundant-story-name', source)[0];

    expect(diagnostic.messageId).toBe('storyNameIsRedundant');
    expect(applyFixes(source, diagnostic)).toBe('export const PrimaryButton = {  args: {} }');
  });

  it('reports no-renderer-packages', () => {
    expect(
      scan('no-renderer-packages', "import { Meta } from '@storybook/react'")[0],
    ).toMatchObject({
      data: {
        rendererPackage: '@storybook/react',
        suggestions:
          '@storybook/nextjs, @storybook/react-vite, @storybook/nextjs-vite, @storybook/react-webpack5, @storybook/react-native-web-vite',
      },
    });
  });

  it('reports no-stories-of', () => {
    expect(scan('no-stories-of', "import { storiesOf } from '@storybook/react'")).toHaveLength(1);
  });

  it('reports and fixes no-title-property-in-meta', () => {
    const source = "export default { title: 'Button', component: Button }";
    const diagnostic = scan('no-title-property-in-meta', source)[0];

    expect(diagnostic.messageId).toBe('noTitleInMeta');
    expect(applyFixes(source, diagnostic)).toBe('export default {  component: Button }');
  });

  it('reports no-uninstalled-addons using adapter-supplied dependencies', () => {
    const source =
      "export default { addons: ['@storybook/addon-essentials', '@storybook/not-installed'] }";
    const diagnostics = scan('no-uninstalled-addons', source, {
      installedAddons: ['@storybook/addon-essentials'],
      packageJsonPath: '/workspace/package.json',
    });

    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]).toMatchObject({
      data: {
        addonName: '@storybook/not-installed',
        packageJsonPath: '/workspace/package.json',
      },
    });
  });

  it('reports and fixes prefer-pascal-case', () => {
    const source = 'export const primary_story = {};';
    const diagnostic = scan('prefer-pascal-case', source)[0];

    expect(diagnostic.data.name).toBe('primary_story');
    expect(applyFixes(source, diagnostic)).toBe('export const PrimaryStory = {};');
  });

  it('reports story-exports when meta has no story exports', () => {
    expect(scan('story-exports', 'export default { component: Button }')[0].messageId).toBe(
      'shouldHaveStoryExport',
    );
  });

  it('reports use-storybook-expect for global expect', () => {
    const source = 'Default.play = () => { expect(1).toBe(1) }';
    expect(scan('use-storybook-expect', source)).toHaveLength(1);
  });

  it('reports and fixes use-storybook-testing-library', () => {
    const source =
      "import userEvent, { within, screen as storyScreen } from '@testing-library/user-event'";
    const diagnostic = scan('use-storybook-testing-library', source)[0];

    expect(diagnostic.data.library).toBe('@testing-library/user-event');
    expect(applyFixes(source, diagnostic)).toBe(
      "import { userEvent, within, screen as storyScreen } from 'storybook/test'",
    );
  });
});
