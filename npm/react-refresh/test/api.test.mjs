import { describe, expect, it } from 'vitest';

import {
  defaultHocs,
  isConstantExportExpressionKind,
  isReactComponentName,
  scanOnlyExportComponents,
  shouldScanFilename,
} from '../api.js';

describe('react-refresh native api', () => {
  it('classifies React component names like upstream', () => {
    expect(isReactComponentName('Foo')).toBe(true);
    expect(isReactComponentName('Foo2')).toBe(true);
    expect(isReactComponentName('CMS')).toBe(true);
    expect(isReactComponentName('foo')).toBe(false);
    expect(isReactComponentName('_Foo')).toBe(false);
    expect(isReactComponentName('Foo-Bar')).toBe(false);
  });

  it('keeps upstream filename scan gates', () => {
    expect(shouldScanFilename('/src/App.tsx')).toBe(true);
    expect(shouldScanFilename('/src/App.jsx')).toBe(true);
    expect(shouldScanFilename('/src/App.js')).toBe(false);
    expect(shouldScanFilename('/src/App.js', true)).toBe(true);
    expect(shouldScanFilename('/src/App.test.tsx')).toBe(false);
    expect(shouldScanFilename('/src/App.spec.jsx')).toBe(false);
    expect(shouldScanFilename('/src/App.cy.jsx')).toBe(false);
    expect(shouldScanFilename('/src/App.stories.tsx')).toBe(false);
  });

  it('exposes higher-order component and constant export helpers', () => {
    expect(defaultHocs()).toEqual(['memo', 'forwardRef', 'lazy']);
    expect(isConstantExportExpressionKind('Literal')).toBe(true);
    expect(isConstantExportExpressionKind('UnaryExpression')).toBe(true);
    expect(isConstantExportExpressionKind('TemplateLiteral')).toBe(true);
    expect(isConstantExportExpressionKind('BinaryExpression')).toBe(true);
    expect(isConstantExportExpressionKind('ObjectExpression')).toBe(false);
    expect(isConstantExportExpressionKind('CallExpression')).toBe(false);
  });

  it('runs the rule scan in native code', () => {
    expect(
      scanOnlyExportComponents(
        'export const Foo = () => null;\nexport const foo = 1;\n',
        'Component.tsx',
      ).map((diagnostic) => diagnostic.messageId),
    ).toEqual(['namedExport']);

    expect(
      scanOnlyExportComponents(
        "import React from 'react';\nexport const Foo = () => null;\nexport const foo = 1;\n",
        'Component.js',
        { checkJS: true },
      ).map((diagnostic) => diagnostic.messageId),
    ).toEqual(['namedExport']);
  });
});
