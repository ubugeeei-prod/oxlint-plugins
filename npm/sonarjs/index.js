'use strict';

// Oxlint plugin port of eslint-plugin-sonarjs (upstream is LGPL-3.0).
// Clean-room implementation: behaviour is reproduced from public RSPEC docs and
// observed output only. The JavaScript layer adapts Oxlint's ESLint-compatible
// plugin API; parsing and rule checks run in Rust through Oxc. Message strings
// live here (independently authored), not in the Rust core.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedSonarjsRuleNames, scanSonarjs } = require('./api.js');

const PLUGIN_NAME = 'sonarjs';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/sonarjs';
const diagnosticsCache = new WeakMap();

const messages = Object.freeze({
  'no-nested-template-literals': {
    nestedTemplateLiteral:
      'Do not nest template literals. Extract the inner template literal into a separate variable.',
  },
  'no-nested-switch': {
    nestedSwitch:
      'Do not nest switch statements. Extract the nested switch into a separate function.',
  },
  'no-nested-conditional': {
    nestedConditional:
      'Do not nest ternary/conditional expressions; extract the nested conditional into an independent statement.',
  },
  'no-collapsible-if': {
    collapsibleIf: "Merge this 'if' statement with the nested one to reduce nesting.",
  },
  'no-redundant-boolean': {
    redundantBoolean: 'Remove this redundant boolean literal.',
  },
  'comma-or-logical-or-case': {
    commaOrLogicalOrInCase:
      "This 'case' label uses '||' or ',', which does not compare against multiple values as it appears to.",
  },
  'no-duplicate-in-composite': {
    duplicateType: 'Remove this duplicated type or replace with another one.',
  },
  'non-existent-operator': {
    nonExistentOperator:
      "Was this '=-', '=+', or '=!' meant to be a compound assignment or comparison? Add a space to clarify, or fix the operator.",
  },
  'no-identical-conditions': {
    identicalConditions:
      "This branch's condition duplicates an earlier one in the same if/else-if chain, so it can never be reached.",
  },
  'no-all-duplicated-branches': {
    allDuplicatedBranches:
      'Remove this conditional structure or edit its code blocks so that they are not all the same.',
  },
  'no-identical-expressions': {
    identicalExpressions:
      'Identical sub-expressions on both sides of this operator make the result constant or redundant.',
  },
  'arguments-usage': {
    argumentsUsage: "Use the rest parameter syntax (...args) instead of the 'arguments' object.",
  },
  'no-labels': {
    noLabels: 'Remove this label and refactor the code to use structured control flow instead.',
  },
  'no-delete-var': {
    noDeleteVar:
      "Do not use 'delete' on a variable; it has no effect. Use 'delete' only to remove object properties.",
  },
  'constructor-for-side-effects': {
    constructorForSideEffects:
      'Either use this object, assign it to a variable, or move the side effects into a named function instead of a constructor.',
  },
  'no-empty-character-class': {
    emptyCharacterClass:
      'This empty character class [] can never match, so this regular expression will never match anything.',
  },
  'generator-without-yield': {
    generatorWithoutYield:
      "This generator contains no 'yield'; either add a 'yield' or convert it to a regular function.",
  },
  'no-exclusive-tests': {
    noExclusiveTests: "Remove '.only' so the whole test suite runs, not just this test.",
  },
  'no-built-in-override': {
    noBuiltInOverride: 'Do not override or shadow a built-in object or function.',
  },
  'class-prototype': {
    classPrototype:
      'Define this on a class using method syntax instead of assigning to the prototype.',
  },
  'max-switch-cases': {
    maxSwitchCases: 'This switch has too many cases; consider a lookup table or polymorphism.',
  },
  'max-union-size': {
    maxUnionSize:
      'This union type has too many members; consider refactoring into a named type or interface.',
  },
  'elseif-without-else': {
    elseifWithoutElse:
      "Add a final 'else' clause to this 'if … else if' chain to handle the remaining cases explicitly.",
  },
  'no-case-label-in-switch': {
    caseLabelInSwitch:
      "Remove this misleading label; it looks like a 'case' clause but is a labeled statement.",
  },
  'for-in': {
    forIn:
      "Wrap this 'for...in' loop body in an 'if' statement to filter out inherited properties.",
  },
  'prefer-while': {
    preferWhile:
      "Replace this 'for' loop with a 'while' loop; it has no initializer or update clause.",
  },
  'no-small-switch': {
    smallSwitch: "This switch has too few cases; use an 'if' statement instead.",
  },
  'prefer-default-last': {
    defaultLast: "Move this 'default' clause to the end of the switch statement.",
  },
  'no-inverted-boolean-check': {
    invertedBooleanCheck:
      'Use the opposite comparison operator instead of negating this comparison.',
  },
  'no-useless-catch': {
    uselessCatch: "Remove this useless 'catch' clause; it only rethrows the caught exception.",
  },
  'no-redundant-optional': {
    redundantOptional:
      "Remove this redundant 'undefined' type; the '?' optional marker already allows it.",
  },
  'prefer-immediate-return': {
    preferImmediateReturn:
      'Return or throw this expression directly instead of assigning it to a temporary variable first.',
  },
  'no-redundant-jump': {
    redundantJump: 'Remove this redundant jump; it does not change the control flow.',
  },
  'no-primitive-wrappers': {
    primitiveWrapper:
      "Use the primitive type instead of the 'new Number/String/Boolean' wrapper object.",
  },
  'no-skipped-tests': {
    skippedTest: 'Re-enable or remove this skipped test.',
  },
  'prefer-single-boolean-return': {
    preferSingleBooleanReturn:
      "Replace this if/else returning booleans with a single 'return' of the condition.",
  },
  'no-unthrown-error': {
    unthrownError: 'Throw this error or remove this useless statement.',
  },
  'no-tab': {
    noTab: 'Replace this tab character with spaces.',
  },
  'fixme-tag': {
    fixmeTag: 'Address this FIXME-tagged comment.',
  },
  'todo-tag': {
    todoTag: 'Complete the task tracked by this TODO-tagged comment.',
  },
  'no-sonar-comments': {
    noSonarComments: 'Remove this NOSONAR comment and fix the underlying issue.',
  },
  'array-constructor': {
    arrayConstructor: 'Use an array literal instead of the Array constructor.',
  },
  'no-function-declaration-in-block': {
    noFunctionDeclarationInBlock:
      'Move this function declaration out of the block, or use a function expression instead.',
  },
  'no-inconsistent-returns': {
    inconsistentReturns:
      'Refactor this function to use "return" consistently, either always with a value or always without.',
  },
  'no-same-line-conditional': {
    sameLineConditional:
      'Move this "if" to a new line or add the missing "else" to clarify the intent.',
  },
  'no-nested-assignment': {
    nestedAssignment: 'Extract this assignment out of the expression into its own statement.',
  },
  'no-nested-incdec': {
    nestedIncDec: 'Extract this increment or decrement operator into a separate statement.',
  },
  'no-useless-increment': {
    uselessIncrement:
      'Remove this useless increment or decrement; the updated value is immediately discarded.',
  },
  'class-name': {
    className: 'Rename this class to start with an uppercase letter (PascalCase).',
  },
  'max-lines': {
    maxLines: 'This file has more lines than the maximum allowed; split it into smaller files.',
  },
  'max-lines-per-function': {
    maxLinesPerFunction:
      'This function has more lines than the maximum allowed; split it into smaller functions.',
  },
  'nested-control-flow': {
    nestedControlFlow: 'Refactor this code to reduce the nesting of control flow statements.',
  },
});

