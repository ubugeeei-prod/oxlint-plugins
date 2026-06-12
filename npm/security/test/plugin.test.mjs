import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

const readMethods = [
  'readUInt8',
  'readUInt16LE',
  'readUInt16BE',
  'readUInt32LE',
  'readUInt32BE',
  'readInt8',
  'readInt16LE',
  'readInt16BE',
  'readInt32LE',
  'readInt32BE',
  'readFloatLE',
  'readFloatBE',
  'readDoubleLE',
  'readDoubleBE',
];

const writeMethods = [
  'writeUInt8',
  'writeUInt16LE',
  'writeUInt16BE',
  'writeUInt32LE',
  'writeUInt32BE',
  'writeInt8',
  'writeInt16LE',
  'writeInt16BE',
  'writeInt32LE',
  'writeInt32BE',
  'writeFloatLE',
  'writeFloatBE',
  'writeDoubleLE',
  'writeDoubleBE',
];

const validCases = [
  ['detect-bidi-characters', 'clean source', 'const accessLevel = "user";\n'],
  ...readMethods.map((method) => [
    'detect-buffer-noassert',
    `${method} without noAssert`,
    `a.${method}(0, false)`,
  ]),
  ...writeMethods.map((method) => [
    'detect-buffer-noassert',
    `${method} without noAssert`,
    `a.${method}(0, 0, false)`,
  ]),
  ['detect-child-process', 'literal child_process exec', "child_process.exec('ls')"],
  [
    'detect-child-process',
    'spawn import is allowed',
    "import { spawn } from 'child_process'; spawn(str);",
  ],
  [
    'detect-child-process',
    'shadowed exec receiver is ignored',
    "var foo = require('child_process'); function fn () { var foo = /hello/; foo.exec(str); }",
  ],
  [
    'detect-child-process',
    'static exec argument is allowed',
    "var child_process = require('child_process'); var FOO = 'ls'; child_process.exec(FOO);",
  ],
  ['detect-disable-mustache-escape', 'bare escapeMarkup assignment', 'escapeMarkup = false'],
  ['detect-eval-with-expression', 'literal eval', "eval('alert()')"],
  ['detect-eval-with-expression', 'empty eval', 'eval()'],
  ['detect-new-buffer', 'literal new Buffer', "var a = new Buffer('test')"],
  [
    'detect-no-csrf-before-method-override',
    'methodOverride before csrf',
    'express.methodOverride();express.csrf()',
  ],
  [
    'detect-non-literal-fs-filename',
    'literal fs filename',
    "var fs = require('fs'); var a = fs.open('test')",
  ],
  [
    'detect-non-literal-fs-filename',
    'non-fs readFile receiver',
    "var something = require('some'); var a = something.readFile(c);",
  ],
  [
    'detect-non-literal-fs-filename',
    'static path resolve',
    "import fs from 'fs'; import path from 'path'; fs.readFileSync(path.resolve(__dirname, './index.html'));",
  ],
  [
    'detect-non-literal-fs-filename',
    'static dirname variable',
    "import fs from 'fs'; import path from 'path'; const dirname = path.dirname(__filename); fs.readFileSync(path.resolve(dirname, './index.html'));",
  ],
  [
    'detect-non-literal-fs-filename',
    'static process cwd template',
    "import fs from 'fs'; fs.readFileSync(`${process.cwd()}/path/to/foo.json`);",
  ],
  ['detect-non-literal-regexp', 'literal regexp constructor', "var a = new RegExp('ab+c', 'i')"],
  [
    'detect-non-literal-regexp',
    'static regexp source variable',
    "var source = 'ab+c'; var a = new RegExp(source, 'i')",
  ],
  ['detect-non-literal-require', 'literal require', "var a = require('b')"],
  [
    'detect-non-literal-require',
    'static template require',
    "const d = 'debounce'; var a = require(`lodash/${d}`)",
  ],
  ['detect-non-literal-require', 'dirname require', "const utils = require(__dirname + '/utils');"],
  ['detect-object-injection', 'plain object', 'var a = {};'],
  ['detect-possible-timing-attacks', 'non-secret comparison', 'if (age === 5) {}'],
  ['detect-pseudoRandomBytes', 'randomBytes member', 'crypto.randomBytes'],
  ['detect-unsafe-regex', 'safe regex literal', '/^d+1337d+$/'],
  ['detect-unsafe-regex', 'safe RegExp constructor', "new RegExp('^d+1337d+$')"],
];

