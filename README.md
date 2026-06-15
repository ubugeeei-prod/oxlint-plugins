# oxlint-plugins

Rust-backed Oxlint plugin workspace for porting ESLint plugins through NAPI-RS.

The public package shape is an Oxlint JS plugin. Hot rule logic lives in Rust and is exposed through NAPI-RS so each plugin can be installed independently from npm.

This is unofficial community work. It is not an official Oxlint project, and builtin migration should happen only through normal upstream review.

<!-- BEGIN GENERATED: plugin-coverage -->

## Supported Plugins

<!-- This section is generated from `status.json` by `tools/tasks/generate-readme-coverage.ts`. Do not edit by hand; run `pnpm run docs:readme`. -->

**26** ESLint plugins are being ported · **664 / 1159** rules implemented (**57%**).

| Plugin                                                     | Upstream                                                                                                               | Implemented | Total | Coverage |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ----------- | ----- | -------- |
| [`angular-eslint`](npm/angular-eslint)                     | [`@angular-eslint/eslint-plugin`](https://github.com/angular-eslint/angular-eslint)                                    | 48          | 48    | 100%     |
| [`angular-eslint-template`](npm/angular-eslint-template)   | [`@angular-eslint/eslint-plugin-template`](https://github.com/angular-eslint/angular-eslint)                           | 0           | 39    | 0%       |
| [`cypress`](npm/cypress)                                   | [`eslint-plugin-cypress`](https://github.com/cypress-io/eslint-plugin-cypress)                                         | 13          | 13    | 100%     |
| [`e18e`](npm/e18e)                                         | [`@e18e/eslint-plugin`](https://github.com/e18e/eslint-plugin)                                                         | 25          | 25    | 100%     |
| [`eslint-comments`](npm/eslint-comments)                   | [`@eslint-community/eslint-plugin-eslint-comments`](https://github.com/eslint-community/eslint-plugin-eslint-comments) | 9           | 9     | 100%     |
| [`eslint-json`](npm/eslint-json)                           | [`@eslint/json`](https://github.com/eslint/json)                                                                       | 6           | 6     | 100%     |
| [`eslint-markdown`](npm/eslint-markdown)                   | [`@eslint/markdown`](https://github.com/eslint/markdown)                                                               | 21          | 21    | 100%     |
| [`functional`](npm/functional)                             | [`eslint-plugin-functional`](https://github.com/eslint-functional/eslint-plugin-functional)                            | 20          | 20    | 100%     |
| [`mocha`](npm/mocha)                                       | [`eslint-plugin-mocha`](https://github.com/lo1tuma/eslint-plugin-mocha)                                                | 24          | 24    | 100%     |
| [`no-forbidden-identifiers`](npm/no-forbidden-identifiers) | —                                                                                                                      | 1           | 1     | 100%     |
| [`perfectionist`](npm/perfectionist)                       | [`eslint-plugin-perfectionist`](https://github.com/azat-io/eslint-plugin-perfectionist)                                | 23          | 23    | 100%     |
| [`playwright`](npm/playwright)                             | [`eslint-plugin-playwright`](https://github.com/mskelton/eslint-plugin-playwright)                                     | 58          | 58    | 100%     |
| [`postgresql`](npm/postgresql)                             | [`eslint-plugin-postgresql`](https://github.com/baseballyama/eslint-plugin-postgresql)                                 | 89          | 89    | 100%     |
| [`react`](npm/react)                                       | [`eslint-plugin-react`](https://github.com/jsx-eslint/eslint-plugin-react)                                             | 0           | 103   | 0%       |
| [`react-hooks`](npm/react-hooks)                           | [`eslint-plugin-react-hooks`](https://github.com/facebook/react)                                                       | 1           | 29    | 3%       |
| [`react-refresh`](npm/react-refresh)                       | [`eslint-plugin-react-refresh`](https://github.com/ArnaudBarre/eslint-plugin-react-refresh)                            | 1           | 1     | 100%     |
| [`regexp`](npm/regexp)                                     | [`eslint-plugin-regexp`](https://github.com/ota-meshi/eslint-plugin-regexp)                                            | 82          | 82    | 100%     |
| [`security`](npm/security)                                 | [`eslint-plugin-security`](https://github.com/eslint-community/eslint-plugin-security)                                 | 14          | 14    | 100%     |
| [`simple-import-sort`](npm/simple-import-sort)             | [`eslint-plugin-simple-import-sort`](https://github.com/lydell/eslint-plugin-simple-import-sort)                       | 2           | 2     | 100%     |
| [`sonarjs`](npm/sonarjs)                                   | [`eslint-plugin-sonarjs`](https://github.com/SonarSource/SonarJS)                                                      | 130         | 269   | 48%      |
| [`storybook`](npm/storybook)                               | [`eslint-plugin-storybook`](https://github.com/storybookjs/storybook)                                                  | 16          | 16    | 100%     |
| [`stylistic`](npm/stylistic)                               | [`corsa-oxlint/stylistic`](https://github.com/ubugeeei-prod/corsa-bind)                                                | 46          | 98    | 47%      |
| [`testing-library`](npm/testing-library)                   | [`eslint-plugin-testing-library`](https://github.com/testing-library/eslint-plugin-testing-library)                    | 29          | 29    | 100%     |
| [`typescript-eslint`](npm/typescript-eslint)               | [`@typescript-eslint/eslint-plugin`](https://github.com/typescript-eslint/typescript-eslint)                           | 0           | 134   | 0%       |
| [`unocss`](npm/unocss)                                     | [`@unocss/eslint-plugin`](https://github.com/unocss/unocss)                                                            | 4           | 4     | 100%     |
| [`unused-imports`](npm/unused-imports)                     | [`eslint-plugin-unused-imports`](https://github.com/sweepline/eslint-plugin-unused-imports)                            | 2           | 2     | 100%     |

<details>
<summary><code>angular-eslint</code> — 48/48 implemented</summary>

**Implemented (48):** `component-class-suffix`, `component-max-inline-declarations`, `component-selector`, `computed-must-return`, `consistent-component-styles`, `contextual-decorator`, `contextual-lifecycle`, `directive-class-suffix`, `directive-selector`, `no-async-lifecycle-method`, `no-attribute-decorator`, `no-developer-preview`, `no-duplicates-in-metadata-arrays`, `no-empty-lifecycle-method`, `no-experimental`, `no-forward-ref`, `no-implicit-take-until-destroyed`, `no-input-prefix`, `no-input-rename`, `no-inputs-metadata-property`, `no-lifecycle-call`, `no-output-native`, `no-output-on-prefix`, `no-output-rename`, `no-outputs-metadata-property`, `no-pipe-impure`, `no-queries-metadata-property`, `no-uncalled-signals`, `pipe-prefix`, `prefer-host-metadata-property`, `prefer-inject`, `prefer-on-push-component-change-detection`, `prefer-output-emitter-ref`, `prefer-output-readonly`, `prefer-signal-model`, `prefer-signals`, `prefer-standalone`, `relative-url-prefix`, `require-lifecycle-on-prototype`, `require-localize-metadata`, `runtime-localize`, `sort-keys-in-type-decorator`, `sort-lifecycle-methods`, `use-component-selector`, `use-component-view-encapsulation`, `use-injectable-provided-in`, `use-lifecycle-interface`, `use-pipe-transform-interface`

</details>
<details>
<summary><code>angular-eslint-template</code> — 0/39 implemented</summary>

**Not implemented (39):** `alt-text`, `attributes-order`, `banana-in-box`, `button-has-type`, `click-events-have-key-events`, `conditional-complexity`, `cyclomatic-complexity`, `elements-content`, `eqeqeq`, `i18n`, `interactive-supports-focus`, `label-has-associated-control`, `mouse-events-have-key-events`, `no-any`, `no-autofocus`, `no-call-expression`, `no-distracting-elements`, `no-duplicate-attributes`, `no-empty-control-flow`, `no-inline-styles`, `no-interpolation-in-attributes`, `no-negated-async`, `no-nested-tags`, `no-non-null-assertion`, `no-positive-tabindex`, `prefer-at-else`, `prefer-at-empty`, `prefer-built-in-pipes`, `prefer-class-binding`, `prefer-contextual-for-variables`, `prefer-control-flow`, `prefer-ngsrc`, `prefer-self-closing-tags`, `prefer-static-string-properties`, `prefer-template-literal`, `role-has-required-aria`, `table-scope`, `use-track-by-function`, `valid-aria`

</details>
<details>
<summary><code>cypress</code> — 13/13 implemented</summary>

**Implemented (13):** `assertion-before-screenshot`, `no-and`, `no-assigning-return-values`, `no-async-before`, `no-async-tests`, `no-chained-get`, `no-debug`, `no-force`, `no-pause`, `no-unnecessary-waiting`, `no-xpath`, `require-data-selectors`, `unsafe-to-chain-command`

</details>
<details>
<summary><code>e18e</code> — 25/25 implemented</summary>

**Implemented (25):** `ban-dependencies`, `no-delete-property`, `no-indexof-equality`, `no-spread-in-reduce`, `prefer-array-at`, `prefer-array-fill`, `prefer-array-from-map`, `prefer-array-some`, `prefer-array-to-reversed`, `prefer-array-to-sorted`, `prefer-array-to-spliced`, `prefer-date-now`, `prefer-exponentiation-operator`, `prefer-includes`, `prefer-includes-over-regex-test`, `prefer-inline-equality`, `prefer-nullish-coalescing`, `prefer-object-has-own`, `prefer-regex-test`, `prefer-spread-syntax`, `prefer-static-collator`, `prefer-static-regex`, `prefer-string-fromcharcode`, `prefer-timer-args`, `prefer-url-canparse`

</details>
<details>
<summary><code>eslint-comments</code> — 9/9 implemented</summary>

**Implemented (9):** `disable-enable-pair`, `no-aggregating-enable`, `no-duplicate-disable`, `no-restricted-disable`, `no-unlimited-disable`, `no-unused-disable`, `no-unused-enable`, `no-use`, `require-description`

</details>
<details>
<summary><code>eslint-json</code> — 6/6 implemented</summary>

**Implemented (6):** `no-duplicate-keys`, `no-empty-keys`, `no-unnormalized-keys`, `no-unsafe-values`, `sort-keys`, `top-level-interop`

</details>
<details>
<summary><code>eslint-markdown</code> — 21/21 implemented</summary>

**Implemented (21):** `fenced-code-language`, `fenced-code-meta`, `heading-increment`, `no-bare-urls`, `no-duplicate-definitions`, `no-duplicate-headings`, `no-empty-definitions`, `no-empty-images`, `no-empty-links`, `no-html`, `no-invalid-label-refs`, `no-missing-atx-heading-space`, `no-missing-label-refs`, `no-missing-link-fragments`, `no-multiple-h1`, `no-reference-like-urls`, `no-reversed-media-syntax`, `no-space-in-emphasis`, `no-unused-definitions`, `require-alt-text`, `table-column-count`

</details>
<details>
<summary><code>functional</code> — 20/20 implemented</summary>

**Implemented (20):** `functional-parameters`, `immutable-data`, `no-class-inheritance`, `no-classes`, `no-conditional-statements`, `no-expression-statements`, `no-let`, `no-loop-statements`, `no-mixed-types`, `no-promise-reject`, `no-return-void`, `no-this-expressions`, `no-throw-statements`, `no-try-statements`, `prefer-immutable-types`, `prefer-property-signatures`, `prefer-readonly-type`, `prefer-tacit`, `readonly-type`, `type-declaration-immutability`

</details>
<details>
<summary><code>mocha</code> — 24/24 implemented</summary>

**Implemented (24):** `consistent-interface`, `consistent-spacing-between-blocks`, `handle-done-callback`, `max-top-level-suites`, `no-async-suite`, `no-empty-title`, `no-exclusive-tests`, `no-exports`, `no-global-tests`, `no-hooks`, `no-hooks-for-single-case`, `no-identical-title`, `no-mocha-arrows`, `no-nested-tests`, `no-pending-tests`, `no-return-and-callback`, `no-return-from-async`, `no-setup-in-describe`, `no-sibling-hooks`, `no-synchronous-tests`, `no-top-level-hooks`, `prefer-arrow-callback`, `valid-suite-title`, `valid-test-title`

</details>
<details>
<summary><code>no-forbidden-identifiers</code> — 1/1 implemented</summary>

**Implemented (1):** `no-forbidden-identifiers`

</details>
<details>
<summary><code>perfectionist</code> — 23/23 implemented</summary>

**Implemented (23):** `sort-array-includes`, `sort-arrays`, `sort-classes`, `sort-decorators`, `sort-enums`, `sort-export-attributes`, `sort-exports`, `sort-heritage-clauses`, `sort-import-attributes`, `sort-imports`, `sort-interfaces`, `sort-intersection-types`, `sort-jsx-props`, `sort-maps`, `sort-modules`, `sort-named-exports`, `sort-named-imports`, `sort-object-types`, `sort-objects`, `sort-sets`, `sort-switch-case`, `sort-union-types`, `sort-variable-declarations`

</details>
<details>
<summary><code>playwright</code> — 58/58 implemented</summary>

**Implemented (58):** `consistent-spacing-between-blocks`, `expect-expect`, `max-expects`, `max-nested-describe`, `missing-playwright-await`, `no-commented-out-tests`, `no-conditional-expect`, `no-conditional-in-test`, `no-duplicate-hooks`, `no-duplicate-slow`, `no-element-handle`, `no-eval`, `no-focused-test`, `no-force-option`, `no-get-by-title`, `no-hooks`, `no-nested-step`, `no-networkidle`, `no-nth-methods`, `no-page-pause`, `no-raw-locators`, `no-restricted-locators`, `no-restricted-matchers`, `no-restricted-roles`, `no-skipped-test`, `no-slowed-test`, `no-standalone-expect`, `no-unsafe-references`, `no-unused-locators`, `no-useless-await`, `no-useless-not`, `no-wait-for-navigation`, `no-wait-for-selector`, `no-wait-for-timeout`, `prefer-comparison-matcher`, `prefer-equality-matcher`, `prefer-hooks-in-order`, `prefer-hooks-on-top`, `prefer-locator`, `prefer-lowercase-title`, `prefer-native-locators`, `prefer-strict-equal`, `prefer-to-be`, `prefer-to-contain`, `prefer-to-have-count`, `prefer-to-have-length`, `prefer-web-first-assertions`, `require-hook`, `require-soft-assertions`, `require-tags`, `require-to-pass-timeout`, `require-to-throw-message`, `require-top-level-describe`, `valid-describe-callback`, `valid-expect`, `valid-expect-in-promise`, `valid-test-tags`, `valid-title`

</details>
<details>
<summary><code>postgresql</code> — 89/89 implemented</summary>

**Implemented (89):** `align-column-definitions`, `align-values`, `consistent-as-for-column-alias`, `consistent-as-for-table-alias`, `consistent-between-over-and`, `consistent-create-index-concurrently`, `consistent-create-or-replace`, `consistent-drop-index-concurrently`, `consistent-explicit-inner-join`, `consistent-explicit-outer-join`, `consistent-fk-not-valid`, `consistent-identity-over-serial`, `consistent-jsonb-over-json`, `consistent-reindex-concurrently`, `consistent-text-over-varchar`, `consistent-timestamptz`, `no-add-check-constraint-without-not-valid`, `no-add-column-not-null-without-default`, `no-add-unique-constraint-directly`, `no-alter-column-type`, `no-char-type`, `no-cluster`, `no-composite-primary-key`, `no-create-role`, `no-cross-join`, `no-distinct-on-without-order-by`, `no-drop-column`, `no-drop-database`, `no-drop-not-null`, `no-drop-schema-cascade`, `no-drop-table-cascade`, `no-equality-with-null`, `no-grant-all`, `no-grant-to-public`, `no-group-by-ordinal`, `no-having-without-group-by`, `no-identifier-too-long`, `no-implicit-join`, `no-leading-wildcard-like`, `no-money-type`, `no-natural-join`, `no-not-in-subquery`, `no-numeric-without-precision`, `no-on-delete-cascade`, `no-order-by-ordinal`, `no-rename-column`, `no-rename-table`, `no-rule`, `no-security-definer-without-search-path`, `no-select-into`, `no-select-star`, `no-set-not-null`, `no-set-search-path`, `no-syntax-error`, `no-temporary-table`, `no-time-type`, `no-truncate-cascade`, `no-unlogged-table`, `no-unnecessary-quoted-identifier`, `no-update-primary-key`, `no-update-without-from-binding`, `no-vacuum-full`, `no-volatile-default-on-add-column`, `no-with-recursive-without-limit`, `plpgsql-keyword-case`, `prefer-add-constraint-not-valid`, `prefer-bigint-id`, `prefer-cast-operator`, `prefer-coalesce-over-case`, `prefer-current-timestamp-over-now`, `prefer-exists-over-in-subquery`, `prefer-explicit-null-ordering`, `prefer-in-list-over-or`, `prefer-keyword-case`, `prefer-not-equals-operator`, `require-fk-include-columns`, `require-if-exists`, `require-index-on-fk-column`, `require-limit`, `require-named-constraint`, `require-on-delete-action`, `require-primary-key`, `require-schema-qualified-table`, `require-table-columns`, `require-trailing-semicolon`, `require-where-in-delete`, `require-where-in-update`, `snake-case-column-name`, `snake-case-table-name`

</details>
<details>
<summary><code>react</code> — 0/103 implemented</summary>

**Not implemented (103):** `boolean-prop-naming`, `button-has-type`, `checked-requires-onchange-or-readonly`, `default-props-match-prop-types`, `destructuring-assignment`, `display-name`, `forbid-component-props`, `forbid-dom-props`, `forbid-elements`, `forbid-foreign-prop-types`, `forbid-prop-types`, `forward-ref-uses-ref`, `function-component-definition`, `hook-use-state`, `iframe-missing-sandbox`, `jsx-boolean-value`, `jsx-child-element-spacing`, `jsx-closing-bracket-location`, `jsx-closing-tag-location`, `jsx-curly-brace-presence`, `jsx-curly-newline`, `jsx-curly-spacing`, `jsx-equals-spacing`, `jsx-filename-extension`, `jsx-first-prop-new-line`, `jsx-fragments`, `jsx-handler-names`, `jsx-indent`, `jsx-indent-props`, `jsx-key`, `jsx-max-depth`, `jsx-max-props-per-line`, `jsx-newline`, `jsx-no-bind`, `jsx-no-comment-textnodes`, `jsx-no-constructed-context-values`, `jsx-no-duplicate-props`, `jsx-no-leaked-render`, `jsx-no-literals`, `jsx-no-script-url`, `jsx-no-target-blank`, `jsx-no-undef`, `jsx-no-useless-fragment`, `jsx-one-expression-per-line`, `jsx-pascal-case`, `jsx-props-no-multi-spaces`, `jsx-props-no-spread-multi`, `jsx-props-no-spreading`, `jsx-sort-default-props`, `jsx-sort-props`, `jsx-space-before-closing`, `jsx-tag-spacing`, `jsx-uses-react`, `jsx-uses-vars`, `jsx-wrap-multilines`, `no-access-state-in-setstate`, `no-adjacent-inline-elements`, `no-array-index-key`, `no-arrow-function-lifecycle`, `no-children-prop`, `no-danger`, `no-danger-with-children`, `no-deprecated`, `no-did-mount-set-state`, `no-did-update-set-state`, `no-direct-mutation-state`, `no-find-dom-node`, `no-invalid-html-attribute`, `no-is-mounted`, `no-multi-comp`, `no-namespace`, `no-object-type-as-default-prop`, `no-redundant-should-component-update`, `no-render-return-value`, `no-set-state`, `no-string-refs`, `no-this-in-sfc`, `no-typos`, `no-unescaped-entities`, `no-unknown-property`, `no-unsafe`, `no-unstable-nested-components`, `no-unused-class-component-methods`, `no-unused-prop-types`, `no-unused-state`, `no-will-update-set-state`, `prefer-es6-class`, `prefer-exact-props`, `prefer-read-only-props`, `prefer-stateless-function`, `prop-types`, `react-in-jsx-scope`, `require-default-props`, `require-optimization`, `require-render-return`, `self-closing-comp`, `sort-comp`, `sort-default-props`, `sort-prop-types`, `state-in-constructor`, `static-property-placement`, `style-prop-object`, `void-dom-elements-no-children`

</details>
<details>
<summary><code>react-hooks</code> — 1/29 implemented</summary>

**Implemented (1):** `rules-of-hooks`

**Not implemented (28):** `capitalized-calls`, `component-hook-factories`, `config`, `error-boundaries`, `exhaustive-deps`, `exhaustive-effect-dependencies`, `fbt`, `gating`, `globals`, `hooks`, `immutability`, `incompatible-library`, `invariant`, `memo-dependencies`, `memoized-effect-dependencies`, `no-deriving-state-in-effects`, `preserve-manual-memoization`, `purity`, `refs`, `rule-suppression`, `set-state-in-effect`, `set-state-in-render`, `static-components`, `syntax`, `todo`, `unsupported-syntax`, `use-memo`, `void-use-memo`

</details>
<details>
<summary><code>react-refresh</code> — 1/1 implemented</summary>

**Implemented (1):** `only-export-components`

</details>
<details>
<summary><code>regexp</code> — 82/82 implemented</summary>

**Implemented (82):** `confusing-quantifier`, `control-character-escape`, `grapheme-string-literal`, `hexadecimal-escape`, `letter-case`, `match-any`, `negation`, `no-contradiction-with-assertion`, `no-control-character`, `no-dupe-characters-character-class`, `no-dupe-disjunctions`, `no-empty-alternative`, `no-empty-capturing-group`, `no-empty-character-class`, `no-empty-group`, `no-empty-lookarounds-assertion`, `no-empty-string-literal`, `no-escape-backspace`, `no-extra-lookaround-assertions`, `no-invalid-regexp`, `no-invisible-character`, `no-lazy-ends`, `no-legacy-features`, `no-misleading-capturing-group`, `no-misleading-unicode-character`, `no-missing-g-flag`, `no-non-standard-flag`, `no-obscure-range`, `no-octal`, `no-optional-assertion`, `no-potentially-useless-backreference`, `no-standalone-backslash`, `no-super-linear-backtracking`, `no-super-linear-move`, `no-trivially-nested-assertion`, `no-trivially-nested-quantifier`, `no-unused-capturing-group`, `no-useless-assertions`, `no-useless-backreference`, `no-useless-character-class`, `no-useless-dollar-replacements`, `no-useless-escape`, `no-useless-flag`, `no-useless-lazy`, `no-useless-non-capturing-group`, `no-useless-quantifier`, `no-useless-range`, `no-useless-set-operand`, `no-useless-string-literal`, `no-useless-two-nums-quantifier`, `no-zero-quantifier`, `optimal-lookaround-quantifier`, `optimal-quantifier-concatenation`, `prefer-character-class`, `prefer-d`, `prefer-escape-replacement-dollar-char`, `prefer-lookaround`, `prefer-named-backreference`, `prefer-named-capture-group`, `prefer-named-replacement`, `prefer-plus-quantifier`, `prefer-predefined-assertion`, `prefer-quantifier`, `prefer-question-quantifier`, `prefer-range`, `prefer-regexp-exec`, `prefer-regexp-test`, `prefer-result-array-groups`, `prefer-set-operation`, `prefer-star-quantifier`, `prefer-unicode-codepoint-escapes`, `prefer-w`, `require-unicode-regexp`, `require-unicode-sets-regexp`, `simplify-set-operations`, `sort-alternatives`, `sort-character-class-elements`, `sort-flags`, `strict`, `unicode-escape`, `unicode-property`, `use-ignore-case`

</details>
<details>
<summary><code>security</code> — 14/14 implemented</summary>

**Implemented (14):** `detect-bidi-characters`, `detect-buffer-noassert`, `detect-child-process`, `detect-disable-mustache-escape`, `detect-eval-with-expression`, `detect-new-buffer`, `detect-no-csrf-before-method-override`, `detect-non-literal-fs-filename`, `detect-non-literal-regexp`, `detect-non-literal-require`, `detect-object-injection`, `detect-possible-timing-attacks`, `detect-pseudoRandomBytes`, `detect-unsafe-regex`

</details>
<details>
<summary><code>simple-import-sort</code> — 2/2 implemented</summary>

**Implemented (2):** `exports`, `imports`

</details>
<details>
<summary><code>sonarjs</code> — 130/269 implemented</summary>

**Implemented (130):** `anchor-precedence`, `arguments-order`, `arguments-usage`, `array-callback-without-return`, `array-constructor`, `bitwise-operators`, `block-scoped-var`, `call-argument-line`, `class-name`, `class-prototype`, `code-eval`, `comma-or-logical-or-case`, `constructor-for-side-effects`, `cyclomatic-complexity`, `duplicates-in-character-class`, `elseif-without-else`, `empty-string-repetition`, `file-name-differ-from-class`, `fixme-tag`, `for-in`, `for-loop-increment-sign`, `function-inside-loop`, `generator-without-yield`, `hashing`, `inconsistent-function-call`, `index-of-compare-to-positive-number`, `inverted-assertion-arguments`, `link-with-target-blank`, `max-lines`, `max-lines-per-function`, `max-switch-cases`, `max-union-size`, `misplaced-loop-counter`, `nested-control-flow`, `new-operator-misuse`, `no-all-duplicated-branches`, `no-alphabetical-sort`, `no-array-delete`, `no-associative-arrays`, `no-built-in-override`, `no-case-label-in-switch`, `no-clear-text-protocols`, `no-code-after-done`, `no-collapsible-if`, `no-collection-size-mischeck`, `no-control-regex`, `no-delete-var`, `no-duplicate-in-composite`, `no-duplicate-string`, `no-duplicated-branches`, `no-empty-after-reluctant`, `no-empty-alternatives`, `no-empty-character-class`, `no-empty-group`, `no-empty-test-file`, `no-equals-in-for-termination`, `no-exclusive-tests`, `no-extra-arguments`, `no-for-in-iterable`, `no-function-declaration-in-block`, `no-global-this`, `no-hardcoded-ip`, `no-hardcoded-passwords`, `no-identical-conditions`, `no-identical-expressions`, `no-identical-functions`, `no-ignored-exceptions`, `no-ignored-return`, `no-in-misuse`, `no-inconsistent-returns`, `no-invalid-regexp`, `no-invariant-returns`, `no-inverted-boolean-check`, `no-labels`, `no-literal-call`, `no-misleading-array-reverse`, `no-nested-assignment`, `no-nested-conditional`, `no-nested-functions`, `no-nested-incdec`, `no-nested-switch`, `no-nested-template-literals`, `no-parameter-reassignment`, `no-primitive-wrappers`, `no-redundant-boolean`, `no-redundant-jump`, `no-redundant-optional`, `no-regex-spaces`, `no-require-or-define`, `no-same-argument-assert`, `no-same-line-conditional`, `no-skipped-tests`, `no-small-switch`, `no-sonar-comments`, `no-tab`, `no-undefined-argument`, `no-undefined-assignment`, `no-unenclosed-multiline-block`, `no-unthrown-error`, `no-unused-function-argument`, `no-use-of-empty-return-value`, `no-useless-catch`, `no-useless-increment`, `no-useless-intersection`, `no-variable-usage-before-declaration`, `no-weak-cipher`, `no-wildcard-import`, `non-existent-operator`, `object-alt-content`, `prefer-default-last`, `prefer-immediate-return`, `prefer-object-literal`, `prefer-promise-shorthand`, `prefer-single-boolean-return`, `prefer-while`, `process-argv`, `pseudo-random`, `public-static-readonly`, `reduce-initial-value`, `shorthand-property-grouping`, `single-char-in-character-classes`, `single-character-alternation`, `standard-input`, `todo-tag`, `too-many-break-or-continue-in-loop`, `unicode-aware-regex`, `updated-const-var`, `updated-loop-counter`, `use-type-alias`, `void-use`

**Not implemented (139):** `argument-type`, `arrow-function-convention`, `assertions-in-tests`, `aws-apigateway-public-api`, `aws-ec2-rds-dms-public`, `aws-ec2-unencrypted-ebs-volume`, `aws-efs-unencrypted`, `aws-iam-all-privileges`, `aws-iam-all-resources-accessible`, `aws-iam-privilege-escalation`, `aws-iam-public-access`, `aws-opensearchservice-domain`, `aws-rds-unencrypted-databases`, `aws-restricted-ip-admin-access`, `aws-s3-bucket-granted-access`, `aws-s3-bucket-insecure-http`, `aws-s3-bucket-public-access`, `aws-s3-bucket-server-encryption`, `aws-s3-bucket-versioning`, `aws-sagemaker-unencrypted-notebook`, `aws-sns-unencrypted-topics`, `aws-sqs-unencrypted-queue`, `bool-param-default`, `certificate-transparency`, `chai-determinate-assertion`, `cognitive-complexity`, `comment-regex`, `concise-regex`, `conditional-indentation`, `confidential-information-logging`, `content-length`, `content-security-policy`, `cookie-no-httponly`, `cookies`, `cors`, `csrf`, `declarations-in-global-scope`, `deprecation`, `destructuring-assignment-syntax`, `different-types-comparison`, `disabled-auto-escaping`, `disabled-resource-integrity`, `disabled-timeout`, `dns-prefetching`, `dompurify-unsafe-config`, `dynamically-constructed-templates`, `encryption`, `encryption-secure-mode`, `existing-groups`, `expression-complexity`, `file-header`, `file-permissions`, `file-uploads`, `frame-ancestors`, `function-name`, `function-return-type`, `future-reserved-words`, `hardcoded-secret-signatures`, `hidden-files`, `in-operator-type-error`, `insecure-cookie`, `insecure-jwt-token`, `jsx-no-leaked-render`, `label-position`, `no-angular-bypass-sanitization`, `no-async-constructor`, `no-commented-code`, `no-dead-store`, `no-element-overwrite`, `no-empty-collection`, `no-fallthrough`, `no-globals-shadowing`, `no-gratuitous-expressions`, `no-hardcoded-secrets`, `no-hook-setter-in-body`, `no-implicit-dependencies`, `no-implicit-global`, `no-incomplete-assertions`, `no-incorrect-string-concat`, `no-internal-api-use`, `no-intrusive-permissions`, `no-ip-forward`, `no-mime-sniff`, `no-misleading-character-class`, `no-mixed-content`, `no-os-command-from-path`, `no-redundant-assignments`, `no-redundant-parentheses`, `no-reference-error`, `no-referrer-policy`, `no-return-type-any`, `no-selector-parameter`, `no-session-cookies-on-static-assets`, `no-table-as-layout`, `no-try-promise`, `no-uniq-key`, `no-unsafe-unzip`, `no-unused-collection`, `no-unused-vars`, `no-useless-react-setstate`, `no-vue-bypass-sanitization`, `no-weak-keys`, `non-number-in-arithmetic-expression`, `null-dereference`, `operation-returning-nan`, `os-command`, `post-message`, `prefer-read-only-props`, `prefer-regexp-exec`, `prefer-type-guard`, `production-debug`, `publicly-writable-directories`, `redundant-type-aliases`, `regex-complexity`, `regular-expr`, `review-blockchain-mnemonic`, `session-regeneration`, `slow-regex`, `sockets`, `sql-queries`, `stable-tests`, `stateful-regex`, `strict-transport-security`, `strings-comparison`, `table-header`, `table-header-reference`, `test-check-exception`, `unused-import`, `unused-named-groups`, `unverified-certificate`, `unverified-hostname`, `useless-string-operation`, `values-not-convertible-to-numbers`, `variable-name`, `weak-ssl`, `web-sql-database`, `x-powered-by`, `xml-parser-xxe`, `xpath`

</details>
<details>
<summary><code>storybook</code> — 16/16 implemented</summary>

**Implemented (16):** `await-interactions`, `context-in-play-function`, `csf-component`, `default-exports`, `hierarchy-separator`, `meta-inline-properties`, `meta-satisfies-type`, `no-redundant-story-name`, `no-renderer-packages`, `no-stories-of`, `no-title-property-in-meta`, `no-uninstalled-addons`, `prefer-pascal-case`, `story-exports`, `use-storybook-expect`, `use-storybook-testing-library`

</details>
<details>
<summary><code>stylistic</code> — 46/98 implemented</summary>

**Implemented (46):** `array-bracket-spacing`, `arrow-parens`, `arrow-spacing`, `block-spacing`, `comma-dangle`, `comma-spacing`, `comma-style`, `computed-property-spacing`, `dot-location`, `eol-last`, `function-call-spacing`, `generator-star-spacing`, `implicit-arrow-linebreak`, `key-spacing`, `keyword-spacing`, `linebreak-style`, `max-len`, `max-statements-per-line`, `new-parens`, `no-extra-semi`, `no-floating-decimal`, `no-multi-spaces`, `no-multiple-empty-lines`, `no-tabs`, `no-trailing-spaces`, `no-whitespace-before-property`, `object-curly-spacing`, `operator-linebreak`, `padded-blocks`, `quote-props`, `quotes`, `rest-spread-spacing`, `semi-spacing`, `semi-style`, `space-before-blocks`, `space-before-function-paren`, `space-in-parens`, `space-infix-ops`, `space-unary-ops`, `spaced-comment`, `switch-colon-spacing`, `template-curly-spacing`, `template-tag-spacing`, `unicode-bom`, `wrap-regex`, `yield-star-spacing`

**Not implemented (52):** `array-bracket-newline`, `array-element-newline`, `brace-style`, `curly-newline`, `exp-jsx-props-style`, `exp-list-style`, `function-call-argument-newline`, `function-paren-newline`, `indent`, `indent-binary-ops`, `jsx-child-element-spacing`, `jsx-closing-bracket-location`, `jsx-closing-tag-location`, `jsx-curly-brace-presence`, `jsx-curly-newline`, `jsx-curly-spacing`, `jsx-equals-spacing`, `jsx-first-prop-new-line`, `jsx-function-call-newline`, `jsx-indent`, `jsx-indent-props`, `jsx-max-props-per-line`, `jsx-newline`, `jsx-one-expression-per-line`, `jsx-pascal-case`, `jsx-props-no-multi-spaces`, `jsx-quotes`, `jsx-self-closing-comp`, `jsx-sort-props`, `jsx-tag-spacing`, `jsx-wrap-multilines`, `line-comment-position`, `lines-around-comment`, `lines-between-class-members`, `member-delimiter-style`, `multiline-comment-style`, `multiline-ternary`, `newline-per-chained-call`, `no-confusing-arrow`, `no-extra-parens`, `no-mixed-operators`, `no-mixed-spaces-and-tabs`, `nonblock-statement-body-position`, `object-curly-newline`, `object-property-newline`, `one-var-declaration-per-line`, `padding-line-between-statements`, `semi`, `type-annotation-spacing`, `type-generic-spacing`, `type-named-tuple-spacing`, `wrap-iife`

</details>
<details>
<summary><code>testing-library</code> — 29/29 implemented</summary>

**Implemented (29):** `await-async-events`, `await-async-queries`, `await-async-utils`, `consistent-data-testid`, `no-await-sync-events`, `no-await-sync-queries`, `no-container`, `no-debugging-utils`, `no-dom-import`, `no-global-regexp-flag-in-query`, `no-manual-cleanup`, `no-node-access`, `no-promise-in-fire-event`, `no-render-in-lifecycle`, `no-test-id-queries`, `no-unnecessary-act`, `no-wait-for-multiple-assertions`, `no-wait-for-side-effects`, `no-wait-for-snapshot`, `prefer-explicit-assert`, `prefer-find-by`, `prefer-implicit-assert`, `prefer-presence-queries`, `prefer-query-by-disappearance`, `prefer-query-matchers`, `prefer-screen-queries`, `prefer-user-event`, `prefer-user-event-setup`, `render-result-naming-convention`

</details>
<details>
<summary><code>typescript-eslint</code> — 0/134 implemented</summary>

**Not implemented (134):** `adjacent-overload-signatures`, `array-type`, `await-thenable`, `ban-ts-comment`, `ban-tslint-comment`, `class-literal-property-style`, `class-methods-use-this`, `consistent-generic-constructors`, `consistent-indexed-object-style`, `consistent-return`, `consistent-type-assertions`, `consistent-type-definitions`, `consistent-type-exports`, `consistent-type-imports`, `default-param-last`, `dot-notation`, `explicit-function-return-type`, `explicit-member-accessibility`, `explicit-module-boundary-types`, `init-declarations`, `max-params`, `member-ordering`, `method-signature-style`, `naming-convention`, `no-array-constructor`, `no-array-delete`, `no-base-to-string`, `no-confusing-non-null-assertion`, `no-confusing-void-expression`, `no-deprecated`, `no-dupe-class-members`, `no-duplicate-enum-values`, `no-duplicate-type-constituents`, `no-dynamic-delete`, `no-empty-function`, `no-empty-interface`, `no-empty-object-type`, `no-explicit-any`, `no-extra-non-null-assertion`, `no-extraneous-class`, `no-floating-promises`, `no-for-in-array`, `no-implied-eval`, `no-import-type-side-effects`, `no-inferrable-types`, `no-invalid-this`, `no-invalid-void-type`, `no-loop-func`, `no-loss-of-precision`, `no-magic-numbers`, `no-meaningless-void-operator`, `no-misused-new`, `no-misused-promises`, `no-misused-spread`, `no-mixed-enums`, `no-namespace`, `no-non-null-asserted-nullish-coalescing`, `no-non-null-asserted-optional-chain`, `no-non-null-assertion`, `no-redeclare`, `no-redundant-type-constituents`, `no-require-imports`, `no-restricted-imports`, `no-restricted-types`, `no-shadow`, `no-this-alias`, `no-type-alias`, `no-unnecessary-boolean-literal-compare`, `no-unnecessary-condition`, `no-unnecessary-parameter-property-assignment`, `no-unnecessary-qualifier`, `no-unnecessary-template-expression`, `no-unnecessary-type-arguments`, `no-unnecessary-type-assertion`, `no-unnecessary-type-constraint`, `no-unnecessary-type-conversion`, `no-unnecessary-type-parameters`, `no-unsafe-argument`, `no-unsafe-assignment`, `no-unsafe-call`, `no-unsafe-declaration-merging`, `no-unsafe-enum-comparison`, `no-unsafe-function-type`, `no-unsafe-member-access`, `no-unsafe-return`, `no-unsafe-type-assertion`, `no-unsafe-unary-minus`, `no-unused-expressions`, `no-unused-private-class-members`, `no-unused-vars`, `no-use-before-define`, `no-useless-constructor`, `no-useless-default-assignment`, `no-useless-empty-export`, `no-var-requires`, `no-wrapper-object-types`, `non-nullable-type-assertion-style`, `only-throw-error`, `parameter-properties`, `prefer-as-const`, `prefer-destructuring`, `prefer-enum-initializers`, `prefer-find`, `prefer-for-of`, `prefer-function-type`, `prefer-includes`, `prefer-literal-enum-member`, `prefer-namespace-keyword`, `prefer-nullish-coalescing`, `prefer-optional-chain`, `prefer-promise-reject-errors`, `prefer-readonly`, `prefer-readonly-parameter-types`, `prefer-reduce-type-parameter`, `prefer-regexp-exec`, `prefer-return-this-type`, `prefer-string-starts-ends-with`, `prefer-ts-expect-error`, `promise-function-async`, `related-getter-setter-pairs`, `require-array-sort-compare`, `require-await`, `restrict-plus-operands`, `restrict-template-expressions`, `return-await`, `sort-type-constituents`, `strict-boolean-expressions`, `strict-void-return`, `switch-exhaustiveness-check`, `triple-slash-reference`, `typedef`, `unbound-method`, `unified-signatures`, `use-unknown-in-catch-callback-variable`

</details>
<details>
<summary><code>unocss</code> — 4/4 implemented</summary>

**Implemented (4):** `blocklist`, `enforce-class-compile`, `order`, `order-attributify`

</details>
<details>
<summary><code>unused-imports</code> — 2/2 implemented</summary>

**Implemented (2):** `no-unused-imports`, `no-unused-vars`

</details>

<!-- END GENERATED: plugin-coverage -->

## Commands

```sh
nix develop
vp install
vp build
vp lint
vp fmt
vp test
vp run bench:check
vp run bench
cargo test --workspace --all-features
vp run new eslint-plugin-react/jsx-no-bind
vp run profile bench
vp run release patch
```

`vp run release major|minor|patch` bumps versions, verifies locally, commits, tags, and pushes. The tag triggers trusted publishing through GitHub Actions.

## Layout

- `crates/_carton`: shared allocation and fast-hash primitives.
- `crates/stylistic`: stylistic-domain Rust rule logic. Add future domains like `import`, `react`, or `security` instead of one crate per rule.
- `npm/*`: individually installable npm packages, including oxlint plugins and shared JS helpers.
- `examples/*`: small usage examples outside the npm workspace graph.
- `docs/site`: ox-content + Void SDK website with rule status pages.
- `docs/guides`: project policy and contributor guides rendered by the website.
- `tools/tasks/*`: Node type-stripped TypeScript task scripts.
- `tools/vite/*`: Vite/Vite+ build helper files.
- `tools/license-exceptions.json`: audited license policy exceptions.
- `tools/port-targets.json`: manifest of the ESLint plugins we intend to port (single source of truth for rule enumeration and release tracking).
- `upstream/*`: upstream port-target sources vendored as shallow git submodules, pinned to each plugin's baseline version. For behavioral reference only; never copy upstream code without honoring its license.
- `docs/port-targets`: generated, per-plugin rule inventories. Run `pnpm run port:rules` to regenerate from the submodules.
- `.github/workflows`: Blacksmith CI and trusted publishing release workflow.

## Port Targets

`tools/port-targets.json` lists the ESLint plugins that Oxlint does not yet support natively (`eslint-plugin-svelte` is excluded; it is handled by [rsvelte](https://github.com/baseballyama/rsvelte), and `eslint-plugin-vue` is excluded; it is handled by [vize](https://vizejs.dev/)). Their sources are vendored under `upstream/` as submodules.

```sh
git submodule update --init --depth 1   # fetch upstream sources
pnpm run port:rules                      # regenerate docs/port-targets/*
pnpm run port:status                     # sync upstream rules into status.json as pending
```

`pnpm run port:rules` enumerates every rule of each target straight from its submodule and fails if a plugin's rule count drifts from the manifest, so the porting backlog stays complete. See `docs/port-targets/README.md` for the generated inventory.

`pnpm run port:status` reads that inventory and ensures every upstream rule of every port target is listed in `status.json`. Rules that have not been ported yet are added with `status: "pending"` and zeroed test flags; existing entries are preserved verbatim. New plugins (e.g. `react`, `typescript-eslint`, `sonarjs`, `postgresql`, `angular-eslint-template`) are bootstrapped with a scaffold `npm/<plugin>/package.json` so the entire 1157-rule backlog is visible and parallelizable across contributors.

## Sample Plugin

`@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers` demonstrates the intended shape:

- JS wrapper uses `@oxlint/plugins` and `createOnce`.
- Rust performs a file-level pre-scan through NAPI-RS.
- Rust tests use `insta` snapshots.
- Vitest covers wrapper reports and skip behavior.

See `docs/guides/porting.md`, `docs/guides/testing-strategy.md`, and `docs/guides/trusted-publishing.md`.

## Credits

Credited to [@ubugeeei](https://github.com/ubugeeei), [@baseballyama](https://github.com/baseballyama), [Blacksmith](https://www.blacksmith.sh/), and [OpenAI](https://openai.com/).

## Motivation And Policy

See `docs/guides/motivation.md`, `docs/guides/governance.md`, and `docs/guides/license-compliance.md` before porting existing ecosystem rules.
Type-aware ports must also follow `docs/guides/type-aware.md`.
Performance policy lives in `docs/guides/performance.md` and is the highest-priority engineering constraint.
Environment setup is Nix-first; see `docs/guides/environment.md`.

## Docs Site

```sh
vp run docs:dev
vp run docs:build
vp run status:sync
```

The status page is generated from `status.json`.