const ruleDescriptions = Object.freeze({
  'no-nested-template-literals': 'Disallow nested template literals',
  'no-nested-switch': 'Disallow nested switch statements',
  'no-nested-conditional': 'Disallow nested conditional (ternary) expressions',
  'no-collapsible-if': 'Disallow collapsible if statements that should be merged',
  'no-redundant-boolean': 'Disallow redundant boolean literals in expressions',
  'comma-or-logical-or-case': "Disallow '||' or ',' expressions as switch case labels",
  'no-duplicate-in-composite':
    'Disallow duplicate type members in TypeScript union or intersection types',
  'non-existent-operator':
    "Disallow the suspicious '=-', '=+', or '=!' operator typos adjacent to a plain assignment",
  'no-identical-conditions':
    'Disallow duplicate conditions in the same if/else-if chain (dead branch)',
  'no-all-duplicated-branches':
    'Disallow conditional structures where every branch has the same implementation',
  'no-identical-expressions':
    'Disallow identical sub-expressions on both sides of binary or logical operators where the result is constant or redundant',
  'arguments-usage': "Disallow use of the 'arguments' object; use rest parameters instead",
  'no-labels': 'Disallow labeled statements; use structured control flow instead',
  'no-delete-var':
    "Disallow 'delete' applied to a plain variable; use it only on object properties",
  'constructor-for-side-effects':
    'Disallow using new solely for side effects without capturing or using the constructed object',
  'no-empty-character-class':
    'Disallow empty character classes in regular expression literals, which can never match',
  'generator-without-yield':
    "Disallow generator functions that contain no 'yield' expression and therefore behave like plain functions",
  'no-exclusive-tests':
    'Disallow .only on test-runner functions (describe, it, test, etc.) that would disable all other tests',
  'no-built-in-override':
    'Disallow overriding or shadowing standard ECMAScript built-in global objects and functions',
  'class-prototype':
    'Disallow assigning methods or properties to a constructor prototype; use class syntax instead',
  'max-switch-cases':
    'Disallow switch statements with more than the configured number of case/default clauses (the "maximum" option; default 30)',
  'max-union-size':
    'Disallow union types with more than the configured number of members (the "threshold" option; default 3; each TSUnionType node is counted per-node)',
  'elseif-without-else':
    "Require a final 'else' clause when an 'if … else if' chain is present, to explicitly handle the remaining case",
  'no-case-label-in-switch':
    "Disallow labeled statements appearing directly in a switch case's consequent list, where they are likely mistaken 'case' clauses",
  'for-in':
    "Require a 'for...in' loop body to be a single 'if' statement that filters inherited properties (structural check only — the 'if' condition is not inspected)",
  'prefer-while':
    "Disallow 'for' loops with no initializer and no update clause; use a 'while' loop instead",
  'no-small-switch':
    "Disallow switch statements with fewer than two real 'case' clauses; use an 'if' statement instead (default clause not counted)",
  'prefer-default-last':
    "Require the 'default' clause of a switch statement to appear as the last clause for readability",
  'no-inverted-boolean-check':
    'Disallow negating a comparison expression; use the opposite comparison operator instead',
  'no-useless-catch':
    "Disallow 'catch' clauses that only rethrow the caught exception; remove them and let the error propagate naturally",
  'no-redundant-optional':
    "Disallow optional property signatures whose type annotation already includes 'undefined'; the '?' marker already permits undefined",
  'prefer-immediate-return':
    'Disallow declaring a local variable solely to immediately return or throw it; return or throw the initializer expression directly',
  'no-redundant-jump':
    'Disallow jump statements (continue without label, return without value) that do not change the control flow because execution would proceed the same way anyway',
  'no-primitive-wrappers':
    "Disallow using 'new' with the primitive wrapper constructors Number, String, or Boolean, which create wrapper objects instead of primitive values",
  'no-skipped-tests':
    'Disallow committed skipped tests (.skip member or x-prefixed Jasmine calls); re-enable or remove them instead',
  'prefer-single-boolean-return':
    'Disallow if/else structures where both branches return a boolean literal; return the condition directly instead',
  'no-unthrown-error':
    "Disallow creating an Error (or Error subtype) with 'new' as a bare statement without throwing it; the value is discarded and this is almost always a bug",
  'no-tab':
    'Disallow tab characters in source files; tabs render inconsistently across editors and tools, so spaces should be used instead',
  'fixme-tag':
    'Disallow FIXME-tagged comments; a FIXME marks code that is known-broken and must be addressed before shipping',
  'todo-tag':
    'Disallow TODO-tagged comments; a TODO marks incomplete work that should be tracked and completed',
  'no-sonar-comments':
    'Disallow NOSONAR comments; they suppress analysis and can hide real issues that should be fixed',
  'array-constructor':
    'Disallow the Array constructor in favor of array literals, except for the single-argument length form',
  'no-function-declaration-in-block':
    'Disallow function declarations nested directly inside a block; use a function expression or move it to the top level',
  'no-inconsistent-returns':
    'Disallow mixing value returns and bare returns in the same function; return a value on all paths or none',
  'no-same-line-conditional':
    'Disallow an "if" statement placed on the same line as the closing brace of a preceding sibling "if"',
  'no-nested-assignment':
    'Disallow assignments inside sub-expressions such as loop and branch conditions or chained assignments',
  'no-nested-incdec':
    'Disallow increment and decrement operators used as a function or constructor call argument',
  'no-useless-increment':
    'Disallow assigning a postfix increment or decrement of a variable back to that same variable',
  'class-name':
    'Require class names to start with an uppercase letter, following the PascalCase convention',
  'max-lines':
    'Disallow files with more code lines than the configured maximum (the "maximum" option; default 1000); blank lines and comment-only lines are not counted',
  'max-lines-per-function':
    'Disallow functions with more code lines than the configured maximum (the "maximum" option; default 200); IIFEs and JSX-containing functions are excluded',
  'nested-control-flow':
    'Disallow control flow statements (if/for/for-in/for-of/while/do-while/switch/try) nested beyond the configured maximumNestingLevel (default 3); else-if chains do not add a level',
});

