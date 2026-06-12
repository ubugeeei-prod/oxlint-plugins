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
c\` }) class App {}
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
      [...expectedRuleNames].sort(),
    );
  });

  it('returns LSP-shaped locations', () => {
    const [diagnostic] = scanAngularEslint(
      '@Component({ selector: "app-x" }) class App {}\n',
      'fixture.ts',
    );

    expect(diagnostic).toMatchObject({
      ruleName: 'component-class-suffix',
      messageId: 'unexpected',
      loc: {
        startLine: 1,
        endLine: 1,
      },
    });
  });
});
