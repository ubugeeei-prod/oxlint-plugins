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
  'component-class-suffix',
  'component-max-inline-declarations',
  'component-selector',
  'computed-must-return',
  'consistent-component-styles',
  'contextual-decorator',
  'contextual-lifecycle',
  'directive-class-suffix',
  'directive-selector',
  'no-async-lifecycle-method',
  'no-attribute-decorator',
  'no-developer-preview',
  'no-duplicates-in-metadata-arrays',
  'no-empty-lifecycle-method',
  'no-experimental',
  'no-forward-ref',
  'no-implicit-take-until-destroyed',
  'no-input-prefix',
  'no-input-rename',
  'no-inputs-metadata-property',
  'no-lifecycle-call',
  'no-output-native',
  'no-output-on-prefix',
  'no-output-rename',
  'no-outputs-metadata-property',
  'no-pipe-impure',
  'no-queries-metadata-property',
  'no-uncalled-signals',
  'pipe-prefix',
  'prefer-host-metadata-property',
  'prefer-inject',
  'prefer-on-push-component-change-detection',
  'prefer-output-emitter-ref',
  'prefer-output-readonly',
  'prefer-signal-model',
  'prefer-signals',
  'prefer-standalone',
  'relative-url-prefix',
  'require-lifecycle-on-prototype',
  'require-localize-metadata',
  'runtime-localize',
  'sort-keys-in-type-decorator',
  'sort-lifecycle-methods',
  'use-component-selector',
  'use-component-view-encapsulation',
  'use-injectable-provided-in',
  'use-lifecycle-interface',
  'use-pipe-transform-interface',
];

const invalidCases = [
  ['component-class-suffix', '@Component({ selector: "app-x" }) class App {}\n'],
  [
    'component-max-inline-declarations',
    '@Component({ template: `a\nb\nc` }) class AppComponent {}\n',
  ],
  ['computed-must-return', 'const total = computed(() => { totalSignal(); });\n'],
  ['consistent-component-styles', '@Component({ styleUrls: ["./x.css"] }) class AppComponent {}\n'],
  ['contextual-decorator', '@Input() class WrongContext {}\n'],
  ['contextual-lifecycle', 'class Plain { ngOnInit() {} }\n'],
  ['directive-class-suffix', '@Directive({ selector: "[x]" }) class Highlight {}\n'],
  ['no-async-lifecycle-method', 'class Life { async ngOnInit() {} }\n'],
  ['no-attribute-decorator', 'class Attr { constructor(@Attribute("role") role: string) {} }\n'],
  ['no-developer-preview', 'afterNextRender(() => {});\n'],
  [
    'no-duplicates-in-metadata-arrays',
    '@Component({ imports: [CommonModule, CommonModule] }) class C {}\n',
  ],
  ['no-empty-lifecycle-method', 'class Empty { ngOnDestroy() {} }\n'],
  ['no-experimental', 'resource(() => {});\n'],
  ['no-forward-ref', 'forwardRef(() => Service);\n'],
  ['no-implicit-take-until-destroyed', 'source.pipe(takeUntilDestroyed());\n'],
  ['no-input-prefix', 'class Inputs { @Input() isDisabled: boolean; }\n'],
  ['no-input-rename', 'class Inputs { @Input("renamed") name: string; }\n'],
  ['no-inputs-metadata-property', '@Component({ inputs: ["name"] }) class C {}\n'],
  ['no-lifecycle-call', 'class Caller { run() { this.ngOnInit(); } }\n'],
  ['no-output-native', 'class Outputs { @Output() click = new EventEmitter<void>(); }\n'],
  ['no-output-on-prefix', 'class Outputs { @Output() onSave = new EventEmitter<void>(); }\n'],
  ['no-output-rename', 'class Outputs { @Output("renamed") saved = new EventEmitter<void>(); }\n'],
  ['no-outputs-metadata-property', '@Component({ outputs: ["saved"] }) class C {}\n'],
  ['no-pipe-impure', '@Pipe({ name: "badPipe", pure: false }) class BadPipe { transform() {} }\n'],
  ['no-queries-metadata-property', '@Component({ queries: {} }) class C {}\n'],
  ['no-uncalled-signals', 'class SignalUser { run() { this.totalSignal; } }\n'],
  ['pipe-prefix', '@Pipe({ name: "badPipe" }) class BadPipe { transform() {} }\n'],
  ['prefer-host-metadata-property', 'class Host { @HostBinding("class.active") active = true; }\n'],
  ['prefer-inject', 'class Host { constructor(private service: Service) {} }\n'],
  [
    'prefer-on-push-component-change-detection',
    '@Component({ changeDetection: ChangeDetectionStrategy.Default }) class C {}\n',
  ],
  ['prefer-output-emitter-ref', 'class Emitter { saved = new EventEmitter<void>(); }\n'],
  ['prefer-output-readonly', 'class Emitter { @Output() saved = output<void>(); }\n'],
  [
    'prefer-signal-model',
    'class Model { @Input() value: string; @Output() valueChange = new EventEmitter<string>(); }\n',
  ],
  ['prefer-signals', 'class SignalInput { @Input() label: string; }\n'],
  ['prefer-standalone', '@Component({ standalone: false }) class C {}\n'],
  ['relative-url-prefix', '@Component({ templateUrl: "cmp.html" }) class C {}\n'],
  ['require-lifecycle-on-prototype', 'class LifecycleField { ngOnInit = () => {}; }\n'],
  ['require-localize-metadata', '$localize`Hello`;\n'],
  ['runtime-localize', '$localize.locale = "fr";\n'],
  [
    'sort-keys-in-type-decorator',
    '@Component({ template: "", selector: "app-sorted" }) class C {}\n',
  ],
  ['sort-lifecycle-methods', 'class C { ngOnDestroy() {} ngOnInit() {} }\n'],
  ['use-component-selector', '@Component({ template: "" }) class C {}\n'],
  [
    'use-component-view-encapsulation',
    '@Component({ encapsulation: ViewEncapsulation.None }) class C {}\n',
  ],
  ['use-injectable-provided-in', '@Injectable() class Service {}\n'],
  ['use-lifecycle-interface', 'class Plain { ngOnInit() {} }\n'],
  ['use-pipe-transform-interface', '@Pipe({ name: "plain" }) class PlainPipe { transform() {} }\n'],
];