const ruleTypes = Object.freeze({
  'no-nested-template-literals': 'suggestion',
  'no-nested-switch': 'suggestion',
  'no-nested-conditional': 'suggestion',
  'no-collapsible-if': 'suggestion',
  'no-redundant-boolean': 'suggestion',
  'comma-or-logical-or-case': 'suggestion',
  'no-duplicate-in-composite': 'suggestion',
  'non-existent-operator': 'problem',
  'no-identical-conditions': 'problem',
  'no-all-duplicated-branches': 'problem',
  'no-identical-expressions': 'problem',
  'arguments-usage': 'suggestion',
  'no-labels': 'suggestion',
  'no-delete-var': 'problem',
  'constructor-for-side-effects': 'problem',
  'no-empty-character-class': 'problem',
  'generator-without-yield': 'problem',
  'no-exclusive-tests': 'problem',
  'no-built-in-override': 'problem',
  'class-prototype': 'suggestion',
  'max-switch-cases': 'suggestion',
  'max-union-size': 'suggestion',
  'elseif-without-else': 'suggestion',
  'no-case-label-in-switch': 'problem',
  'for-in': 'suggestion',
  'prefer-while': 'suggestion',
  'no-small-switch': 'suggestion',
  'prefer-default-last': 'suggestion',
  'no-inverted-boolean-check': 'suggestion',
  'no-useless-catch': 'suggestion',
  'no-redundant-optional': 'suggestion',
  'prefer-immediate-return': 'suggestion',
  'no-redundant-jump': 'suggestion',
  'no-primitive-wrappers': 'problem',
  'no-skipped-tests': 'problem',
  'prefer-single-boolean-return': 'suggestion',
  'no-unthrown-error': 'problem',
  'no-tab': 'suggestion',
  'fixme-tag': 'suggestion',
  'todo-tag': 'suggestion',
  'no-sonar-comments': 'suggestion',
  'array-constructor': 'suggestion',
  'no-function-declaration-in-block': 'suggestion',
  'no-inconsistent-returns': 'suggestion',
  'no-same-line-conditional': 'suggestion',
  'no-nested-assignment': 'suggestion',
  'no-nested-incdec': 'suggestion',
  'no-useless-increment': 'suggestion',
  'class-name': 'suggestion',
  'max-lines': 'suggestion',
  'max-lines-per-function': 'suggestion',
  'nested-control-flow': 'suggestion',
});

