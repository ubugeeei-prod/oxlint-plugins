import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');
const ruleName = 'only-export-components';
const fullRuleName = `react-refresh/${ruleName}`;

const validCases = [
  {
    name: 'named function component export',
    filename: 'NamedFunction.tsx',
    code: 'export function Foo() { return null; }\n',
  },
  {
    name: 'export specifier for a local component',
    filename: 'Specifier.tsx',
    code: 'const Foo = () => null;\nexport { Foo };\n',
  },
  {
    name: 'default named function component',
    filename: 'DefaultFunction.tsx',
    code: 'export default function Foo() { return null; }\n',
  },
  {
    name: 'component variable arrow export',
    filename: 'Arrow.tsx',
    code: 'export const Foo = () => null;\n',
  },
  {
    name: 'memo and forwardRef built-in HOCs',
    filename: 'Hoc.tsx',
    code: [
      "import { memo, forwardRef } from 'react';",
      'export const Foo = memo(function Foo() { return null; });',
      'export const Bar = forwardRef(function Bar() { return null; });',
    ].join('\n'),
  },
  {
    name: 'nested built-in HOCs',
    filename: 'NestedHoc.tsx',
    code: 'export default memo(forwardRef(function Foo() { return null; }));\n',
  },
  {
    name: 'extra HOC option for styled-components',
    filename: 'Styled.tsx',
    options: { extraHOCs: ['styled'] },
    code: 'export const Button = styled.div`color: red;`;\n',
  },
  {
    name: 'allowed constant export option',
    filename: 'Constant.tsx',
    options: { allowConstantExport: true },
    code: 'export const Foo = () => null;\nexport const answer = 42;\n',
  },
  {
    name: 'allowed framework export names',
    filename: 'Framework.tsx',
    options: { allowExportNames: ['loader'] },
    code: 'export const Foo = () => null;\nexport const loader = () => null;\n',
  },
  {
    name: 'standalone React context export',
    filename: 'Context.tsx',
    code: "import { createContext } from 'react';\nexport const FooContext = createContext(null);\n",
  },
  {
    name: 'React class component export',
    filename: 'Class.tsx',
    code: 'export class Foo extends React.Component { render() { return null; } }\n',
  },
  {
    name: 'js files are ignored without checkJS',
    filename: 'Ignored.js',
    code: 'export const Foo = () => null;\nexport const foo = 1;\n',
  },
  {
    name: 'checkJS skips js files without React import',
    filename: 'NoReactImport.js',
    options: { checkJS: true },
    code: 'export const Foo = () => null;\nexport const foo = 1;\n',
  },
  {
    name: 'test-like files are ignored',
    filename: 'Component.test.tsx',
    code: 'export const Foo = () => null;\nexport const foo = 1;\n',
  },
];

const invalidCases = [
  {
    name: 'non-component named export next to a component',
    filename: 'NamedExport.tsx',
    code: 'export const Foo = () => null;\nexport const foo = 1;\n',
    messageIds: ['namedExport'],
  },
  {
    name: 'underscore-prefixed export next to a component',
    filename: 'Underscore.tsx',
    code: 'export const _Foo = () => null;\nexport const Bar = () => null;\n',
    messageIds: ['namedExport'],
  },
  {
    name: 'export all declaration',
    filename: 'ExportAll.tsx',
    code: "export * from './foo';\n",
    messageIds: ['exportAll'],
  },
  {
    name: 'anonymous default arrow export',
    filename: 'AnonymousArrow.tsx',
    code: 'export default () => null;\n',
    messageIds: ['anonymousExport'],
  },
  {
    name: 'anonymous memo default export',
    filename: 'AnonymousMemo.tsx',
    code: 'export default memo(() => null);\n',
    messageIds: ['anonymousExport'],
  },
  {
    name: 'local component with only non-component exports',
    filename: 'LocalComponent.tsx',
    code: 'const Foo = () => null;\nexport const tabs = [];\n',
    messageIds: ['localComponents'],
  },
  {
    name: 'local component without exports',
    filename: 'NoExport.tsx',
    code: 'const Foo = () => null;\n',
    messageIds: ['noExport'],
  },
  {
    name: 'checkJS reports js files with React import',
    filename: 'ReactImport.js',
    options: { checkJS: true },
    code: "import React from 'react';\nexport const Foo = () => null;\nexport const foo = 1;\n",
    messageIds: ['namedExport'],
  },
  {
    name: 'custom HOC requires extraHOCs',
    filename: 'CustomHoc.tsx',
    code: 'const MyComponent = () => null;\nexport default observer(MyComponent);\n',
    messageIds: ['localComponents'],
  },
  {
    name: 'React context next to a component',
    filename: 'ContextWithComponent.tsx',
    code: 'export const FooContext = React.createContext(null);\nexport const Foo = () => null;\n',
    messageIds: ['reactContext'],
  },
  {
    name: 'non-React class next to a component',
    filename: 'PlainClass.tsx',
    code: 'export class Foo { render() { return null; } }\nexport const Bar = () => null;\n',
    messageIds: ['namedExport'],
  },
  {
    name: 'anonymous default class export',
    filename: 'AnonymousClass.tsx',
    code: 'export default class { render() { return null; } }\n',
    messageIds: ['anonymousExport'],
  },
  {
    name: 'TypeScript enum next to a component',
    filename: 'Enum.tsx',
    code: 'export const Foo = () => null;\nexport enum RouteKind { Static }\n',
    messageIds: ['namedExport'],
  },
];

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

function runOxlint({ filename, code, options }) {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'react-refresh-plugin-'));

  try {
    const sourcePath = join(temp, filename);
    const configPath = join(temp, 'oxlint.config.jsonc');
    const ruleConfig = options == null ? 'error' : ['error', options];

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'react-refresh',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [fullRuleName]: ruleConfig,
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

describe('react-refresh plugin shape', () => {
  it('exposes the only-export-components rule', () => {
    expect(plugin.meta?.name).toBe('react-refresh');
    expect(plugin.rules[ruleName].meta.messages.namedExport).toContain('Fast refresh');
    expect(typeof plugin.rules[ruleName].createOnce).toBe('function');
  });

  it('ships upstream-compatible configs', () => {
    expect(plugin.configs.recommended.rules).toEqual({
      [fullRuleName]: ['error', {}],
    });
    expect(plugin.configs.vite.rules).toEqual({
      [fullRuleName]: ['error', { allowConstantExport: true }],
    });
    expect(plugin.configs.next.rules[fullRuleName][1].allowExportNames).toContain('revalidate');
    expect(plugin.reactRefresh.configs.recommended().rules).toEqual(
      plugin.configs.recommended.rules,
    );
  });
});

describe('react-refresh only-export-components through oxlint jsPlugins', () => {
  it.each(validCases)('accepts $name', (fixture) => {
    const result = runOxlint(fixture);

    expect(result.status).toBe(0);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toEqual([]);
  });

  it.each(invalidCases)('reports $name', (fixture) => {
    const result = runOxlint(fixture);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics.map((diagnostic) => diagnostic.code)).toEqual(
      fixture.messageIds.map(() => 'react-refresh(only-export-components)'),
    );
    expect(result.diagnostics.map((diagnostic) => diagnostic.message)).toEqual(
      fixture.messageIds.map((messageId) => plugin.rules[ruleName].meta.messages[messageId]),
    );
  });
});
