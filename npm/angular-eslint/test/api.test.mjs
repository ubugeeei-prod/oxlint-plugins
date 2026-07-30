import { describe, expect, it } from 'vitest';

import { implementedAngularEslintRuleNames, scanAngularEslint } from '../api.js';

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

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName).sort()).toEqual(
      expectedRuleNames
        .filter(
          (ruleName) =>
            ![
              'component-selector',
              'directive-selector',
              'no-input-prefix',
              'pipe-prefix',
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
});
