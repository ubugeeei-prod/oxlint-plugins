import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');
const consistentComponentStylesFixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/consistent-component-styles-v22.0.0.json', import.meta.url),
    'utf8',
  ),
);
const noInputRenameFixture = JSON.parse(
  readFileSync(new URL('./fixtures/no-input-rename-v22.0.0.json', import.meta.url), 'utf8'),
);
const preferSignalsFixture = JSON.parse(
  readFileSync(new URL('./fixtures/prefer-signals-v22.1.0.json', import.meta.url), 'utf8'),
);
const requireLocalizeMetadataFixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/require-localize-metadata-v22.1.0.json', import.meta.url),
    'utf8',
  ),
);
const playgroundCatalog = JSON.parse(
  readFileSync(resolve(workspaceRoot, 'playground/src/catalog.json'), 'utf8'),
);

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
    '@Component({ template: `a\nb\nc\nd` }) class AppComponent {}\n',
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

  it.each(
    invalidCases.filter(([ruleName]) => !['no-input-prefix', 'pipe-prefix'].includes(ruleName)),
  )('reports %s through direct createOnce', (ruleName, code) => {
    const reports = runRule(ruleName, code);

    expect(reports).toHaveLength(1);
    const expectedMessages = {
      'component-class-suffix':
        'Component class names should end with one of these suffixes: {{suffixes}}',
      'component-max-inline-declarations':
        '`{{propertyType}}` has too many lines ({{lineCount}}). Maximum allowed is {{max}}',
      'consistent-component-styles':
        'Use `styleUrl` instead of `styleUrls` for a single stylesheet',
      'directive-class-suffix':
        'Directive class names should end with one of these suffixes: {{suffixes}}',
      'no-input-rename':
        'Input bindings should not be aliased (https://angular.dev/guide/components/inputs#choosing-input-names)',
      'prefer-signals':
        'Use `InputSignal`s (e.g. via `input()`) for Component input properties rather than the legacy `@Input()` decorator',
    };
    expect(plugin.rules[ruleName].meta.messages[reports[0].messageId]).toBe(
      expectedMessages[ruleName] || 'Unexpected Angular pattern.',
    );
  });

  it('exposes complete class-suffix schemas and messages', () => {
    for (const ruleName of ['component-class-suffix', 'directive-class-suffix']) {
      const { meta } = plugin.rules[ruleName];
      expect(meta.schema).toEqual([
        {
          type: 'object',
          properties: {
            suffixes: {
              type: 'array',
              items: { type: 'string' },
            },
          },
          additionalProperties: false,
        },
      ]);
      expect(Object.values(meta.messages)).toEqual([expect.stringContaining('{{suffixes}}')]);
    }
  });

  it('exposes complete prefix schemas and messages', () => {
    for (const ruleName of ['no-input-prefix', 'pipe-prefix']) {
      const { meta } = plugin.rules[ruleName];
      expect(meta.schema).toHaveLength(1);
      expect(meta.schema[0]).toMatchObject({
        type: 'object',
        properties: {
          prefixes: {
            type: 'array',
            items: { type: 'string' },
          },
        },
        additionalProperties: false,
      });
      expect(Object.values(meta.messages).join(' ')).toContain('{{prefixes}}');
    }
    expect(plugin.rules['pipe-prefix'].meta.schema[0].properties.prefixes.uniqueItems).toBe(true);
    expect(plugin.rules['no-input-prefix'].meta.schema[0].properties.prefixes.uniqueItems).toBe(
      undefined,
    );
  });

  it('exposes the exact upstream consistent-component-styles contract', () => {
    const { meta } = plugin.rules['consistent-component-styles'];
    expect(meta.type).toBe('suggestion');
    expect(meta.docs.description).toBe(
      'Ensures consistent usage of `styles`/`styleUrls`/`styleUrl` within Component metadata',
    );
    expect(meta.schema).toEqual([
      {
        type: 'string',
        enum: ['array', 'string'],
      },
    ]);
    expect(meta.messages).toEqual({
      useStyleUrl: 'Use `styleUrl` instead of `styleUrls` for a single stylesheet',
      useStyleUrls: 'Use `styleUrls` instead of `styleUrl`',
      useStylesArray: 'Use a `string[]` instead of a `string` for the `styles` property',
      useStylesString: 'Use a `string` instead of a `string[]` for the `styles` property',
    });
    expect(meta.fixable).toBe('code');
    expect(meta.hasSuggestions).toBeUndefined();

    const playgroundRule = playgroundCatalog.plugins
      .find(({ plugin: pluginName }) => pluginName === '@angular-eslint')
      .rules.find(({ name }) => name === 'consistent-component-styles');
    expect(playgroundRule).toMatchObject({
      description: meta.docs.description,
      messages: meta.messages,
    });
  });

  it('exposes the exact upstream no-input-rename contract', () => {
    const { meta } = plugin.rules['no-input-rename'];
    expect(meta.schema).toEqual([
      {
        type: 'object',
        properties: {
          allowedNames: {
            type: 'array',
            items: { type: 'string' },
            description: 'A list with allowed input names',
            uniqueItems: true,
          },
        },
        additionalProperties: false,
      },
    ]);
    expect(meta.messages).toEqual({
      noInputRename:
        'Input bindings should not be aliased (https://angular.dev/guide/components/inputs#choosing-input-names)',
      suggestRemoveAliasName: 'Remove alias name',
      suggestReplaceOriginalNameWithAliasName: 'Remove alias name and use it as the original name',
    });
    expect(meta.fixable).toBe('code');
    expect(meta.hasSuggestions).toBe(true);
  });

  it('exposes the exact upstream prefer-signals contract and playground metadata', () => {
    const { meta } = plugin.rules['prefer-signals'];
    expect(meta.type).toBe('suggestion');
    expect(meta.docs.description).toBe(
      'Use readonly signals instead of `@Input()`, `@ViewChild()` and other legacy decorators',
    );
    expect(meta.schema).toEqual([
      {
        type: 'object',
        properties: {
          preferReadonlySignalProperties: { type: 'boolean', default: true },
          preferInputSignals: { type: 'boolean', default: true },
          preferQuerySignals: { type: 'boolean', default: true },
          useTypeChecking: { type: 'boolean', default: false },
          additionalSignalCreationFunctions: {
            type: 'array',
            items: { type: 'string' },
            default: [],
          },
        },
        additionalProperties: false,
      },
    ]);
    expect(meta.messages).toEqual({
      preferInputSignals:
        'Use `InputSignal`s (e.g. via `input()`) for Component input properties rather than the legacy `@Input()` decorator',
      preferQuerySignals:
        'Use the `{{function}}` function instead of the `{{decorator}}` decorator',
      preferReadonlySignalProperties:
        'Properties declared using signals should be marked as `readonly` since they should not be reassigned',
    });
    expect(meta.fixable).toBe('code');
    expect(meta.hasSuggestions).toBeUndefined();

    const playgroundRule = playgroundCatalog.plugins
      .find(({ plugin: pluginName }) => pluginName === '@angular-eslint')
      .rules.find(({ name }) => name === 'prefer-signals');
    expect(playgroundRule).toMatchObject({
      description: meta.docs.description,
      messages: meta.messages,
    });
  });

  it('exposes the complete component inline declaration contract', () => {
    const { meta } = plugin.rules['component-max-inline-declarations'];
    expect(meta.schema).toEqual([
      {
        type: 'object',
        properties: {
          template: { minimum: 0, type: 'number' },
          styles: { minimum: 0, type: 'number' },
          animations: { minimum: 0, type: 'number' },
        },
        additionalProperties: false,
      },
    ]);
    expect(meta.messages).toEqual({
      componentMaxInlineDeclarations:
        '`{{propertyType}}` has too many lines ({{lineCount}}). Maximum allowed is {{max}}',
    });
  });

  it.each([
    [
      '@Component({ template: `one\ntwo\nthree\nfour` }) class Test {}',
      [],
      [
        {
          messageId: 'componentMaxInlineDeclarations',
          data: { propertyType: 'template', lineCount: '4', max: '3' },
        },
      ],
    ],
    ['@Component({ template: `one\ntwo\nthree` }) class Test {}', [], []],
    [
      '@Component({ styles: [`one\ntwo`, `three\nfour`] }) class Test {}',
      [{ styles: 3 }],
      [
        {
          messageId: 'componentMaxInlineDeclarations',
          data: { propertyType: 'styles', lineCount: '4', max: '3' },
        },
      ],
    ],
    [
      '@Component({ animations: [one()] }) class Test {}',
      [{ animations: 0 }],
      [
        {
          messageId: 'componentMaxInlineDeclarations',
          data: { propertyType: 'animations', lineCount: '1', max: '0' },
        },
      ],
    ],
    [
      '@Component({ template: "one", styles: ["two"], animations: [three()] }) class Test {}',
      [{ template: 0, styles: 0, animations: 0 }],
      [
        {
          messageId: 'componentMaxInlineDeclarations',
          data: { propertyType: 'template', lineCount: '1', max: '0' },
        },
        {
          messageId: 'componentMaxInlineDeclarations',
          data: { propertyType: 'styles', lineCount: '1', max: '0' },
        },
        {
          messageId: 'componentMaxInlineDeclarations',
          data: { propertyType: 'animations', lineCount: '1', max: '0' },
        },
      ],
    ],
    ['@Directive({ template: `one\ntwo` }) class Test {}', [{ template: 0 }], []],
  ])(
    'honors component inline declaration options through createOnce',
    (code, options, expected) => {
      expect(runRule('component-max-inline-declarations', code, options)).toMatchObject(expected);
    },
  );

  it.each([
    [
      'no-input-prefix',
      '@Component() class Test { @Input() on: string; }',
      [{ prefixes: ['on'] }],
      [{ messageId: 'noInputPrefix', data: { prefixes: '"on"' } }],
    ],
    [
      'no-input-prefix',
      '@Injectable() class Test { @Input("on") isPrefix: string; }',
      [{ prefixes: ['on', 'is', 'should'] }],
      [
        { messageId: 'noInputPrefix', data: { prefixes: '"on", "is" or "should"' } },
        { messageId: 'noInputPrefix', data: { prefixes: '"on", "is" or "should"' } },
      ],
    ],
    [
      'no-input-prefix',
      '@Component({ inputs: ["value: onAlias"] }) class Test {}',
      [{ prefixes: ['on'] }],
      [{ messageId: 'noInputPrefix', data: { prefixes: '"on"' } }],
    ],
    [
      'pipe-prefix',
      '@Pipe({ name: "foo" }) class Test {}',
      [{ prefixes: ['ng', 'app'] }],
      [{ messageId: 'pipePrefix', data: { prefixes: '"ng" or "app"' } }],
    ],
    [
      'pipe-prefix',
      '@Pipe({ name: "ng" }) class Test {}',
      [{ prefixes: ['ng'] }],
      [{ messageId: 'selectorAfterPrefixFailure', data: { prefixes: '"ng"' } }],
    ],
    ['pipe-prefix', '@Pipe({ name: "ngTitle" }) class Test {}', [{ prefixes: ['ng'] }], []],
  ])('honors configured %s prefixes through createOnce', (ruleName, code, options, expected) => {
    expect(runRule(ruleName, code, options)).toMatchObject(expected);
  });

  it.each(consistentComponentStylesFixture.valid)(
    'accepts upstream consistent-component-styles valid case through createOnce: $name',
    ({ code, options }) => {
      expect(runRule('consistent-component-styles', code, options)).toEqual([]);
    },
  );

  it.each(consistentComponentStylesFixture.invalid)(
    'matches upstream consistent-component-styles invalid location through createOnce: $name',
    ({ code, errors, options }) => {
      expect(runRule('consistent-component-styles', code, options)).toEqual(
        errors.map((error) => ({
          messageId: error.messageId,
          data: {},
          loc: {
            start: {
              line: error.line,
              column: error.column - 1,
            },
            end: {
              line: error.endLine,
              column: error.endColumn - 1,
            },
          },
        })),
      );
    },
  );

  it('forwards consistent-component-styles modes independently through createOnce', () => {
    const code = `
@Component({
  styles: 'inline',
  styleUrl: 'one.css',
  styleUrls: ['two.css'],
})
class Test {}
`;
    expect(runRule('consistent-component-styles', code)).toMatchObject([
      { messageId: 'useStyleUrl' },
    ]);
    expect(runRule('consistent-component-styles', code, ['array'])).toMatchObject([
      { messageId: 'useStylesArray' },
      { messageId: 'useStyleUrls' },
    ]);
  });

  it('does not invent consistent-component-styles fixes outside the diagnostic ABI', () => {
    const reports = runRule(
      'consistent-component-styles',
      `@Component({ styles: ['inline'] }) class Test {}`,
    );
    expect(reports).toHaveLength(1);
    expect(reports[0]).not.toHaveProperty('fix');
    expect(reports[0]).not.toHaveProperty('suggest');
  });

  it.each(noInputRenameFixture.valid)(
    'accepts upstream no-input-rename valid case through createOnce: $name',
    ({ code, options }) => {
      expect(runRule('no-input-rename', code, options)).toEqual([]);
    },
  );

  it.each(noInputRenameFixture.invalid)(
    'matches upstream no-input-rename invalid location through createOnce: $name',
    ({ code, errors, options }) => {
      expect(runRule('no-input-rename', code, options)).toEqual(
        errors.map((error) => ({
          messageId: 'noInputRename',
          data: {},
          loc: {
            start: {
              line: error.line,
              column: error.column - 1,
            },
            end: {
              line: error.endLine,
              column: error.endColumn - 1,
            },
          },
        })),
      );
    },
  );

  it('forwards allowedNames independently for metadata, decorators, and signal inputs', () => {
    const code = `
@Component({ inputs: ['metadata: allowed'] })
class Test {
  @Input('allowed') decorated: string;
  signal = input(0, { alias: 'allowed' });
  required = input.required<string>({ alias: 'blocked' });
}
`;
    expect(runRule('no-input-rename', code, [{ allowedNames: ['allowed'] }])).toMatchObject([
      { messageId: 'noInputRename' },
    ]);
  });

  it('documents the adapter payload boundary without inventing fixes or suggestions', () => {
    const reports = runRule(
      'no-input-rename',
      `class Test { @Input('external') internal: string; }`,
    );
    expect(reports).toHaveLength(1);
    expect(reports[0]).not.toHaveProperty('fix');
    expect(reports[0]).not.toHaveProperty('suggest');
  });

  it.each([
    ['component-class-suffix', '@Component({}) class TestPage {}', [{ suffixes: ['Page'] }], []],
    [
      'component-class-suffix',
      '@Component({}) class TestPage {}',
      [{ suffixes: ['Component', 'View'] }],
      [{ messageId: 'componentClassSuffix', data: { suffixes: '"Component" or "View"' } }],
    ],
    [
      'directive-class-suffix',
      '@Directive({ selector: "[x]" }) class TestDir {}',
      [{ suffixes: ['Dir'] }],
      [],
    ],
    [
      'directive-class-suffix',
      '@Directive({ selector: "[x]" }) class TestDirectivePage implements AsyncValidator {}',
      [],
      [{ messageId: 'directiveClassSuffix', data: { suffixes: '"Directive" or "Validator"' } }],
    ],
    ['directive-class-suffix', '@Directive() class Wrong {}', [{ suffixes: [] }], []],
  ])('honors configured %s suffixes through createOnce', (ruleName, code, options, expected) => {
    expect(runRule(ruleName, code, options)).toMatchObject(expected);
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

  it('does not run option-required prefix rules without options', () => {
    expect(runRule('no-input-prefix', '@Component() class Test { @Input() on: string; }')).toEqual(
      [],
    );
    expect(runRule('pipe-prefix', '@Pipe({ name: "bad" }) class Test {}')).toEqual([]);
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
      expect(payload.diagnostics[0].message).toBe(
        'Component class names should end with one of these suffixes: "Component"',
      );
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('honors class-suffix options through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-angular-class-suffix-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.ts'),
        '@Component({ selector: "app-x" }) class AppComponent {}\n' +
          '@Component({ selector: "app-y" }) class SettingsPage {}\n',
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
            '@angular-eslint/component-class-suffix': ['error', { suffixes: ['Page', 'View'] }],
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
          code: '@angular-eslint(component-class-suffix)',
          message: 'Component class names should end with one of these suffixes: "Page" or "View"',
        },
      ]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('honors component inline declaration limits through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-angular-inline-declarations-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.ts'),
        '@Component({\n' +
          '  template: `one\ntwo`,\n' +
          '  styles: [`one\ntwo`],\n' +
          '  animations: [one()],\n' +
          '}) class Test {}\n',
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
            '@angular-eslint/component-max-inline-declarations': [
              'error',
              { template: 1, styles: 1, animations: 0 },
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
      expect(payload.diagnostics).toHaveLength(3);
      expect(payload.diagnostics.map(({ code }) => code)).toEqual([
        '@angular-eslint(component-max-inline-declarations)',
        '@angular-eslint(component-max-inline-declarations)',
        '@angular-eslint(component-max-inline-declarations)',
      ]);
      expect(payload.diagnostics.map(({ message }) => message)).toEqual([
        '`template` has too many lines (2). Maximum allowed is 1',
        '`styles` has too many lines (2). Maximum allowed is 1',
        '`animations` has too many lines (1). Maximum allowed is 0',
      ]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('honors prefix options through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-angular-prefix-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.ts'),
        '@Component() class Test { @Input("onAlias") isActive: boolean; }\n' +
          '@Pipe({ name: "plain" }) class PlainPipe {}\n',
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
            '@angular-eslint/no-input-prefix': ['error', { prefixes: ['on', 'is'] }],
            '@angular-eslint/pipe-prefix': ['error', { prefixes: ['app'] }],
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
      expect(payload.diagnostics).toHaveLength(3);
      expect(payload.diagnostics.map(({ code }) => code).sort()).toEqual([
        '@angular-eslint(no-input-prefix)',
        '@angular-eslint(no-input-prefix)',
        '@angular-eslint(pipe-prefix)',
      ]);
      expect(payload.diagnostics.map(({ message }) => message)).toEqual(
        expect.arrayContaining([
          'Input bindings, including aliases, should not be named, nor prefixed by "on" or "is"',
          '@Pipes should be prefixed with "app"',
        ]),
      );
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('honors consistent-component-styles mode through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-angular-consistent-styles-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.ts'),
        '@Component({\n' +
          '  styles: "inline",\n' +
          '  styleUrl: `one.css`,\n' +
          '  styleUrls: ["already-array.css"],\n' +
          '}) class Test {}\n',
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
            '@angular-eslint/consistent-component-styles': ['error', 'array'],
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
      expect(payload.diagnostics).toHaveLength(2);
      expect(payload.diagnostics.map(({ code }) => code)).toEqual([
        '@angular-eslint(consistent-component-styles)',
        '@angular-eslint(consistent-component-styles)',
      ]);
      expect(payload.diagnostics.map(({ message }) => message)).toEqual([
        'Use a `string[]` instead of a `string` for the `styles` property',
        'Use `styleUrls` instead of `styleUrl`',
      ]);
      expect(payload.diagnostics.map(({ labels }) => labels[0].span.line)).toEqual([2, 3]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('honors no-input-rename allowedNames through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-angular-no-input-rename-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.ts'),
        '@Component({ inputs: ["metadata: allowed"] })\n' +
          'class Test {\n' +
          '  @Input("allowed") decorated: string;\n' +
          '  signal = input(0, { alias: "blockedSignal" });\n' +
          '  required = input.required<string>({ alias: "blockedRequired" });\n' +
          '}\n',
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
            '@angular-eslint/no-input-rename': ['error', { allowedNames: ['allowed'] }],
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
      expect(payload.diagnostics).toHaveLength(2);
      expect(payload.diagnostics.map(({ code }) => code)).toEqual([
        '@angular-eslint(no-input-rename)',
        '@angular-eslint(no-input-rename)',
      ]);
      expect(payload.diagnostics.map(({ message }) => message)).toEqual([
        'Input bindings should not be aliased (https://angular.dev/guide/components/inputs#choosing-input-names)',
        'Input bindings should not be aliased (https://angular.dev/guide/components/inputs#choosing-input-names)',
      ]);
      expect(payload.diagnostics.map(({ labels }) => labels[0].span.line)).toEqual([4, 5]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it.each(preferSignalsFixture.valid)(
    'accepts upstream prefer-signals valid case through createOnce: $name',
    ({ code, options }) => {
      expect(runRule('prefer-signals', code, options)).toEqual([]);
    },
  );

  it.each(preferSignalsFixture.invalid)(
    'matches upstream prefer-signals invalid diagnostic through createOnce: $name',
    ({ code, errors, options }) => {
      expect(runRule('prefer-signals', code, options)).toEqual(
        errors.map((error) => ({
          messageId: error.messageId,
          data: error.data,
          loc: {
            start: {
              line: error.line,
              column: error.column - 1,
            },
            end: {
              line: error.endLine,
              column: error.endColumn - 1,
            },
          },
        })),
      );
    },
  );

  it('forwards every prefer-signals option independently through createOnce', () => {
    const code = `
class Test {
  @Input() custom = createCustomSignal();
  @ViewChildren('item') items: QueryList<Item>;
  typed = createTypedSignal();
}
declare function createTypedSignal(): Signal<boolean>;
`;
    expect(
      runRule('prefer-signals', code, [
        {
          preferInputSignals: false,
          preferQuerySignals: false,
          additionalSignalCreationFunctions: ['createCustomSignal'],
          useTypeChecking: true,
        },
      ]).map(({ messageId, loc }) => ({ messageId, line: loc.start.line })),
    ).toEqual([
      { messageId: 'preferReadonlySignalProperties', line: 3 },
      { messageId: 'preferReadonlySignalProperties', line: 5 },
    ]);
  });

  it('does not invent prefer-signals fixes outside the diagnostic ABI', () => {
    const reports = runRule(
      'prefer-signals',
      `class Test { @Input() value = signal(1); @ViewChild('x') child: Widget; }`,
    );
    expect(reports).toHaveLength(3);
    for (const report of reports) {
      expect(report).not.toHaveProperty('fix');
      expect(report).not.toHaveProperty('suggest');
    }
  });

  it('honors prefer-signals options through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-angular-prefer-signals-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.ts'),
        'class Test {\n' +
          '  custom = createCustomSignal();\n' +
          '  typed = createTypedSignal();\n' +
          '  @Input() legacy = 1;\n' +
          '  @ViewChildren("item") items: QueryList<Item>;\n' +
          '}\n' +
          'declare function createTypedSignal(): Signal<boolean>;\n',
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
            '@angular-eslint/prefer-signals': [
              'error',
              {
                preferInputSignals: false,
                additionalSignalCreationFunctions: ['createCustomSignal'],
                useTypeChecking: true,
              },
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
      expect(payload.diagnostics).toHaveLength(3);
      expect(payload.diagnostics.map(({ code }) => code)).toEqual([
        '@angular-eslint(prefer-signals)',
        '@angular-eslint(prefer-signals)',
        '@angular-eslint(prefer-signals)',
      ]);
      expect(payload.diagnostics.map(({ message }) => message)).toEqual([
        'Properties declared using signals should be marked as `readonly` since they should not be reassigned',
        'Properties declared using signals should be marked as `readonly` since they should not be reassigned',
        'Use the `viewChildren` function instead of the `ViewChildren` decorator',
      ]);
      expect(payload.diagnostics.map(({ labels }) => labels[0].span.line)).toEqual([2, 3, 5]);
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

describe('require-localize-metadata plugin contract', () => {
  it('exposes exact upstream metadata and matching playground catalog data', () => {
    const { meta } = plugin.rules['require-localize-metadata'];
    expect(meta.type).toBe('suggestion');
    expect(meta.docs.description).toBe(
      'Ensures that $localize tagged messages contain helpful metadata to aid with translations.',
    );
    expect(meta.schema).toEqual([
      {
        type: 'object',
        properties: {
          requireDescription: { type: 'boolean', default: false },
          requireMeaning: { type: 'boolean', default: false },
          requireCustomId: {
            oneOf: [{ type: 'boolean' }, { type: 'string' }],
            default: false,
          },
        },
        additionalProperties: false,
      },
    ]);
    expect(meta.messages).toEqual({
      requireLocalizeDescription:
        '$localize tagged messages should contain a description. See more at https://angular.dev/guide/i18n/prepare#i18n-metadata-for-translation',
      requireLocalizeMeaning:
        '$localize tagged messages should contain a meaning. See more at https://angular.dev/guide/i18n/prepare#i18n-metadata-for-translation',
      requireLocalizeCustomId:
        '$localize tagged messages should contain a custom id{{patternMessage}}. See more at https://angular.dev/guide/i18n/prepare#i18n-metadata-for-translation',
    });
    expect(meta.fixable).toBeUndefined();
    expect(meta.hasSuggestions).toBeUndefined();

    const playgroundRule = playgroundCatalog.plugins
      .find(({ plugin: pluginName }) => pluginName === '@angular-eslint')
      .rules.find(({ name }) => name === 'require-localize-metadata');
    expect(playgroundRule).toMatchObject({
      description: meta.docs.description,
      messages: meta.messages,
    });
  });

  it.each(requireLocalizeMetadataFixture.valid)(
    'accepts upstream require-localize-metadata valid case through createOnce: $name',
    ({ code, options }) => {
      expect(runRule('require-localize-metadata', code, options)).toEqual([]);
    },
  );

  it.each(requireLocalizeMetadataFixture.invalid)(
    'matches upstream require-localize-metadata diagnostic through createOnce: $name',
    ({ code, errors, options }) => {
      expect(runRule('require-localize-metadata', code, options)).toEqual(
        errors.map((error) => ({
          messageId: error.messageId,
          data: error.data,
          loc: {
            start: {
              line: error.line,
              column: error.column - 1,
            },
            end: {
              line: error.endLine,
              column: error.endColumn - 1,
            },
          },
        })),
      );
    },
  );

  it('forwards defaults, independent options, exact order, data, and locations', () => {
    expect(runRule('require-localize-metadata', '$localize`Hello`;')).toEqual([]);
    const reports = runRule('require-localize-metadata', '$localize`Hello ${name}`;', [
      {
        requireDescription: true,
        requireMeaning: true,
        requireCustomId: '^stable$',
      },
    ]);
    expect(reports).toEqual([
      {
        messageId: 'requireLocalizeDescription',
        data: {},
        loc: {
          start: { line: 1, column: 9 },
          end: { line: 1, column: 18 },
        },
      },
      {
        messageId: 'requireLocalizeMeaning',
        data: {},
        loc: {
          start: { line: 1, column: 9 },
          end: { line: 1, column: 18 },
        },
      },
      {
        messageId: 'requireLocalizeCustomId',
        data: {
          patternMessage: " matching the pattern /^stable$/ on 'undefined'",
        },
        loc: {
          start: { line: 1, column: 9 },
          end: { line: 1, column: 18 },
        },
      },
    ]);
    for (const report of reports) {
      expect(report).not.toHaveProperty('fix');
      expect(report).not.toHaveProperty('suggest');
    }
  });

  it('honors all options through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-angular-require-localize-metadata-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.ts'),
        '$localize`Hello`;\n' + '$localize`:meaning|description@@wrong:World`;\n',
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
            '@angular-eslint/require-localize-metadata': [
              'error',
              {
                requireDescription: true,
                requireMeaning: true,
                requireCustomId: '^stable$',
              },
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
      expect(payload.diagnostics).toHaveLength(4);
      expect(payload.diagnostics.map(({ code }) => code)).toEqual(
        Array(4).fill('@angular-eslint(require-localize-metadata)'),
      );
      expect(payload.diagnostics.map(({ labels }) => labels[0].span.line)).toEqual([1, 1, 1, 2]);
      expect(payload.diagnostics.map(({ message }) => message)).toEqual([
        plugin.rules['require-localize-metadata'].meta.messages.requireLocalizeDescription,
        plugin.rules['require-localize-metadata'].meta.messages.requireLocalizeMeaning,
        '$localize tagged messages should contain a custom id matching the pattern /^stable$/ on ' +
          "'undefined'. See more at https://angular.dev/guide/i18n/prepare#i18n-metadata-for-translation",
        '$localize tagged messages should contain a custom id matching the pattern /^stable$/ on ' +
          "'wrong'. See more at https://angular.dev/guide/i18n/prepare#i18n-metadata-for-translation",
      ]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});