function runRule(ruleName, sourceText, options = [], filename = 'fixture.ts') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[ruleName].createOnce({
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

describe('angular-eslint plugin adapter', () => {
  it('exposes rules and all config', () => {
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.configs.all.rules).toHaveProperty('@angular-eslint/component-class-suffix');
    expect(plugin.configs.all.plugins).toEqual(['@angular-eslint']);
  });

  it.each(invalidCases)('reports %s through direct createOnce', (ruleName, code) => {
    const reports = runRule(ruleName, code);

    expect(reports).toHaveLength(1);
    expect(plugin.rules[ruleName].meta.messages[reports[0].messageId]).toBe(
      'Unexpected Angular pattern.',
    );
  });

  it('exposes the complete selector schemas and messages', () => {
    for (const ruleName of ['component-selector', 'directive-selector']) {
      const { meta } = plugin.rules[ruleName];
      expect(meta.schema).toHaveLength(1);
      expect(meta.schema[0].oneOf).toHaveLength(2);
      expect(meta.schema[0].oneOf[0]).toMatchObject({
        type: 'object',
        required: ['type', 'style'],
        additionalProperties: false,
      });
      expect(meta.messages).toMatchObject({
        prefixFailure: expect.stringContaining('{{prefix}}'),
        styleFailure: expect.stringContaining('{{style}}'),
        typeFailure: expect.stringContaining('{{type}}'),
        selectorAfterPrefixFailure: expect.stringContaining('{{prefix}}'),
      });
    }
    expect(plugin.rules['component-selector'].meta.messages).toMatchObject({
      styleAndPrefixFailure: expect.stringContaining('{{style}}'),
      shadowDomEncapsulatedStyleFailure: expect.stringContaining('ShadowDom'),
    });
  });

  it.each([
    [
      'component-selector',
      '@Component({ selector: "app-user-card" }) class UserCard {}',
      [{ type: 'element', prefix: 'app', style: 'kebab-case' }],
    ],
    [
      'component-selector',
      '@Component({ selector: "[appUserCard]" }) class UserCard {}',
      [{ type: 'attribute', prefix: 'app', style: 'camelCase' }],
    ],
    [
      'component-selector',
      '@Component({ selector: "lib-user-card" }) class UserCard {}',
      [
        [
          { type: 'element', prefix: ['app', 'lib'], style: 'kebab-case' },
          { type: 'attribute', prefix: 'app', style: 'camelCase' },
        ],
      ],
    ],
    [
      'component-selector',
      '@Component({ selector: "app-user-card", encapsulation: ViewEncapsulation.ShadowDom }) class UserCard {}',
      [{ type: 'element', prefix: 'app', style: 'camelCase' }],
    ],
    [
      'directive-selector',
      '@Directive({ selector: "[appHighlight]" }) class HighlightDirective {}',
      [{ type: 'attribute', prefix: 'app', style: 'camelCase' }],
    ],
    [
      'directive-selector',
      '@Directive({ selector: "[lib-highlight]" }) class HighlightDirective {}',
      [{ type: 'attribute', prefix: ['app', 'lib'], style: 'kebab-case' }],
    ],
  ])('accepts configured %s selectors through createOnce', (ruleName, code, options) => {
    expect(runRule(ruleName, code, options)).toEqual([]);
  });

  it.each([
    [
      'component-selector',
      '@Component({ selector: "wrong-name" }) class UserCard {}',
      [{ type: 'element', prefix: 'app', style: 'kebab-case' }],
      'prefixFailure',
      { prefix: '"app"' },
    ],
    [
      'component-selector',
      '@Component({ selector: "appUserCard" }) class UserCard {}',
      [{ type: 'element', prefix: 'app', style: 'kebab-case' }],
      'styleAndPrefixFailure',
      { style: 'kebab-case', prefix: '"app"' },
    ],
    [
      'component-selector',
      '@Component({ selector: "[appUserCard]" }) class UserCard {}',
      [{ type: 'element', prefix: 'app', style: 'camelCase' }],
      'typeFailure',
      { type: 'element' },
    ],
    [
      'component-selector',
      '@Component({ selector: "app" }) class UserCard {}',
      [{ type: 'element', prefix: 'app', style: 'kebab-case' }],
      'selectorAfterPrefixFailure',
      { prefix: '"app"' },
    ],
    [
      'component-selector',
      '@Component({ selector: "appUserCard", encapsulation: ViewEncapsulation.ShadowDom }) class UserCard {}',
      [{ type: 'element', prefix: 'app', style: 'camelCase' }],
      'shadowDomEncapsulatedStyleFailure',
      {},
    ],
    [
      'directive-selector',
      '@Directive({ selector: "[wrongHighlight]" }) class HighlightDirective {}',
      [{ type: 'attribute', prefix: ['app', 'lib'], style: 'camelCase' }],
      'prefixFailure',
      { prefix: '"app" or "lib"' },
    ],
    [
      'directive-selector',
      '@Directive({ selector: "[app-highlight]" }) class HighlightDirective {}',
      [{ type: 'attribute', prefix: 'app', style: 'camelCase' }],
      'styleFailure',
      { style: 'camelCase' },
    ],
  ])(
    'reports configured %s selector failures through createOnce',
    (ruleName, code, options, messageId, data) => {
      expect(runRule(ruleName, code, options)).toMatchObject([{ messageId, data }]);
    },
  );

  it('does not run option-required selector rules without options', () => {
    expect(
      runRule('component-selector', '@Component({ selector: "WrongSelector" }) class UserCard {}'),
    ).toEqual([]);
    expect(
      runRule(
        'directive-selector',
        '@Directive({ selector: "WrongSelector" }) class HighlightDirective {}',
      ),
    ).toEqual([]);
  });

  it('loads through oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-angular-eslint-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.ts'),
        '@Component({ selector: "app-x" }) class App {}\n',
      );
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [
            {
              name: '@angular-eslint',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            '@angular-eslint/component-class-suffix': 'error',
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics).toHaveLength(1);
      expect(payload.diagnostics[0].message).toBe('Unexpected Angular pattern.');
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('honors selector options through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-angular-selector-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.ts'),
        '@Component({ selector: "wrong-name" }) class AppComponent {}\n',
      );
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [
            {
              name: '@angular-eslint',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            '@angular-eslint/component-selector': [
              'error',
              { type: 'element', prefix: ['app', 'lib'], style: 'kebab-case' },
            ],
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics).toMatchObject([
        {
          code: '@angular-eslint(component-selector)',
          message:
            'The selector should start with one of these prefixes: "app" or "lib" (https://angular.dev/style-guide#choosing-component-selectors)',
        },
      ]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});
