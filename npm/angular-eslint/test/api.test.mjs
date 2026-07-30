import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import { implementedAngularEslintRuleNames, scanAngularEslint } from '../api.js';

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

const representativeSource = `
@Component({ selector: "BadSelector", template: \`a
b
c
d\` }) class App {}
const total = computed(() => { totalSignal(); });
@Component({ styleUrls: ["./x.css"] }) class StyleComponent {}
@Input() class WrongContext {}
class Plain { ngOnInit() {} }
@Directive({ selector: "BadDirective" }) class Highlight {}
class Life { async ngOnInit() {} }
class Attr { constructor(@Attribute("role") role: string) {} }
afterNextRender(() => {});
@Component({ imports: [CommonModule, CommonModule] }) class DupComponent {}
class Empty { ngOnDestroy() {} }
resource(() => {});
forwardRef(() => Service);
takeUntilDestroyed();
class Inputs { @Input() isDisabled: boolean; @Input("renamed") name: string; }
@Component({ inputs: ["name"], outputs: ["saved"], queries: {} }) class MetadataComponent {}
class Caller { run() { this.ngOnInit(); } }
class Outputs { @Output() click = new EventEmitter<void>(); @Output() onSave = new EventEmitter<void>(); @Output("renamed") saved = new EventEmitter<void>(); }
@Pipe({ name: "badPipe", pure: false }) class BadPipe { transform() {} }
class SignalUser { run() { this.totalSignal; } }
class Host { @HostBinding("class.active") active = true; constructor(private service: Service) {} }
@Component({ changeDetection: ChangeDetectionStrategy.Default, standalone: false, templateUrl: "cmp.html", encapsulation: ViewEncapsulation.None }) class OldComponent {}
class Emitter { @Output() saved = new EventEmitter<void>(); }
class Model { @Input() value: string; @Output() valueChange = new EventEmitter<string>(); }
class SignalInput { @Input() label: string; }
class LifecycleField { ngOnInit = () => {}; }
$localize\`Hello\`;
$localize.locale = "fr";
@Component({ template: "", selector: "app-sorted" }) class SortComponent { ngOnDestroy() {} ngOnInit() {} }
@Component({ template: "" }) class MissingSelectorComponent {}
@Injectable() class Service {}
@Pipe({ name: "plain" }) class PlainPipe { transform() {} }
`;