const recommendedRuleConfig = Object.freeze({
  'no-nested-template-literals': 'error',
  'no-nested-switch': 'error',
  'no-nested-conditional': 'error',
  'no-collapsible-if': 'error',
  'no-redundant-boolean': 'error',
  'comma-or-logical-or-case': 'error',
  'no-duplicate-in-composite': 'error',
  'non-existent-operator': 'error',
  'no-identical-conditions': 'error',
  'no-all-duplicated-branches': 'error',
  'no-identical-expressions': 'error',
  'arguments-usage': 'error',
  'no-labels': 'error',
  'no-delete-var': 'error',
  'constructor-for-side-effects': 'error',
  'no-empty-character-class': 'error',
  'generator-without-yield': 'error',
  'no-exclusive-tests': 'error',
  'no-built-in-override': 'error',
  'class-prototype': 'error',
  'max-switch-cases': 'error',
  'max-union-size': 'error',
  'elseif-without-else': 'error',
  'no-case-label-in-switch': 'error',
  'for-in': 'error',
  'prefer-while': 'error',
  'no-small-switch': 'error',
  'prefer-default-last': 'error',
  'no-inverted-boolean-check': 'error',
  'no-useless-catch': 'error',
  'no-redundant-optional': 'error',
  'prefer-immediate-return': 'error',
  'no-redundant-jump': 'error',
  'no-primitive-wrappers': 'error',
  'no-skipped-tests': 'error',
  'prefer-single-boolean-return': 'error',
  'no-unthrown-error': 'error',
  'no-tab': 'error',
  'fixme-tag': 'error',
  'todo-tag': 'error',
  'no-sonar-comments': 'error',
  'array-constructor': 'error',
  'no-function-declaration-in-block': 'error',
  'no-inconsistent-returns': 'error',
  'no-same-line-conditional': 'error',
  'no-nested-assignment': 'error',
  'no-nested-incdec': 'error',
  'no-useless-increment': 'error',
  'class-name': 'error',
  'max-lines': 'error',
  'max-lines-per-function': 'error',
  'nested-control-flow': 'error',
});