const invalidCases = [
  [
    'detect-bidi-characters',
    'bidi in code',
    'var accessLevel = "user‮ ⁦// Check⁩ ⁦";',
    [
      'Detected potential trojan source attack with unicode bidi introduced in this code: \'var accessLevel = "user‮ ⁦// Check⁩ ⁦";\'.',
      'Detected potential trojan source attack with unicode bidi introduced in this code: \'var accessLevel = "user‮ ⁦// Check⁩ ⁦";\'.',
      'Detected potential trojan source attack with unicode bidi introduced in this code: \'var accessLevel = "user‮ ⁦// Check⁩ ⁦";\'.',
      'Detected potential trojan source attack with unicode bidi introduced in this code: \'var accessLevel = "user‮ ⁦// Check⁩ ⁦";\'.',
    ],
  ],
  [
    'detect-bidi-characters',
    'bidi in comment',
    '/*‮ } ⁦if (isAdmin)⁩ ⁦ begin admins only */',
    [
      "Detected potential trojan source attack with unicode bidi introduced in this comment: '/*‮ } ⁦if (isAdmin)⁩ ⁦ begin admins only */'.",
      "Detected potential trojan source attack with unicode bidi introduced in this comment: '/*‮ } ⁦if (isAdmin)⁩ ⁦ begin admins only */'.",
      "Detected potential trojan source attack with unicode bidi introduced in this comment: '/*‮ } ⁦if (isAdmin)⁩ ⁦ begin admins only */'.",
      "Detected potential trojan source attack with unicode bidi introduced in this comment: '/*‮ } ⁦if (isAdmin)⁩ ⁦ begin admins only */'.",
    ],
  ],
  ...readMethods.map((method) => [
    'detect-buffer-noassert',
    `${method} with noAssert`,
    `a.${method}(0, true)`,
    [`Found Buffer.${method} with noAssert flag set true`],
  ]),
  ...writeMethods.map((method) => [
    'detect-buffer-noassert',
    `${method} with noAssert`,
    `a.${method}(0, 0, true)`,
    [`Found Buffer.${method} with noAssert flag set true`],
  ]),
  [
    'detect-child-process',
    'bare child_process require',
    "require('child_process')",
    ['Found require("child_process")'],
  ],
  [
    'detect-child-process',
    'node child_process require',
    "require('node:child_process')",
    ['Found require("node:child_process")'],
  ],
  [
    'detect-child-process',
    'child exec non-literal',
    "var child = require('child_process'); child.exec(com)",
    ['Found child_process.exec() with non Literal first argument'],
  ],
  [
    'detect-child-process',
    'default import child exec non-literal',
    "import child from 'node:child_process'; child.exec(com)",
    ['Found child_process.exec() with non Literal first argument'],
  ],
  [
    'detect-child-process',
    'destructured exec non-literal',
    "const {exec} = require('child_process'); exec(str)",
    ['Found child_process.exec() with non Literal first argument'],
  ],
  [
    'detect-disable-mustache-escape',
    'escapeMarkup false',
    'a.escapeMarkup = false',
    ['Markup escaping disabled.'],
  ],
  [
    'detect-eval-with-expression',
    'identifier eval',
    'eval(a);',
    ['eval with argument of type Identifier'],
  ],
  ['detect-new-buffer', 'new Buffer variable', 'var a = new Buffer(c)', ['Found new Buffer']],
  [
    'detect-no-csrf-before-method-override',
    'csrf before methodOverride',
    'express.csrf();express.methodOverride()',
    ['express.csrf() middleware found before express.methodOverride()'],
  ],
  [
    'detect-non-literal-fs-filename',
    'fs open variable',
    "var something = require('fs'); var a = something.open(c);",
    ['Found open from package "fs" with non literal argument at index 0'],
  ],
  [
    'detect-non-literal-fs-filename',
    'fs readFile alias',
    "var one = require('node:fs').readFile; one(filename);",
    ['Found readFile from package "node:fs" with non literal argument at index 0'],
  ],
  [
    'detect-non-literal-fs-filename',
    'fs promises package',
    "var something = require('fs/promises'); something.readFile(filename);",
    ['Found readFile from package "fs/promises" with non literal argument at index 0'],
  ],
  [
    'detect-non-literal-fs-filename',
    'fs-extra package',
    "var something = require('fs-extra'); something.readFile(filename);",
    ['Found readFile from package "fs-extra" with non literal argument at index 0'],
  ],
  [
    'detect-non-literal-fs-filename',
    'destructured fs readFile',
    "var { readFile: something } = require('fs'); something(filename)",
    ['Found readFile from package "fs" with non literal argument at index 0'],
  ],
  [
    'detect-non-literal-fs-filename',
    'imported fs readFile',
    "import { readFile as something } from 'node:fs'; something(filename);",
    ['Found readFile from package "node:fs" with non literal argument at index 0'],
  ],
  [
    'detect-non-literal-fs-filename',
    'namespace fs readFile',
    "import * as something from 'fs'; something.readFile(filename);",
    ['Found readFile from package "fs" with non literal argument at index 0'],
  ],
  [
    'detect-non-literal-fs-filename',
    'require fs promises member',
    "var something = require('node:fs'); something.promises.readFile(filename)",
    ['Found readFile from package "node:fs" with non literal argument at index 0'],
  ],
  [
    'detect-non-literal-fs-filename',
    'template fs filename',
    "var fs = require('fs'); fs.readFile(`template with ${filename}`);",
    ['Found readFile from package "fs" with non literal argument at index 0'],
  ],
  [
    'detect-non-literal-regexp',
    'regexp variable',
    "var a = new RegExp(c, 'i')",
    ['Found non-literal argument to RegExp Constructor'],
  ],
  [
    'detect-non-literal-require',
    'require variable',
    'var a = require(c)',
    ['Found non-literal argument in require'],
  ],
  [
    'detect-non-literal-require',
    'require dynamic template',
    'var a = require(`${c}`)',
    ['Found non-literal argument in require'],
  ],
  [
    'detect-object-injection',
    'generic object injection',
    'var a = {}; a[b] = 4',
    ['Generic Object Injection Sink'],
  ],
  [
    'detect-object-injection',
    'function object injection',
    'obj[key]()',
    ['Function Call Object Injection Sink'],
  ],
  [
    'detect-possible-timing-attacks',
    'password left side',
    "if (password === 'mypass') {}",
    ['Potential timing attack, left side: true'],
  ],
  [
    'detect-possible-timing-attacks',
    'password right side',
    "if ('mypass' === password) {}",
    ['Potential timing attack, right side: true'],
  ],
  [
    'detect-pseudoRandomBytes',
    'pseudoRandomBytes member',
    'crypto.pseudoRandomBytes',
    ['Found crypto.pseudoRandomBytes which does not produce cryptographically strong numbers'],
  ],
  ['detect-unsafe-regex', 'unsafe regex literal', '/(x+x+)+y/', ['Unsafe Regular Expression']],
  [
    'detect-unsafe-regex',
    'unsafe RegExp constructor',
    "new RegExp('x+x+)+y')",
    ['Unsafe Regular Expression (new RegExp)'],
  ],
];