describe('angular-eslint native API', () => {
  it('exposes all @angular-eslint/eslint-plugin rule names', () => {
    expect(implementedAngularEslintRuleNames()).toEqual(expectedRuleNames);
  });

  it('scans representative Angular patterns for every rule', () => {
    const diagnostics = scanAngularEslint(representativeSource, 'fixture.ts');

    expect([...new Set(diagnostics.map((diagnostic) => diagnostic.ruleName))].sort()).toEqual(
      expectedRuleNames
        .filter(
          (ruleName) =>
            ![
              'component-selector',
              'directive-selector',
              'no-input-prefix',
              'pipe-prefix',
              'require-localize-metadata',
            ].includes(ruleName),
        )
        .sort(),
    );
  });

  it.each([
    [
      'component-selector',
      '@Component({ selector: "app-user-card" }) class UserCard {}',
      { type: 'element', prefix: 'app', style: 'kebab-case' },
      [],
    ],
    [
      'component-selector',
      '@Component({ selector: "wrong-name" }) class UserCard {}',
      { type: 'element', prefix: ['app', 'lib'], style: 'kebab-case' },
      [{ messageId: 'prefixFailure', data: [{ key: 'prefix', value: '"app" or "lib"' }] }],
    ],
    [
      'component-selector',
      '@Component({ selector: "appUserCard" }) class UserCard {}',
      { type: 'element', prefix: 'app', style: 'kebab-case' },
      [
        {
          messageId: 'styleAndPrefixFailure',
          data: [
            { key: 'style', value: 'kebab-case' },
            { key: 'prefix', value: '"app"' },
          ],
        },
      ],
    ],
    [
      'component-selector',
      '@Component({ selector: "[appUserCard]" }) class UserCard {}',
      { type: 'element', prefix: 'app', style: 'camelCase' },
      [{ messageId: 'typeFailure', data: [{ key: 'type', value: 'element' }] }],
    ],
    [
      'component-selector',
      '@Component({ selector: "appUserCard", encapsulation: ViewEncapsulation.ShadowDom }) class UserCard {}',
      { type: 'element', prefix: 'app', style: 'camelCase' },
      [{ messageId: 'shadowDomEncapsulatedStyleFailure', data: [] }],
    ],
    [
      'directive-selector',
      '@Directive({ selector: "[appHighlight]" }) class HighlightDirective {}',
      { type: 'attribute', prefix: 'app', style: 'camelCase' },
      [],
    ],
    [
      'directive-selector',
      '@Directive({ selector: "[app-highlight]" }) class HighlightDirective {}',
      { type: 'attribute', prefix: 'app', style: 'camelCase' },
      [{ messageId: 'styleFailure', data: [{ key: 'style', value: 'camelCase' }] }],
    ],
  ])('honors %s native options and data', (ruleName, source, option, expected) => {
    const diagnostics = scanAngularEslint(source, 'fixture.ts', {
      ruleNames: [ruleName],
      options: [option],
    });

    expect(diagnostics).toMatchObject(expected);
  });

  it('supports separate element and attribute selector configs', () => {
    const options = [
      [
        { type: 'element', prefix: 'app', style: 'kebab-case' },
        { type: 'attribute', prefix: 'lib', style: 'camelCase' },
      ],
    ];
    expect(
      scanAngularEslint(
        '@Component({ selector: "app-user-card" }) class UserCard {}',
        'fixture.ts',
        { ruleNames: ['component-selector'], options },
      ),
    ).toEqual([]);
    expect(
      scanAngularEslint(
        '@Component({ selector: "[libUserCard]" }) class UserCard {}',
        'fixture.ts',
        { ruleNames: ['component-selector'], options },
      ),
    ).toEqual([]);
  });

  it.each([
    ['component-class-suffix', '@Component({}) class TestPage {}', [{ suffixes: ['Page'] }], []],
    [
      'component-class-suffix',
      '@Component({}) class TestPage {}',
      [{ suffixes: ['Component', 'View'] }],
      [
        {
          messageId: 'componentClassSuffix',
          data: [{ key: 'suffixes', value: '"Component" or "View"' }],
        },
      ],
    ],
    ['component-class-suffix', '@Component({}) class TestComponent {}', [], []],
    [
      'component-class-suffix',
      '@Component({}) class TestComponent {}',
      [{ suffixes: [] }],
      [
        {
          messageId: 'componentClassSuffix',
          data: [{ key: 'suffixes', value: '' }],
        },
      ],
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
      [
        {
          messageId: 'directiveClassSuffix',
          data: [{ key: 'suffixes', value: '"Directive" or "Validator"' }],
        },
      ],
    ],
    [
      'directive-class-suffix',
      '@Directive({ selector: "[x]" }) class TestValidator implements forms.AsyncValidator {}',
      [],
      [],
    ],
    ['directive-class-suffix', '@Directive() class Test {}', [{ suffixes: [] }], []],
  ])('honors %s suffix options through the native API', (ruleName, source, options, expected) => {
    expect(
      scanAngularEslint(source, 'fixture.ts', {
        ruleNames: [ruleName],
        options,
      }),
    ).toMatchObject(expected);
  });

  it.each([
    [
      'no-input-prefix',
      '@Component() class Test { @Input() on: string; }',
      [{ prefixes: ['on'] }],
      [{ messageId: 'noInputPrefix', data: [{ key: 'prefixes', value: '"on"' }] }],
    ],
    [
      'no-input-prefix',
      '@Directive() class Test { @Input({ alias: "onPrefix" }) value: string; }',
      [{ prefixes: ['on'] }],
      [{ messageId: 'noInputPrefix', data: [{ key: 'prefixes', value: '"on"' }] }],
    ],
    [
      'no-input-prefix',
      '@Component({ inputs: ["onTest: value"] }) class Test {}',
      [{ prefixes: ['on'] }],
      [{ messageId: 'noInputPrefix', data: [{ key: 'prefixes', value: '"on"' }] }],
    ],
    [
      'no-input-prefix',
      '@Injectable() class Test { @Input("on") isPrefix: string; }',
      [{ prefixes: ['on', 'is', 'should'] }],
      [
        {
          messageId: 'noInputPrefix',
          data: [{ key: 'prefixes', value: '"on", "is" or "should"' }],
        },
        {
          messageId: 'noInputPrefix',
          data: [{ key: 'prefixes', value: '"on", "is" or "should"' }],
        },
      ],
    ],
    ['no-input-prefix', '@Component() class Test { @Input() ontype: string; }', [], []],
    [
      'pipe-prefix',
      '@Pipe({ name: "foo-bar" }) class Test {}',
      [{ prefixes: ['ng'] }],
      [{ messageId: 'pipePrefix', data: [{ key: 'prefixes', value: '"ng"' }] }],
    ],
    [
      'pipe-prefix',
      '@Pipe({ name: "ng" }) class Test {}',
      [{ prefixes: ['ng'] }],
      [
        {
          messageId: 'selectorAfterPrefixFailure',
          data: [{ key: 'prefixes', value: '"ng"' }],
        },
      ],
    ],
    [
      'pipe-prefix',
      '@Pipe({ name: `ngBarFoo` }) class Test {}',
      [{ prefixes: ['ng', 'sg', 'mg'] }],
      [],
    ],
    ['pipe-prefix', '@Pipe({ name: "bad" }) class Test {}', [{ prefixes: [] }], []],
  ])('honors %s prefix options through the native API', (ruleName, source, options, expected) => {
    expect(
      scanAngularEslint(source, 'fixture.ts', {
        ruleNames: [ruleName],
        options,
      }),
    ).toMatchObject(expected);
  });

  it('pins every authored consistent-component-styles case from angular-eslint v22.0.0', () => {
    expect(consistentComponentStylesFixture.metadata).toEqual({
      package: '@angular-eslint/eslint-plugin',
      version: '22.0.0',
      sourceCommit: '7ee4556badebf8c140ffdefdd0b07b02820d5e96',
      sourcePath: 'packages/eslint-plugin/tests/rules/consistent-component-styles/cases.ts',
      sourceSha256: '440ee39c6dd952eaac1f732c8d7f5428ea12ec65e85c617980523db6eb5d410e',
      capture: 'every authored valid and invalid semantic case exactly once',
      counts: {
        valid: 21,
        invalid: 20,
        diagnostics: 20,
      },
    });
  });

  it.each(consistentComponentStylesFixture.valid)(
    'accepts upstream consistent-component-styles valid case: $name',
    ({ code, options }) => {
      expect(
        scanAngularEslint(code, 'fixture.ts', {
          ruleNames: ['consistent-component-styles'],
          options,
        }),
      ).toEqual([]);
    },
  );

  it.each(consistentComponentStylesFixture.invalid)(
    'matches upstream consistent-component-styles invalid diagnostic: $name',
    ({ code, errors, options }) => {
      expect(
        scanAngularEslint(code, 'fixture.ts', {
          ruleNames: ['consistent-component-styles'],
          options,
        }),
      ).toEqual(
        errors.map((error) => ({
          ruleName: 'consistent-component-styles',
          messageId: error.messageId,
          data: [],
          loc: {
            startLine: error.line,
            startColumn: error.column - 1,
            endLine: error.endLine,
            endColumn: error.endColumn - 1,
          },
        })),
      );
    },
  );

  it('reports all consistent-component-styles shapes in metadata source order', () => {
    const source = `
@Component({
  styles: ['inline'],
  styleUrls: ['one.css'],
  nested: { styles: ['nested'], styleUrls: ['nested.css'] },
})
class StringMode {}
`;
    expect(
      scanAngularEslint(source, 'fixture.ts', {
        ruleNames: ['consistent-component-styles'],
        options: ['string'],
      }).map(({ messageId, loc }) => ({ messageId, line: loc.startLine })),
    ).toEqual([
      { messageId: 'useStylesString', line: 3 },
      { messageId: 'useStyleUrl', line: 4 },
      { messageId: 'useStylesString', line: 5 },
      { messageId: 'useStyleUrl', line: 5 },
    ]);

    expect(
      scanAngularEslint(
        `@Component({ styles: \`inline\`, styleUrl: choose('one.css') }) class ArrayMode {}`,
        'fixture.ts',
        {
          ruleNames: ['consistent-component-styles'],
          options: ['array'],
        },
      ).map(({ messageId }) => messageId),
    ).toEqual(['useStylesArray', 'useStyleUrls']);
  });

  it('keeps consistent-component-styles isolated and fails closed on malformed TypeScript', () => {
    const source = `@Component({ styleUrls: ['one.css'] }) class Test {}`;
    expect(
      scanAngularEslint(source, 'fixture.ts', {
        ruleNames: ['no-output-rename'],
        options: [],
      }).some(({ ruleName }) => ruleName === 'consistent-component-styles'),
    ).toBe(false);
    expect(
      scanAngularEslint(`@Component({ styleUrls: ['one.css']`, 'fixture.ts', {
        ruleNames: ['consistent-component-styles'],
        options: [],
      }),
    ).toEqual([]);
  });

  it('preserves UTF-16 columns for consistent-component-styles reports', () => {
    expect(
      scanAngularEslint(
        `@Component({ marker: '😀', styles: ['x'], styleUrls: ['x.css'] }) class Test {}`,
        'fixture.ts',
        {
          ruleNames: ['consistent-component-styles'],
          options: [],
        },
      ),
    ).toMatchObject([
      {
        messageId: 'useStylesString',
        loc: { startLine: 1, startColumn: 35, endLine: 1, endColumn: 40 },
      },
      {
        messageId: 'useStyleUrl',
        loc: { startLine: 1, startColumn: 42, endLine: 1, endColumn: 62 },
      },
    ]);
  });

  it('pins every authored no-input-rename case from angular-eslint v22.0.0', () => {
    expect(noInputRenameFixture.metadata).toMatchObject({
      version: '22.0.0',
      sourceCommit: '7ee4556badebf8c140ffdefdd0b07b02820d5e96',
      counts: {
        valid: 46,
        invalid: 35,
        diagnostics: 35,
      },
    });
  });

  it.each(noInputRenameFixture.valid)(
    'accepts upstream no-input-rename valid case: $name',
    ({ code, options }) => {
      expect(
        scanAngularEslint(code, 'fixture.ts', {
          ruleNames: ['no-input-rename'],
          options,
        }),
      ).toEqual([]);
    },
  );

  it.each(noInputRenameFixture.invalid)(
    'matches upstream no-input-rename invalid location: $name',
    ({ code, errors, options }) => {
      const diagnostics = scanAngularEslint(code, 'fixture.ts', {
        ruleNames: ['no-input-rename'],
        options,
      });
      expect(diagnostics).toHaveLength(errors.length);
      expect(diagnostics).toEqual(
        errors.map((error) => ({
          ruleName: 'no-input-rename',
          messageId: 'noInputRename',
          data: [],
          loc: {
            startLine: error.line,
            startColumn: error.column - 1,
            endLine: error.endLine,
            endColumn: error.endColumn - 1,
          },
        })),
      );
    },
  );

  it('reports decorator, signal, required signal, and metadata aliases in source order', () => {
    const source = `
@Component({ inputs: ['metadata: publicMetadata'] })
class Test {
  @Input('publicDecorator') decorator: string;
  signal = input(0, { alias: 'publicSignal' });
  required = input.required<string>({ alias: 'publicRequired' });
}
`;
    const diagnostics = scanAngularEslint(source, 'fixture.ts', {
      ruleNames: ['no-input-rename'],
      options: [],
    });
    expect(diagnostics).toHaveLength(4);
    expect(diagnostics.map(({ messageId }) => messageId)).toEqual([
      'noInputRename',
      'noInputRename',
      'noInputRename',
      'noInputRename',
    ]);
    expect(diagnostics.map(({ loc }) => loc.startLine)).toEqual([2, 4, 5, 6]);
  });

  it('keeps no-input-rename isolated and fails closed on malformed TypeScript', () => {
    const source = `class Test { @Input('renamed') name: string; }`;
    expect(
      scanAngularEslint(source, 'fixture.ts', {
        ruleNames: ['no-output-rename'],
        options: [],
      }).some(({ ruleName }) => ruleName === 'no-input-rename'),
    ).toBe(false);
    expect(
      scanAngularEslint(`class Test { @Input('renamed'`, 'fixture.ts', {
        ruleNames: ['no-input-rename'],
        options: [],
      }),
    ).toEqual([]);
  });

  it('preserves UTF-16 columns for no-input-rename aliases', () => {
    expect(
      scanAngularEslint(
        `class Test { emoji = '😀'; @Input('renamed') name: string; }`,
        'fixture.ts',
        {
          ruleNames: ['no-input-rename'],
          options: [],
        },
      ),
    ).toMatchObject([
      {
        messageId: 'noInputRename',
        loc: {
          startLine: 1,
          startColumn: 34,
          endLine: 1,
          endColumn: 43,
        },
      },
    ]);
  });

  it('does not mistake computed identifier keys for static Angular metadata', () => {
    const source = `
const selector = 'selector';
const inputs = 'inputs';
@Directive({ [selector]: 'publicName', [inputs]: ['internal: publicName'] })
class Test { @Input('publicName') internal: string; }
`;
    expect(
      scanAngularEslint(source, 'fixture.ts', {
        ruleNames: ['no-input-rename'],
        options: [],
      }),
    ).toMatchObject([{ messageId: 'noInputRename', loc: { startLine: 5 } }]);
  });

  it.each([
    [
      '@Component({ template: `one\ntwo\nthree\nfour` }) class Test {}',
      [],
      [
        {
          messageId: 'componentMaxInlineDeclarations',
          data: [
            { key: 'propertyType', value: 'template' },
            { key: 'lineCount', value: '4' },
            { key: 'max', value: '3' },
          ],
        },
      ],
    ],
    ['@Component({ template: `one\ntwo\nthree` }) class Test {}', [], []],
    [
      '@Component({ styles: ["one"] }) class Test {}',
      [{ styles: 0 }],
      [
        {
          messageId: 'componentMaxInlineDeclarations',
          data: [
            { key: 'propertyType', value: 'styles' },
            { key: 'lineCount', value: '1' },
            { key: 'max', value: '0' },
          ],
        },
      ],
    ],
    [
      '@Component({ animations: [one()] }) class Test {}',
      [{ animations: 0 }],
      [
        {
          messageId: 'componentMaxInlineDeclarations',
          data: [
            { key: 'propertyType', value: 'animations' },
            { key: 'lineCount', value: '1' },
            { key: 'max', value: '0' },
          ],
        },
      ],
    ],
    [
      '@Component({ styles: [`one\ntwo`, `three\nfour`] }) class Test {}',
      [{ template: 0 }],
      [
        {
          messageId: 'componentMaxInlineDeclarations',
          data: [
            { key: 'propertyType', value: 'styles' },
            { key: 'lineCount', value: '4' },
            { key: 'max', value: '3' },
          ],
        },
      ],
    ],
    ['@Component({ styles, animations: [], template }) class Test {}', [{ styles: 0 }], []],
  ])(
    'honors component inline declaration limits through the native API',
    (source, options, expected) => {
      expect(
        scanAngularEslint(source, 'fixture.ts', {
          ruleNames: ['component-max-inline-declarations'],
          options,
        }),
      ).toMatchObject(expected);
    },
  );

  it('returns LSP-shaped locations', () => {
    const [diagnostic] = scanAngularEslint(
      '@Component({ selector: "app-x" }) class App {}\n',
      'fixture.ts',
    );

    expect(diagnostic).toMatchObject({
      ruleName: 'component-class-suffix',
      messageId: 'componentClassSuffix',
      data: [{ key: 'suffixes', value: '"Component"' }],
      loc: {
        startLine: 1,
        endLine: 1,
      },
    });
  });

  it('pins every authored prefer-signals case from angular-eslint v22.1.0', () => {
    expect(preferSignalsFixture.metadata).toEqual({
      package: '@angular-eslint/eslint-plugin',
      version: '22.1.0',
      sourceCommit: 'a666e1b45c9782d1ac2066fd55ec0127d0580950',
      sourceTag: 'v22.1.0',
      sourcePath: 'packages/eslint-plugin/tests/rules/prefer-signals/cases.ts',
      sourceSha256: '33cae0732a9d9a41e1dd943ee8a19e282e240a06a247a31a0dac942fcec96cae',
      capture: 'every authored valid and invalid semantic case exactly once',
      counts: {
        valid: 39,
        invalid: 26,
        diagnostics: 26,
      },
    });
  });

  it.each(preferSignalsFixture.valid)(
    'accepts upstream prefer-signals valid case: $name',
    ({ code, options }) => {
      expect(
        scanAngularEslint(code, 'fixture.ts', {
          ruleNames: ['prefer-signals'],
          options,
        }),
      ).toEqual([]);
    },
  );

  it.each(preferSignalsFixture.invalid)(
    'matches upstream prefer-signals invalid diagnostic: $name',
    ({ code, errors, options }) => {
      expect(
        scanAngularEslint(code, 'fixture.ts', {
          ruleNames: ['prefer-signals'],
          options,
        }),
      ).toEqual(
        errors.map((error) => ({
          ruleName: 'prefer-signals',
          messageId: error.messageId,
          data: Object.entries(error.data).map(([key, value]) => ({ key, value })),
          loc: {
            startLine: error.line,
            startColumn: error.column - 1,
            endLine: error.endLine,
            endColumn: error.endColumn - 1,
          },
        })),
      );
    },
  );

  it('reports readonly, input, and query branches in upstream traversal order', () => {
    const diagnostics = scanAngularEslint(
      `class Test {
  @Input() signalValue = signal(1);
  @ViewChild('child') child: Widget;
  plain = contentChildren('items');
}`,
      'fixture.ts',
      { ruleNames: ['prefer-signals'], options: [] },
    );

    expect(
      diagnostics.map(({ messageId, data, loc }) => ({
        messageId,
        data,
        line: loc.startLine,
      })),
    ).toEqual([
      { messageId: 'preferReadonlySignalProperties', data: [], line: 2 },
      { messageId: 'preferInputSignals', data: [], line: 2 },
      {
        messageId: 'preferQuerySignals',
        data: [
          { key: 'function', value: 'viewChild' },
          { key: 'decorator', value: 'ViewChild' },
        ],
        line: 3,
      },
      { messageId: 'preferReadonlySignalProperties', data: [], line: 4 },
    ]);
  });

  it.each([
    [[{ preferReadonlySignalProperties: false }], ['preferInputSignals', 'preferQuerySignals']],
    [[{ preferInputSignals: false }], ['preferReadonlySignalProperties', 'preferQuerySignals']],
    [[{ preferQuerySignals: false }], ['preferReadonlySignalProperties', 'preferInputSignals']],
    [
      [
        {
          preferReadonlySignalProperties: false,
          preferInputSignals: false,
          preferQuerySignals: false,
        },
      ],
      [],
    ],
  ])('isolates prefer-signals option branches for %j', (options, messageIds) => {
    const diagnostics = scanAngularEslint(
      `class Test {
  @Input() value = signal(1);
  @ContentChild('child') child: Widget;
}`,
      'fixture.ts',
      { ruleNames: ['prefer-signals'], options },
    );
    expect(diagnostics.map(({ messageId }) => messageId)).toEqual(messageIds);
  });

  it('supports additional factories and source-local type checking independently', () => {
    const source = `
class Test {
  custom = createCustomSignal();
  typed = createTypedSignal();
}
declare function createTypedSignal(): Signal<boolean>;
`;
    expect(
      scanAngularEslint(source, 'fixture.ts', {
        ruleNames: ['prefer-signals'],
        options: [{ additionalSignalCreationFunctions: ['createCustomSignal'] }],
      }).map(({ loc }) => loc.startLine),
    ).toEqual([3]);
    expect(
      scanAngularEslint(source, 'fixture.ts', {
        ruleNames: ['prefer-signals'],
        options: [{ useTypeChecking: true }],
      }).map(({ loc }) => loc.startLine),
    ).toEqual([4]);
  });

  it('preserves UTF-16 columns and ignores non-Angular state factories', () => {
    const [diagnostic] = scanAngularEslint(
      `class Test { marker = '😀'; signalValue = signal(1); state = useState(0); }`,
      'fixture.ts',
      { ruleNames: ['prefer-signals'], options: [] },
    );
    expect(diagnostic).toMatchObject({
      messageId: 'preferReadonlySignalProperties',
      loc: {
        startLine: 1,
        startColumn: 28,
        endLine: 1,
        endColumn: 39,
      },
    });
  });

  it('keeps prefer-signals isolated and fails closed on malformed TypeScript', () => {
    expect(
      scanAngularEslint(`class Test { value = signal(1); }`, 'fixture.ts', {
        ruleNames: ['no-output-rename'],
        options: [],
      }).some(({ ruleName }) => ruleName === 'prefer-signals'),
    ).toBe(false);
    expect(
      scanAngularEslint(`class Test { value = signal(`, 'fixture.ts', {
        ruleNames: ['prefer-signals'],
        options: [],
      }),
    ).toEqual([]);
  });
});