const implementedRuleNames = Object.freeze(implementedSonarjsRuleNames());
const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createSonarjsRule(ruleName)]),
  ),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 0])),
  configs: {
    recommended: configFromRuleConfig('recommended', recommendedRuleConfig),
  },
});

plugin.implementedSonarjsRuleNames = implementedRuleNames;
plugin.scanSonarjs = scanSonarjs;

function configFromRuleConfig(name, ruleConfig) {
  return {
    name: `${PLUGIN_NAME}/${name}`,
    plugins: [PLUGIN_NAME],
    rules: Object.fromEntries(
      Object.entries(ruleConfig).map(([ruleName, config]) => [
        `${PLUGIN_NAME}/${ruleName}`,
        config,
      ]),
    ),
  };
}

function schemaForRule(ruleName) {
  if (ruleName === 'max-switch-cases') {
    return [
      {
        type: 'object',
        properties: { maximum: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'max-union-size') {
    return [
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'max-lines') {
    return [
      {
        type: 'object',
        properties: { maximum: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'max-lines-per-function') {
    return [
      {
        type: 'object',
        properties: { maximum: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'nested-control-flow') {
    return [
      {
        type: 'object',
        properties: { maximumNestingLevel: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  return [];
}

function scanOptionsForRule(context, ruleName) {
  const raw =
    context.options?.[0] && typeof context.options[0] === 'object' ? context.options[0] : {};
  const options = { ruleNames: [ruleName] };
  if (ruleName === 'max-switch-cases' && Number.isInteger(raw.maximum)) {
    options.maxSwitchCasesThreshold = raw.maximum;
  }
  if (ruleName === 'max-union-size' && Number.isInteger(raw.threshold)) {
    options.maxUnionSizeThreshold = raw.threshold;
  }
  if (ruleName === 'max-lines' && Number.isInteger(raw.maximum)) {
    options.maxLinesThreshold = raw.maximum;
  }
  if (ruleName === 'max-lines-per-function' && Number.isInteger(raw.maximum)) {
    options.maxLinesPerFunctionThreshold = raw.maximum;
  }
  if (ruleName === 'nested-control-flow' && Number.isInteger(raw.maximumNestingLevel)) {
    options.nestedControlFlowThreshold = raw.maximumNestingLevel;
  }
  return options;
}

function createSonarjsRule(ruleName) {
  return {
    meta: {
      type: ruleTypes[ruleName],
      docs: {
        description: ruleDescriptions[ruleName],
        recommended: recommendedRuleConfig[ruleName] !== undefined,
        url: `${DOCS_BASE}#${ruleName}`,
      },
      messages: messages[ruleName],
      schema: schemaForRule(ruleName),
    },
    createOnce(context) {
      return {
        Program() {
          for (const diagnostic of diagnosticsForRule(context, ruleName)) {
            reportDiagnostic(context, diagnostic);
          }
        },
      };
    },
  };
}

function diagnosticsForRule(context, ruleName) {
  return diagnosticsForContext(context, scanOptionsForRule(context, ruleName)).filter(
    (diagnostic) => diagnostic.ruleName === ruleName,
  );
}

function diagnosticsForContext(context, options) {
  const sourceCode = context.sourceCode ?? context.getSourceCode?.() ?? {};
  const sourceText = sourceTextForContext(context);
  const filename = context.filename ?? context.getFilename?.() ?? 'file.js';
  const key = JSON.stringify(options);
  let sourceCache = diagnosticsCache.get(sourceCode);

  if (!sourceCache) {
    sourceCache = new Map();
    diagnosticsCache.set(sourceCode, sourceCache);
  }

  const cached = sourceCache.get(key);
  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const diagnostics = scanSonarjs(sourceText, filename, options);
  sourceCache.set(key, { sourceText, filename, diagnostics });
  return diagnostics;
}

function sourceTextForContext(context) {
  const sourceCode = context.sourceCode ?? context.getSourceCode?.() ?? {};
  if (typeof sourceCode.getText === 'function') {
    return sourceCode.getText();
  }
  if (typeof sourceCode.text === 'string') {
    return sourceCode.text;
  }
  return '';
}

function reportDiagnostic(context, diagnostic) {
  const report = {
    messageId: diagnostic.messageId,
    data: compactData(diagnostic.data),
    loc: {
      start: {
        line: diagnostic.loc.startLine,
        column: diagnostic.loc.startColumn,
      },
      end: {
        line: diagnostic.loc.endLine,
        column: diagnostic.loc.endColumn,
      },
    },
  };

  if (diagnostic.fix) {
    report.fix = (fixer) =>
      fixer.replaceTextRange(
        [diagnostic.fix.start, diagnostic.fix.end],
        diagnostic.fix.replacement,
      );
  }

  context.report(report);
}

function compactData(data) {
  const out = {};
  for (const [key, value] of Object.entries(data || {})) {
    if (value != null) {
      out[key] = value;
    }
  }
  return out;
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedSonarjsRuleNames = implementedRuleNames;
module.exports.scanSonarjs = scanSonarjs;
