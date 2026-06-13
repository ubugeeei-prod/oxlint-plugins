import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');
const pluginAlias = 'react-hooks-js';
const ruleName = 'rules-of-hooks';
const fullRuleName = `${pluginAlias}/${ruleName}`;
const canonicalRuleName = `react-hooks/${ruleName}`;

const validCases = [
  {
    name: 'function component calls hooks at top level',
    filename: 'Component.tsx',
    code: 'function Component() { useState(); React.useEffect(() => {}); }\n',
  },
  {
    name: 'custom hook calls hooks at top level',
    filename: 'Hook.tsx',
    code: 'function useThing() { useMemo(() => 1, []); }\n',
  },
  {
    name: 'React memo callback is a component scope',
    filename: 'Memo.tsx',
    code: 'const Component = React.memo(() => { useState(); });\n',
  },
  {
    name: 'forwardRef callback is a component scope',
    filename: 'ForwardRef.tsx',
    code: 'const Fancy = forwardRef(function Fancy(props, ref) { useImperativeHandle(ref, () => ({})); });\n',
  },
  {
    name: 'React use can be conditional and looped',
    filename: 'Use.tsx',
    code: 'function Component() { if (cond) { use(resource); } for (const item of items) { use(item); } }\n',
  },
];

const invalidCases = [
  {
    name: 'top-level hook',
    filename: 'TopLevel.tsx',
    code: 'useState();\n',
    messageIds: ['topLevel'],
  },
  {
    name: 'invalid function name',
    filename: 'InvalidFunction.tsx',
    code: 'function normal() { useState(); }\n',
    messageIds: ['invalidFunction'],
  },
  {
    name: 'callback inside component',
    filename: 'Callback.tsx',
    code: 'function Component() { items.map(() => { useState(); }); }\n',
    messageIds: ['callback'],
  },
  {
    name: 'conditional hook',
    filename: 'Conditional.tsx',
    code: 'function Component() { if (cond) { useState(); } }\n',
    messageIds: ['conditional'],
  },
  {
    name: 'early return before hook',
    filename: 'EarlyReturn.tsx',
    code: 'function Component() { if (cond) return null; useState(); }\n',
    messageIds: ['conditional'],
  },
  {
    name: 'hook in loop',
    filename: 'Loop.tsx',
    code: 'function Component() { while (cond) { useState(); } }\n',
    messageIds: ['loop'],
  },
  {
    name: 'hook in async component',
    filename: 'Async.tsx',
    code: 'async function Component() { useState(); }\n',
    messageIds: ['async'],
  },
  {
    name: 'hook in class component',
    filename: 'Class.tsx',
    code: 'class App extends React.Component { render() { useState(); } }\n',
    messageIds: ['class'],
  },
  {
    name: 'React use inside try/catch',
    filename: 'TryCatch.tsx',
    code: 'function Component() { try { use(resource); } catch (err) {} }\n',
    messageIds: ['tryCatch'],
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

function runOxlint({ filename, code }) {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'react-hooks-plugin-'));

  try {
    const sourcePath = join(temp, filename);
    const configPath = join(temp, 'oxlint.config.jsonc');

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: pluginAlias,
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [fullRuleName]: 'error',
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

describe('react-hooks plugin shape', () => {
  it('exposes rules-of-hooks and implemented configs only', () => {
    expect(plugin.meta?.name).toBe('react-hooks');
    expect(Object.keys(plugin.rules)).toEqual(['rules-of-hooks']);
    expect(plugin.rules[ruleName].meta.messages.conditional).toContain('called conditionally');
    expect(typeof plugin.rules[ruleName].createOnce).toBe('function');

    expect(plugin.configs.recommended.rules).toEqual({
      [canonicalRuleName]: 'error',
    });
    expect(plugin.configs['recommended-latest'].rules).toEqual({
      [canonicalRuleName]: 'error',
    });
    expect(plugin.configs.flat.recommended.plugins['react-hooks']).toBe(plugin);
  });
});

describe('react-hooks rules-of-hooks through oxlint jsPlugins', () => {
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
      fixture.messageIds.map(() => 'react-hooks-js(rules-of-hooks)'),
    );
    expect(result.diagnostics.map((diagnostic) => diagnostic.message)).toEqual(
      fixture.messageIds.map((messageId) =>
        plugin.rules[ruleName].meta.messages[messageId]
          .replace('{{hook}}', messageId === 'tryCatch' ? 'use' : 'useState')
          .replace('{{functionName}}', 'normal'),
      ),
    );
  });
});