describe('require-localize-metadata native API', () => {
  it('pins every authored angular-eslint v22.1.0 case deterministically', () => {
    expect(requireLocalizeMetadataFixture.metadata).toEqual({
      package: '@angular-eslint/eslint-plugin',
      version: '22.1.0',
      sourceCommit: 'a666e1b45c9782d1ac2066fd55ec0127d0580950',
      sourceTag: 'v22.1.0',
      sourcePath: 'packages/eslint-plugin/tests/rules/require-localize-metadata/cases.ts',
      sourceSha256: '532a9e3c1d93294fd7245b183eab26098217f6971c3fa1fddb230dc5cc904faa',
      capture: 'every authored valid and invalid semantic case exactly once',
      counts: {
        valid: 13,
        invalid: 15,
        diagnostics: 16,
      },
    });
  });

  it.each(requireLocalizeMetadataFixture.valid)(
    'accepts upstream require-localize-metadata valid case: $name',
    ({ code, options }) => {
      expect(
        scanAngularEslint(code, 'fixture.ts', {
          ruleNames: ['require-localize-metadata'],
          options,
        }),
      ).toEqual([]);
    },
  );

  it.each(requireLocalizeMetadataFixture.invalid)(
    'matches upstream require-localize-metadata diagnostic: $name',
    ({ code, errors, options }) => {
      expect(
        scanAngularEslint(code, 'fixture.ts', {
          ruleNames: ['require-localize-metadata'],
          options,
        }),
      ).toEqual(
        errors.map((error) => ({
          ruleName: 'require-localize-metadata',
          messageId: error.messageId,
          data: Object.entries(error.data).map(([key, value]) => ({ key, value })),
          loc: {
            startLine: error.line,
            startColumn: error.column - 1,
            endLine: error.endLine,
            endColumn: error.endColumn - 1,
          },
        })),
      );
    },
  );

  it('keeps every requirement independently disabled by default', () => {
    const source = '$localize`Hello`;';
    expect(
      scanAngularEslint(source, 'fixture.ts', {
        ruleNames: ['require-localize-metadata'],
        options: [],
      }),
    ).toEqual([]);
    expect(
      [{ requireDescription: true }, { requireMeaning: true }, { requireCustomId: true }].map(
        (option) =>
          scanAngularEslint(source, 'fixture.ts', {
            ruleNames: ['require-localize-metadata'],
            options: [option],
          })[0].messageId,
      ),
    ).toEqual(['requireLocalizeDescription', 'requireLocalizeMeaning', 'requireLocalizeCustomId']);
  });

  it('preserves report order, pattern data, first-quasi parsing, and identifier tags', () => {
    const source = [
      '$localize`Hello`;',
      '$localize`:meaning|description@@wrong:${value}`;',
      'i18n.$localize`Hello`;',
      '($localize)`:meaning|description@@stable:Hello`;',
    ].join('\n');
    const diagnostics = scanAngularEslint(source, 'fixture.ts', {
      ruleNames: ['require-localize-metadata'],
      options: [
        {
          requireDescription: true,
          requireMeaning: true,
          requireCustomId: '^stable$',
        },
      ],
    });
    expect(
      diagnostics.map(({ messageId, data, loc }) => ({
        messageId,
        data,
        line: loc.startLine,
      })),
    ).toEqual([
      { messageId: 'requireLocalizeDescription', data: [], line: 1 },
      { messageId: 'requireLocalizeMeaning', data: [], line: 1 },
      {
        messageId: 'requireLocalizeCustomId',
        data: [
          {
            key: 'patternMessage',
            value: " matching the pattern /^stable$/ on 'undefined'",
          },
        ],
        line: 1,
      },
      {
        messageId: 'requireLocalizeCustomId',
        data: [
          {
            key: 'patternMessage',
            value: " matching the pattern /^stable$/ on 'wrong'",
          },
        ],
        line: 2,
      },
    ]);
  });

  it('preserves UTF-16 ESTree TemplateElement locations and fails closed on malformed syntax', () => {
    const [diagnostic] = scanAngularEslint(
      "const marker = '😀'; $localize`Hello ${name}`;",
      'fixture.ts',
      {
        ruleNames: ['require-localize-metadata'],
        options: [{ requireMeaning: true }],
      },
    );
    expect(diagnostic).toMatchObject({
      messageId: 'requireLocalizeMeaning',
      loc: {
        startLine: 1,
        startColumn: 30,
        endLine: 1,
        endColumn: 39,
      },
    });
    expect(
      scanAngularEslint('const text = $localize`unterminated', 'fixture.ts', {
        ruleNames: ['require-localize-metadata'],
        options: [{ requireMeaning: true }],
      }),
    ).toEqual([]);
  });
});