function runRule(ruleName, sourceText, filename = 'fixture.js') {
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
    options: [],
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function renderMessage(ruleName, report) {
  const template = plugin.rules[ruleName].meta.messages[report.messageId];
  return template.replace(/\{\{(\w+)\}\}/g, (_, key) => report.data?.[key] ?? '');
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

function runOxlint(ruleName, code) {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'security-plugin-'));

  try {
    const sourcePath = join(temp, 'fixture.js');
    const configPath = join(temp, 'oxlint.config.jsonc');

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'security',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`security/${ruleName}`]: 'error',
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

describe('security plugin shape', () => {
  it('exports the security plugin surface', () => {
    expect(plugin.meta?.name).toBe('security');
    expect(plugin.implementedSecurityRuleNames).toEqual(Object.keys(plugin.rules));
    expect(plugin.rules['detect-child-process'].meta.messages.execNonLiteral).toContain(
      'child_process.exec',
    );
  });

  it('ships upstream-compatible recommended configs', () => {
    expect(plugin.configs.recommended.rules['security/detect-child-process']).toBe('warn');
    expect(plugin.configs['recommended-legacy'].plugins).toEqual(['security']);
  });
});

describe('security rules through direct Oxlint plugin adapter', () => {
  it.each(validCases)('accepts %s: %s', (ruleName, _name, code) => {
    expect(runRule(ruleName, code)).toEqual([]);
  });

  it.each(invalidCases)('reports %s: %s', (ruleName, _name, code, expectedMessages) => {
    const reports = runRule(ruleName, code);

    expect(reports.map((report) => renderMessage(ruleName, report))).toEqual(expectedMessages);
  });
});

describe('security rules through oxlint jsPlugins', () => {
  it('reports a native diagnostic through the CLI', () => {
    const result = runOxlint('detect-non-literal-require', 'require(name);\n');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toMatchObject([
      {
        code: 'security(detect-non-literal-require)',
        message: 'Found non-literal argument in require',
      },
    ]);
  });
});
