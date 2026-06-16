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
  'array-callback-without-return': {
    addReturn: 'Add a "return" statement to this callback.',
  },
  'declarations-in-global-scope': {
    defineLocally:
      'Move this declaration into a local scope, or attach it explicitly to the global object.',
  },
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
  'label-position': {
    removeLabel: 'Remove this label or move it directly onto a loop or switch statement.',
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
  'no-empty-group': {
    emptyGroup: 'Remove this empty group or add content to it.',
  },
  'no-empty-alternatives': {
    emptyAlternative: 'Remove this empty alternative or replace the alternation with an optional.',
  },
  'no-regex-spaces': {
    multipleSpaces: 'Use a quantifier (e.g. " {3}") instead of multiple consecutive spaces.',
  },
  'no-control-regex': {
    controlCharacter:
      'Remove this control character from the regular expression or write it as a conventional escape.',
  },
  'single-char-in-character-classes': {
    singleCharInCharacterClass: 'Replace this single-character class with the character itself.',
  },
  'duplicates-in-character-class': {
    duplicateCharacter: 'Remove this duplicated character from the character class.',
  },
  'anchor-precedence': {
    anchorPrecedence:
      'Group the alternatives or add anchors to each branch to make operator precedence explicit.',
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
  'no-invariant-returns': {
    invariantReturn:
      "This function always returns the same value; the return value does not depend on the function's logic.",
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
  'function-name': {
    renameFunction:
      'Rename this function "{{value}}" to match the regular expression "{{format}}".',
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
  'no-duplicate-string': {
    duplicateString: 'Define this repeated string literal as a constant to avoid duplication.',
  },
  'cyclomatic-complexity': {
    cyclomaticComplexity: 'Refactor this function to reduce its cyclomatic complexity.',
  },
  'no-collection-size-mischeck': {
    collectionSizeMischeck:
      'This size/length comparison is always true or always false; fix the comparison.',
  },
  'index-of-compare-to-positive-number': {
    indexOfPositive:
      'This "indexOf" check ignores index 0; compare against -1 or use ">= 0" instead.',
  },
  'no-nested-functions': {
    noNestedFunctions: 'This function is nested too deeply. Refactor to reduce nesting depth.',
  },
  'too-many-break-or-continue-in-loop': {
    tooManyBreakContinue:
      'Reduce the total number of break and continue statements in this loop to at most one.',
  },
  'void-use': {
    voidUse: 'Remove this use of the "void" operator.',
  },
  'code-eval': {
    codeEval:
      'Review this use of dynamic code execution; it can introduce security vulnerabilities.',
  },
  'prefer-promise-shorthand': {
    preferShorthand: 'Replace this trivial promise with Promise.resolve or Promise.reject.',
  },
  'pseudo-random': {
    pseudoRandom:
      'Use a cryptographically secure random number generator instead of Math.random().',
  },
  hashing: {
    weakHash: 'Use a stronger hashing algorithm for security-sensitive data.',
  },
  'no-clear-text-protocols': {
    clearTextProtocol: 'Use an encrypted protocol instead of this clear-text URL.',
  },
  'no-hardcoded-ip': {
    hardcodedIp: 'Make this IP address configurable rather than hardcoding it in source code.',
  },
  'no-global-this': {
    noGlobalThis: 'Remove this use of the global "this" object.',
  },
  'single-character-alternation': {
    singleCharAlternation: 'Replace this alternation of single characters with a character class.',
  },
  'empty-string-repetition': {
    emptyStringRepetition:
      'Rework this part of the regex to not repeat an expression that can match the empty string.',
  },
  'no-misleading-array-reverse': {
    misleadingReverse:
      'Move this array "reverse" operation to a separate statement or operate on a copy; ' +
      'it mutates the original array in place.',
  },
  'no-alphabetical-sort': {
    provideCompareFunction: 'Provide a compare function to avoid sorting elements alphabetically.',
  },
  'no-for-in-iterable': {
    noForInIterable:
      'Use a "for...of" loop instead of a "for...in" loop to iterate over this array.',
  },
  'no-associative-arrays': {
    noAssociativeArray: 'Use an object or a Map instead of this array with non-numeric keys.',
  },
  'bitwise-operators': {
    bitwiseOperator: 'Review this use of a bitwise operator; "&&" or "||" may have been intended.',
  },
  'no-same-argument-assert': {
    sameArgumentAssert:
      'Replace this assertion; the actual and expected arguments are the same expression.',
  },
  'inverted-assertion-arguments': {
    invertedArguments:
      'Swap these assertion arguments so the actual value comes first and the expected value second.',
  },
  'no-incomplete-assertions': {
    incompleteAssertion:
      'Complete this assertion: add a terminal assertion property or method call after "expect(...)".',
  },
  'for-loop-increment-sign': {
    wrongDirection:
      'This loop update moves the counter away from the termination condition, so the loop may not stop as intended.',
  },
  'no-equals-in-for-termination': {
    noEqualsInForTermination:
      'Replace this equality operator in the loop termination condition with a relational operator.',
  },
  'reduce-initial-value': {
    provideInitialValue: 'Provide an initial value to this "reduce" call.',
  },
  'no-parameter-reassignment': {
    noParameterReassignment: 'Introduce a new variable instead of reassigning this parameter.',
  },
  'no-wildcard-import': {
    noWildcardImport: 'Import only the specific members you need instead of the whole namespace.',
  },
  'updated-loop-counter': {
    noCounterUpdate:
      "Do not update the loop counter inside the loop body; use the for-statement's update clause.",
  },
  'misplaced-loop-counter': {
    misplacedCounter:
      "This loop's update clause does not modify any variable checked in its condition.",
  },
  'no-array-delete': {
    noArrayDelete:
      'Use Array.prototype.splice() to remove this element instead of the delete operator.',
  },
  'no-literal-call': {
    noLiteralCall: 'This literal cannot be called as a function and throws a TypeError at runtime.',
  },
  'shorthand-property-grouping': {
    groupShorthand:
      'Group all shorthand properties together at the beginning or end of this object declaration.',
  },
  'process-argv': {
    processArgv: 'Make sure that reading command-line arguments is safe here.',
  },
  'standard-input': {
    standardInput: 'Make sure that reading from the standard input is safe here.',
  },
  'no-code-after-done': {
    noCodeAfterDone: 'Refactor this test; the code after the "done()" call will run unexpectedly.',
  },
  'function-inside-loop': {
    noFunctionInLoop: 'Refactor this code; do not define functions inside loops.',
  },
  'no-useless-intersection': {
    uselessIntersection:
      'Remove this "any", "never", or "unknown" member; it makes the whole intersection type pointless.',
  },
  'use-type-alias': {
    useTypeAlias: 'Replace this repeated union/intersection type with a named type alias.',
  },
  'public-static-readonly': {
    publicStaticReadonly: 'Make this public static field "readonly".',
  },
  'call-argument-line': {
    sameLineAsCallee:
      'Make the arguments of this call start on the same line as the function name.',
  },
  'prefer-object-literal': {
    preferObjectLiteral: 'Declare this object with its properties in a single object literal.',
  },
  'no-undefined-argument': {
    removeUndefined: "Remove this redundant 'undefined' argument.",
  },
  'no-identical-functions': {
    identicalFunctions:
      'Update this function so that its implementation is not identical to the one on line {{value}}.',
  },
  'no-in-misuse': {
    inMisuse:
      "Use 'indexOf' or 'includes' to check for the presence of a value in this array; the 'in' operator only checks property keys.",
  },
  'no-require-or-define': {
    noRequireOrDefine: "Use a standard 'import' statement instead of 'require' or 'define'.",
  },
  'no-invalid-regexp': {
    invalidRegExp:
      'This regular expression is invalid; fix the pattern (or flags) passed to RegExp.',
  },
  'no-extra-arguments': {
    extraArguments: 'This function is called with more arguments than it declares ({{value}}).',
  },
  'link-with-target-blank': {
    targetBlankNoOpener:
      "Add 'rel=\"noopener\"' (or 'noreferrer') to this link with target=\"_blank\" to prevent the opened page from accessing 'window.opener'.",
  },
  'no-weak-cipher': {
    weakCipher: 'Use a modern cipher instead of this weak cipher algorithm.',
  },
  'no-hardcoded-passwords': {
    hardcodedPassword:
      'Remove this hardcoded password; provide credentials via configuration or environment instead.',
  },
  'no-ignored-exceptions': {
    ignoredException: 'Handle this exception or explain in a comment why it can be safely ignored.',
  },
  'no-unused-function-argument': {
    unusedFunctionArgument:
      'Remove this unused trailing function parameter or rename it with a leading underscore to mark it as intentional.',
  },
  'object-alt-content': {
    objectAltContent:
      'Add an accessible text alternative to this <object> element (child content, aria-label, aria-labelledby, or title).',
  },
  'no-use-of-empty-return-value': {
    useOfEmptyReturnValue:
      'Remove this use of the return value of a function that does not return anything.',
  },
  'no-duplicated-branches': {
    duplicatedBranch:
      "This branch's implementation is duplicated on another branch; merge or differentiate them.",
  },
  'block-scoped-var': {
    blockScopedVar:
      "Declare this variable in the enclosing scope, or use 'let'/'const' — it is a 'var' used outside the block where it is declared.",
  },
  'no-variable-usage-before-declaration': {
    usedBeforeDeclaration: 'This variable is used before its declaration; declare it before use.',
  },
  'arguments-order': {
    argumentsOrder:
      'These arguments match the parameter names but are passed in a different order; check for swapped arguments.',
  },
  'updated-const-var': {
    updateConst: 'Correct this attempt to modify "{{value}}" or use "let" in its declaration.',
  },
  'unicode-aware-regex': {
    unicodeAwareRegex:
      "Add the 'u' flag to this regular expression so its Unicode property escape (\\p{...}) works correctly.",
  },
  'no-undefined-assignment': {
    noUndefinedAssignment:
      "Do not explicitly assign 'undefined'; use 'null' or leave the variable uninitialized.",
  },
  'no-empty-after-reluctant': {
    emptyAfterReluctant:
      'This reluctant quantifier will always match the empty string because everything that follows it is optional or there is nothing after it; review the pattern.',
  },
  'no-ignored-return': {
    ignoredReturn:
      'The return value of this pure method call is discarded; use the result or remove the call.',
  },
  'file-name-differ-from-class': {
    fileNameDifferFromClass:
      'Rename this file to match the name of the class it exports, or rename the class to match the file.',
  },
  'no-unenclosed-multiline-block': {
    unenclosedMultilineBlock:
      'This line is indented as if it belongs to the preceding unbraced block, but it does not; add braces or fix the indentation.',
  },
  'inconsistent-function-call': {
    inconsistentFunctionCall:
      "This function is invoked both as a constructor (with 'new') and as a plain function; use it consistently.",
  },
  'new-operator-misuse': {
    newOperatorMisuse:
      "Do not use 'new' with an arrow function; arrow functions are not constructors and this throws a TypeError.",
  },
  'no-empty-test-file': {
    emptyTestFile: "This test file does not contain any test cases ('it'/'test').",
  },
  deprecation: {
    deprecatedUse:
      'Do not use code that is marked as deprecated; replace it with the recommended alternative.',
  },
  'cognitive-complexity': {
    cognitiveComplexity: 'Refactor this function to reduce its Cognitive Complexity.',
  },
  'expression-complexity': {
    expressionComplexity:
      'Refactor this expression to reduce the number of logical and conditional operators.',
  },
  'prefer-regexp-exec': {
    preferRegExpExec:
      'Use RegExp.prototype.exec() instead of String.prototype.match() for a non-global regular expression.',
  },
  'no-fallthrough': {
    noFallthrough:
      'End this switch case with an unconditional jump, or add an intentional fallthrough comment.',
  },
  'no-commented-code': {
    commentedCode: 'Remove this commented-out code.',
  },
  'destructuring-assignment-syntax': {
    useDestructuring:
      'Use destructuring to merge these consecutive property extractions into a single declaration.',
  },
  'no-element-overwrite': {
    elementOverwrite:
      'This collection element is overwritten before it is read; the earlier assignment is dead.',
  },
  'no-redundant-assignments': {
    redundantAssignment:
      'This assignment is redundant; the value is never used before being overwritten.',
  },
  'no-unused-collection': {
    unusedCollection:
      'The contents of this collection are never read; the collection is only ever written to.',
  },
  'no-empty-collection': {
    emptyCollection:
      'This collection is empty and is never populated; reading from it has no effect.',
  },
  'no-redundant-parentheses': {
    redundantParentheses: 'Remove these redundant parentheses.',
  },
  'bool-param-default': {
    boolParamDefault:
      'Provide a default value for this optional boolean parameter so callers are not forced to reason about three states.',
  },
  'post-message': {
    postMessage: 'Specify a target origin instead of "*" for this cross-document message.',
  },
  'in-operator-type-error': {
    inOperatorTypeError:
      'The right operand of "in" must be an object; a primitive value here throws a TypeError at runtime.',
  },
  'different-types-comparison': {
    differentTypesComparison:
      'These operands have different primitive types, so this strict comparison is always constant.',
  },
  'operation-returning-nan': {
    operationReturningNan: 'This arithmetic operation will always evaluate to NaN.',
  },
  'production-debug': {
    productionDebug: 'Remove this "debugger" statement.',
  },
  'no-hardcoded-secrets': {
    hardcodedSecret: 'Revoke and change this secret, as it is compromised by being hardcoded here.',
  },
  'concise-regex': {
    conciseRegex:
      'Use a concise character class shorthand (\\d, \\D, or \\w) instead of this verbose character class.',
  },
  'no-misleading-character-class': {
    misleadingCharacterClass:
      'This character class contains a multi-code-unit character; without the "u" flag it is split into surrogate halves and will not match as intended.',
  },
  'slow-regex': {
    slowRegex:
      'This nested quantifier can cause catastrophic backtracking (super-linear runtime) on crafted input.',
  },
  'web-sql-database': {
    webSqlDatabase:
      'The Web SQL Database API is deprecated and removed from the web platform; do not use it.',
  },
  'no-intrusive-permissions': {
    intrusivePermission:
      'Requesting this browser permission is intrusive; make sure it is necessary and properly justified.',
  },
  'encryption-secure-mode': {
    insecureCipherMode: 'Use a secure cipher mode (e.g. GCM); ECB and CBC are vulnerable.',
  },
  'no-unsafe-unzip': {
    unsafeUnzip:
      'Expanding this archive without limiting size or entry count risks a zip-bomb denial of service.',
  },
  'disabled-timeout': {
    disabledTimeout:
      'This timeout value overflows the 32-bit range and silently disables the timeout; use 0 to disable intentionally or a value within range.',
  },
  'cookie-no-httponly': {
    cookieNoHttpOnly:
      'Set this cookie\'s "httpOnly" flag to true to make it inaccessible to client-side scripts.',
  },
  'content-security-policy': {
    contentSecurityPolicy:
      'Do not disable Content Security Policy (contentSecurityPolicy: false) — it is an important defense against XSS.',
  },
  'certificate-transparency': {
    certificateTransparency:
      'Do not disable Certificate Transparency monitoring (expectCt: false).',
  },
  csrf: {
    csrf: 'Do not disable CSRF protection for state-changing HTTP methods (POST/PUT/DELETE/PATCH).',
  },
  'file-permissions': {
    weakFilePermissions:
      'Make sure this permissive file access is safe; granting access to "others" can be a security risk.',
  },
  'file-uploads': {
    fileUploads:
      'Configure an explicit upload destination; without it, uploaded files are written to the OS temporary directory.',
  },
  cors: {
    cors: 'This permissive CORS policy allows any origin; restrict it to trusted origins.',
  },
  'dns-prefetching': {
    dnsPrefetching:
      'Enabling DNS prefetching can leak information about the links a user is offered; disable it unless required.',
  },
  'disabled-auto-escaping': {
    disabledAutoEscaping: 'Do not disable template auto-escaping; it is a key defense against XSS.',
  },
  'aws-s3-bucket-granted-access': {
    s3PublicAccess:
      'This S3 bucket access control grants access beyond the owner; use a private access control.',
  },
  'aws-rds-unencrypted-databases': {
    rdsUnencrypted: 'Enable encryption at rest for this database (storageEncrypted: true).',
  },
  'aws-iam-public-access': {
    iamPublicAccess:
      'This policy grants public access to all AWS accounts; restrict the principal.',
  },
  'hidden-files': {
    hiddenFiles:
      "Serving hidden files (dotfiles: 'allow') can expose sensitive files like .env or .git; use 'ignore' or 'deny'.",
  },
  'aws-sqs-unencrypted-queue': {
    sqsUnencrypted: 'Enable server-side encryption for this SQS queue.',
  },
  'aws-apigateway-public-api': {
    apigatewayPublicApi:
      'This API method has no authorization (authorizationType: NONE); require authorization or confirm public access is intended.',
  },
  'aws-iam-all-privileges': {
    iamAllPrivileges:
      'This policy grants all actions ("*"); grant only the specific actions required.',
  },
  'aws-s3-bucket-versioning': {
    s3BucketVersioning:
      'Versioning is disabled on this S3 bucket; enable it to protect against accidental or malicious data loss.',
  },
  'aws-ec2-rds-dms-public': {
    ec2RdsDmsPublic: 'Make sure allowing public network access to this resource is safe.',
  },
  'aws-s3-bucket-public-access': {
    s3BucketPublicAccess:
      'This S3 bucket does not block public access; ensure public exposure is intended.',
  },
  'confidential-information-logging': {
    confidentialLogging:
      'Configure secret-masking patterns for this logger so confidential information is not logged.',
  },
  'aws-iam-all-resources-accessible': {
    iamAllResources:
      'This policy grants access to all resources ("*"); scope it to the specific resources required.',
  },
  'aws-ec2-unencrypted-ebs-volume': {
    ebsUnencrypted: 'Enable encryption for this EBS volume (encrypted: true).',
  },
  'aws-efs-unencrypted': {
    efsUnencrypted: 'Enable encryption at rest for this EFS file system (encrypted: true).',
  },
  'aws-restricted-ip-admin-access': {
    restrictedIpAdminAccess:
      'Restrict access to administration ports (SSH/RDP) to specific trusted IP ranges instead of all addresses.',
  },
  'redundant-type-aliases': {
    redundantTypeAlias:
      'This type alias only renames an existing type and adds no value; use the underlying type directly.',
  },
  'jsx-no-leaked-render': {
    jsxNoLeakedRender:
      'Convert this `&&` to a boolean condition (e.g. `x.length > 0`); a numeric left operand can leak 0 into the rendered output.',
  },
  'no-uniq-key': {
    noUniqKey:
      'Do not use a random or time-based value as a React key; it changes every render and defeats reconciliation.',
  },
  'insecure-cookie': {
    insecureCookie: 'Set this cookie\'s "secure" flag to true so it is only sent over HTTPS.',
  },
  'no-hook-setter-in-body': {
    noHookSetterInBody:
      'Do not call a useState setter directly in the component body; it triggers an infinite re-render. Move it into an event handler or effect.',
  },
  'content-length': {
    contentLength:
      'This file-size limit is very large; cap upload size to mitigate denial-of-service risk.',
  },
  'unverified-certificate': {
    unverifiedCertificate:
      'Enable server certificate validation (do not set rejectUnauthorized to false).',
  },
  'no-mime-sniff': {
    noMimeSniff: 'Do not disable the X-Content-Type-Options: nosniff protection (noSniff: false).',
  },
  'no-ip-forward': {
    noIpForward:
      'Forwarding the client IP (xfwd: true) can enable IP-based access-control bypass; ensure this is safe.',
  },
  'no-angular-bypass-sanitization': {
    angularBypassSanitization:
      "Bypassing Angular's built-in sanitization is security-sensitive; ensure the value is trusted.",
  },
  'insecure-jwt-token': {
    insecureJwtToken:
      'Do not use the "none" algorithm for JWTs; it disables signature verification.',
  },
  'xml-parser-xxe': {
    xmlParserXxe:
      'Disable external entity expansion (do not set noent: true) to prevent XXE attacks.',
  },
  'no-useless-react-setstate': {
    noUselessReactSetstate: 'This setState call passes the current state value and has no effect.',
  },
  'no-referrer-policy': {
    noReferrerPolicy:
      "This Referrer-Policy value leaks the full URL to other origins; use a stricter policy like 'no-referrer' or 'same-origin'.",
  },
  'weak-ssl': {
    weakSsl:
      'Use a strong TLS protocol version (TLS 1.2 or higher); this configures a weak/deprecated protocol.',
  },
  'no-weak-keys': {
    weakKeys: 'Use a strong key size (RSA/DSA/DH >= 2048 bits, or a strong EC curve).',
  },
  'strict-transport-security': {
    strictTransportSecurity:
      'Strengthen this HSTS policy (enable includeSubDomains and use a long max-age).',
  },
  'unverified-hostname': {
    unverifiedHostname:
      'This checkServerIdentity override disables TLS hostname verification; validate the server hostname.',
  },
  'frame-ancestors': {
    frameAncestors:
      'Make sure this Content Security Policy frame-ancestors directive is safe here.',
  },
  'no-table-as-layout': {
    noTableAsLayout:
      'Do not use an HTML <table> for layout; a table with role="presentation"/"none" confuses screen readers. Use CSS instead.',
  },
  'no-vue-bypass-sanitization': {
    noVueBypassSanitization:
      'Make sure disabling Vue.js built-in escaping (rendering raw HTML) is safe here.',
  },
  'os-command': {
    osCommand: 'Make sure using a shell to execute this OS command is safe here.',
  },
  'argument-type': {
    argumentType: 'Pass a number to this Math function; this argument evaluates to a boolean.',
  },
  'aws-s3-bucket-insecure-http': {
    s3BucketInsecureHttp:
      'Enforce HTTPS-only access on this S3 bucket (set enforceSSL: true) instead of allowing insecure HTTP.',
  },
  'aws-s3-bucket-server-encryption': {
    s3BucketServerEncryption:
      'Do not disable server-side encryption on this S3 bucket; use a managed or KMS encryption mode instead of UNENCRYPTED.',
  },
});

const ruleDescriptions = Object.freeze({
  'array-callback-without-return':
    'Require a return statement in callbacks of array methods that build a result',
  'declarations-in-global-scope':
    'Disallow function and var declarations that create global or module-scope bindings',
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
  'label-position': 'Disallow labels on statements other than loops and switch statements',
  'no-delete-var':
    "Disallow 'delete' applied to a plain variable; use it only on object properties",
  'constructor-for-side-effects':
    'Disallow using new solely for side effects without capturing or using the constructed object',
  'no-empty-character-class':
    'Disallow empty character classes in regular expression literals, which can never match',
  'no-empty-group':
    'Disallow empty capturing or non-capturing groups in regular expression literals',
  'no-empty-alternatives':
    'Disallow empty alternatives in a regular expression alternation (a stray, leading, or trailing "|")',
  'no-regex-spaces':
    'Disallow multiple consecutive spaces in a regular expression; use an explicit quantifier instead',
  'no-control-regex':
    'Disallow control characters written as \\x, \\u, or \\c escapes in regular expressions',
  'single-char-in-character-classes':
    'Disallow a regular-expression character class that contains only a single literal character',
  'duplicates-in-character-class':
    'Disallow the same literal character appearing more than once in a regular-expression character class',
  'anchor-precedence':
    'Disallow regex alternations where ^ or $ anchors only one branch due to operator precedence',
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
  'no-invariant-returns':
    "Disallow functions that always return the same value regardless of their logic; the return value should depend on the function's input or state",
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
  'function-name': 'Require function and method names to match the configured regular expression',
  'max-lines':
    'Disallow files with more code lines than the configured maximum (the "maximum" option; default 1000); blank lines and comment-only lines are not counted',
  'max-lines-per-function':
    'Disallow functions with more code lines than the configured maximum (the "maximum" option; default 200); IIFEs and JSX-containing functions are excluded',
  'nested-control-flow':
    'Disallow control flow statements (if/for/for-in/for-of/while/do-while/switch/try) nested beyond the configured maximumNestingLevel (default 3); else-if chains do not add a level',
  'no-duplicate-string':
    'Disallow string literals of 10+ characters containing a non-word character from appearing at least threshold (default 3) times in a file; import/export sources and JSX attribute values are excluded',
  'cyclomatic-complexity':
    'Disallow functions whose cyclomatic complexity exceeds the configured threshold (the "threshold" option; default 10); each if/for/while/do-while/case/catch/ternary/logical-operator adds +1',
  'no-collection-size-mischeck':
    'Disallow comparisons of collection .length or .size against 0 with < or >= that are always false or always true',
  'index-of-compare-to-positive-number':
    'Disallow comparing the result of indexOf or lastIndexOf against a positive number, which silently excludes the element at index 0',
  'no-nested-functions':
    'Disallow functions nested more deeply than the configured threshold (the "threshold" option; default 4); applies to function declarations, function expressions, and arrow functions',
  'too-many-break-or-continue-in-loop':
    'Disallow more than one break or continue statement targeting the same loop; having two or more jumps makes control flow hard to follow',
  'code-eval':
    'Disallow dynamic code execution via eval() or the Function constructor, which can introduce security vulnerabilities',
  'void-use':
    "Disallow the 'void' operator; write 'undefined' directly or restructure the code to avoid discarding values",
  'prefer-promise-shorthand':
    'Disallow new Promise(executor) when the executor immediately calls resolve or reject with at most one argument; use Promise.resolve or Promise.reject instead',
  'pseudo-random':
    'Disallow Math.random() in security-sensitive contexts; use a cryptographically secure random number generator instead',
  hashing: 'Disallow obsolete hashing algorithms such as MD5 and SHA-1 in crypto hashing APIs',
  'no-clear-text-protocols':
    'Disallow clear-text URL protocols such as http, ftp, telnet, ws, and ldap in string literals',
  'no-hardcoded-ip':
    'Disallow hardcoded IP addresses in string literals; make them configurable instead',
  'no-global-this':
    'Disallow references to the global "this" object outside of any function or class scope ' +
    'that rebinds "this"; prefer accessing global properties directly',
  'single-character-alternation':
    'Disallow regex alternations where every alternative is a single character; ' +
    'use a character class instead',
  'empty-string-repetition':
    'Disallow repetition quantifiers applied to a sub-pattern that can match the empty string',
  'no-misleading-array-reverse':
    'Disallow using the return value of the in-place array-mutating methods ' +
    '"reverse" and "sort" as if they returned a new array',
  'no-alphabetical-sort':
    'Require a compare function when calling "sort" or "toSorted" on an array',
  'no-for-in-iterable':
    'Disallow "for...in" loops over arrays; use a "for...of" loop to iterate values instead',
  'no-associative-arrays':
    'Disallow assigning non-numeric keys to arrays; use an object or a Map instead',
  'bitwise-operators':
    'Disallow a bitwise "&" or "|" whose operand is a boolean-valued expression (comparison, logical, "!", or boolean literal), which is likely a typo for "&&" or "||"',
  'no-same-argument-assert':
    'Disallow a Chai "assert.<method>(...)" call whose first two arguments are the same source expression, since it compares a value to itself and is trivially true',
  'inverted-assertion-arguments':
    'Disallow a Chai "assert.<method>(...)" call whose first argument is a literal constant and second is not, since the actual/expected arguments were likely inverted',
  'no-incomplete-assertions':
    'Disallow a Chai BDD "expect(value)" chain used as a statement that never reaches a terminal assertion property or method call, so the test always passes silently',
  'for-loop-increment-sign':
    'Disallow a "for" loop whose update clause moves the counter away from the relational termination condition (e.g. "i < n" with "i--"), which can prevent the loop from terminating',
  'no-equals-in-for-termination':
    'Disallow an equality operator ("==", "!=", "===", "!==") in a "for" loop termination condition when the counter is advanced by a non-unit step (e.g. "i != 10" with "i += 2"), which can skip the bound and loop forever',
  'reduce-initial-value':
    'Require an initial value (second argument) when calling "reduce" or "reduceRight" on an array, to avoid a TypeError on empty arrays and a skipped first element',
  'no-parameter-reassignment':
    'Disallow reassigning a function parameter, caught exception, or for-in/for-of loop variable, which discards the value supplied at runtime; introduce a new local variable instead',
  'no-wildcard-import':
    'Disallow wildcard (namespace) imports such as \'import * as ns from "mod"\'; import only the specific members you need to keep code readable and tree-shakeable',
  'updated-loop-counter':
    'Disallow updating a classic "for" loop counter inside the loop body (reassignment, compound assignment, or increment/decrement); advance the counter only in the update clause',
  'misplaced-loop-counter':
    'Disallow a classic "for" loop whose update clause modifies only variable(s) absent from the loop condition (e.g. "i < 10" with "j++"), since the tested counter is never advanced by the header',
  'no-array-delete':
    'Disallow the "delete" operator on array elements (e.g. "delete arr[0]"), which leaves a hole without updating the array length; use Array.prototype.splice() instead',
  'no-literal-call':
    'Disallow calling a literal as a function or using it as a tagged-template tag (e.g. "true()" or "true`x`"); a literal is never callable and always throws a TypeError at runtime',
  'shorthand-property-grouping':
    'Require shorthand object properties to be grouped as one contiguous block at the beginning or end of the object literal, rather than interleaved with regular "key: value" properties',
  'process-argv':
    'Flag reads of process.argv, since using command-line arguments is security-sensitive and should be reviewed',
  'standard-input':
    'Flag reads of process.stdin, since reading from the standard input is security-sensitive and should be reviewed',
  'no-code-after-done':
    'Flag statements that run after a "done()" callback call in a Mocha test or hook, since the test is already finished and the trailing code executes unexpectedly',
  'function-inside-loop':
    'Flag functions (declarations, expressions, or arrow functions) defined inside a loop, since such closures are error-prone and inefficient; immediately invoked function expressions are exempt',
  'no-useless-intersection':
    'Disallow a TypeScript intersection type that contains an "any", "never", or "unknown" keyword member, which collapses or is absorbed by the intersection and makes it pointless (syntactic keyword cases only; type-aware subtype/supertype redundancy is out of scope)',
  'use-type-alias':
    'Require extracting a union or intersection type into a named type alias when the same composite type (compared by exact source text, order-sensitive) appears at least 3 times in a file; the first occurrence of each repeated type is reported',
  'public-static-readonly':
    'Require a publicly accessible static class field (explicit "public" or no accessibility modifier) to be declared "readonly"; static fields that are private, protected, readonly, declared with a "#private" key, or non-static are not flagged (modifier-based; applies to both JS and TS source)',
  'call-argument-line':
    "Require a function call's opening parenthesis (and therefore its argument list) to begin on the same line as the end of the callee; writing the call's parentheses on the line below the function name is reported, while wrapping the arguments across lines with the open parenthesis still on the callee's line is allowed",
  'prefer-object-literal':
    'Require an object to be created and initialized with a single object literal rather than declared as an empty object and then populated with property assignments; flags an empty object-literal declaration whose immediately following sibling statement assigns to a property of that same variable',
  'no-undefined-argument':
    "Disallow passing the literal 'undefined' as the last argument to a function or constructor call; omitting it is equivalent and avoids redundant noise",
  'no-identical-functions':
    'Disallow two functions in the same file whose parameter list and body are byte-for-byte identical; such duplication is almost always a copy-paste mistake or a missed abstraction (functions spanning fewer than 3 lines are exempt)',
  'no-in-misuse':
    "Disallow using the 'in' operator to test whether a value is an element of an array; use 'Array.prototype.includes' or 'Array.prototype.indexOf' instead, because 'in' checks property keys, not values",
  'no-require-or-define':
    "Disallow CommonJS 'require()' calls and AMD 'define()' calls; use ES module 'import' statements instead",
  'no-invalid-regexp':
    'Disallow a syntactically invalid regular expression pattern or flag string passed as a string literal to the RegExp constructor; applies to both new RegExp(...) and RegExp(...) call forms',
  'no-extra-arguments':
    'Disallow calling a function with more arguments than the function declares parameters (conservative: only const/let/var-assigned function expressions and arrow functions are checked)',
  'link-with-target-blank':
    'Require <a> and <area> JSX elements with target="_blank" to also carry a rel attribute containing "noopener" or "noreferrer", to prevent the opened page from accessing window.opener (reverse-tabnabbing)',
  'no-weak-cipher':
    'Disallow weak cipher algorithms such as DES, RC2, RC4, Blowfish, and IDEA in Node-style crypto cipher APIs',
  'no-hardcoded-passwords':
    'Disallow hardcoded password string literals assigned to a password-named identifier; ' +
    'provide credentials via configuration or environment variables instead',
  'no-ignored-exceptions':
    'Disallow empty catch blocks that silently swallow exceptions; ' +
    'at minimum log or rethrow the exception, or add a comment explaining why it is safe to ignore',
  'no-unused-function-argument':
    'Disallow trailing function parameters that are never referenced; parameters that appear ' +
    'before a used parameter are exempt (they cannot be removed without changing call sites)',
  'object-alt-content':
    'Require <object> JSX elements to provide an accessible text alternative via child content, ' +
    'aria-label, aria-labelledby, or title; elements explicitly hidden with aria-hidden="true" are exempt',
  'no-use-of-empty-return-value':
    'Disallow using the return value of a function that does not explicitly return a value; ' +
    'such a function always returns undefined, so consuming its result is almost always a bug',
  'no-duplicated-branches':
    'Disallow any two branches in an if/else-if/else chain, or any two case/default clauses in a ' +
    'switch statement, from having byte-identical implementations; merge or differentiate them',
  'block-scoped-var':
    "Disallow 'var' declarations inside a block (if/for/while/do/switch/bare-block) when the binding" +
    ' is used outside that block; use block-scoped let/const or declare the variable in the enclosing scope',
  'no-variable-usage-before-declaration':
    'Disallow referencing a variable before its var/let/const declaration appears in the source ' +
    'text; function declarations are excluded because their hoisting is intentional',
  'arguments-order':
    'Disallow calling a function with arguments that match the parameter names but in a ' +
    'transposed (swapped) order; detects only calls where every argument is a plain identifier ' +
    'whose name is one of the declared parameter names, reordered',
  'updated-const-var':
    'Disallow assigning to a const binding, including update expressions, destructuring assignments, and for-in/for-of assignment targets',
  'unicode-aware-regex':
    "Disallow Unicode property escapes (\\p{...} or \\P{...}) in regular expressions that lack the 'u' or 'v' flag, " +
    "since without those flags the engine treats \\p as a literal 'p'",
  'no-undefined-assignment':
    "Disallow explicitly assigning the bare identifier 'undefined' in an assignment expression; " +
    "use 'null' to clear a value or leave the variable uninitialized instead",
  'no-empty-after-reluctant':
    'Disallow a reluctant (lazy) quantifier that can match the empty string when every following ' +
    'term in the same alternative is also optional or absent, making the quantifier always match empty',
  'no-ignored-return':
    'Disallow discarding the return value of a pure built-in method call when the receiver is a ' +
    'literal of a statically-known type (string, number, or array literal); restricted to ' +
    'literal receivers only to avoid false positives in the absence of type information',
  'file-name-differ-from-class':
    'Require that a file exporting exactly one named class be named after that class; ' +
    'the comparison strips hyphens and underscores and ignores case so that ' +
    '"MyClass", "my-class", and "my_class" all match each other',
  'no-unenclosed-multiline-block':
    'Disallow a sibling statement that is indented as if it belongs to a preceding unbraced ' +
    'control-structure body (if/for/while/else without braces), since it always executes ' +
    'unconditionally despite appearing guarded',
  'inconsistent-function-call':
    'Disallow calling a function both as a plain call and as a constructor (with new) within ' +
    'the same file; pick one calling convention and apply it consistently',
  'new-operator-misuse':
    "Disallow using 'new' with an arrow function; arrow functions cannot be constructors and always throw a TypeError",
  'no-empty-test-file':
    'Require test files (whose name contains .test. or .spec.) to contain at least one it() or test() call; a test file with no test cases is always a bug',
  deprecation:
    'Disallow using a same-file function or class whose declaration is immediately preceded by a ' +
    'block comment containing @deprecated; covers only local declarations to avoid false positives ' +
    'in the absence of cross-module type information',
  'cognitive-complexity':
    'Disallow functions whose Cognitive Complexity exceeds the configured threshold (the "threshold" option; default 15); ' +
    'nesting depth is factored into the increment for each structural element',
  'expression-complexity':
    'Disallow expressions with more than the configured number of logical (&&, ||, ??) ' +
    'and conditional (?:) operators (the "threshold" option; default 3); ' +
    'operators inside nested function or arrow-function bodies are counted independently',
  'prefer-regexp-exec':
    'Prefer RegExp.prototype.exec() over String.prototype.match() when matching a non-global regular-expression literal',
  'no-fallthrough':
    'Require non-empty switch cases to end with an unconditional jump, unless an intentional fallthrough comment is present',
  'no-commented-code':
    'Disallow sections of code that have been commented out; ' +
    'commented-out code makes files harder to read and should be removed',
  'destructuring-assignment-syntax':
    'Require destructuring when two or more consecutive single-declarator const/let statements ' +
    'each extract a property from the same plain identifier and the binding name matches the ' +
    'property name (e.g. const a = obj.a; const b = obj.b; → const { a, b } = obj;)',
  'no-element-overwrite':
    'Disallow writing to the same collection element (array index or object property) twice in ' +
    'consecutive statements with no read in between; the first write is dead and is almost ' +
    'always a bug (wrong index or key)',
  'no-redundant-assignments':
    'Disallow assignments whose value is never used before being overwritten; ' +
    'flags self-assignments (x = x) and consecutive assignments to the same plain identifier ' +
    'with no intervening statement where the first value is immediately replaced',
  'no-unused-collection':
    'Disallow const/let collection bindings (array, object, Map, Set, etc.) that are ' +
    'populated via mutating operations but whose contents are never read',
  'no-empty-collection':
    'Disallow reading from const/let array-like collection bindings (array, Map, Set, ' +
    'WeakMap, WeakSet) that are initialised empty and are never populated with any element',
  'no-redundant-parentheses':
    'Disallow redundant parentheses; flags only the unambiguous nested ' +
    'double-parenthesis subset ((x)) where the inner pair adds no grouping',
  'bool-param-default':
    'Require a default value for optional boolean function parameters typed exactly as ' +
    'the boolean keyword, so callers need not distinguish false from an omitted argument',
  'post-message':
    'Disallow sending cross-document messages with a "*" wildcard target origin; flags ' +
    'only the unambiguous postMessage(message, "*") sending subset',
  'in-operator-type-error':
    'Disallow the "in" operator with a primitive literal right operand (string, number, ' +
    'bigint, boolean, or null), which always throws a TypeError at runtime',
  'different-types-comparison':
    'Disallow strict equality (===/!==) between two primitive literals of provably ' +
    'different types, where the comparison is always constant; flags only the ' +
    'unambiguous literal-vs-literal subset',
  'operation-returning-nan':
    'Disallow arithmetic operations (excluding +) where an operand is a function, class, or ' +
    'plain object literal that always coerces to NaN',
  'production-debug':
    'Disallow leaving debug features active in production code; flags only the ' +
    'unambiguous "debugger" statement (console.* and alert/confirm/prompt are not flagged)',
  'no-hardcoded-secrets':
    'Disallow hardcoded secret/credential string literals assigned to a secret-named identifier ' +
    '(secret, apiKey, token, access_token, private_key, etc.); ' +
    'provide secrets via configuration or environment variables instead',
  'concise-regex':
    'Prefer concise character class shorthands; flags only the exact verbose forms ' +
    '[0-9] (\\d), [^0-9] (\\D), and [A-Za-z0-9_] (\\w)',
  'no-misleading-character-class':
    'Disallow astral (multi-code-unit) characters in a regex character class without the ' +
    '"u"/"v" flag, where they are silently split into surrogate halves',
  'slow-regex':
    'Disallow super-linear regular expressions; flags only the nested unbounded ' +
    'quantifier shape (e.g. (a+)+) that causes catastrophic backtracking',
  'web-sql-database':
    'Disallow use of the deprecated, removed, and security-sensitive Web SQL Database ' +
    'API; flags any call to openDatabase(...) (global or as a member, e.g. window.openDatabase)',
  'no-intrusive-permissions':
    'Flag requests for intrusive browser permissions; matches the distinctive ' +
    'geolocation, Notification.requestPermission, and permissions.query call chains',
  'encryption-secure-mode':
    'Disallow insecure block-cipher modes; flags Node crypto cipher/decipher factory ' +
    'calls whose string-literal cipher spec names the ECB or CBC mode (e.g. aes-128-cbc)',
  'no-unsafe-unzip':
    'Flag archive expansion that risks a zip-bomb denial of service; matches the ' +
    'distinctive adm-zip extractAllTo(...) method only (deliberately narrow to stay zero-false-positive)',
  'disabled-timeout':
    'Disallow Mocha this.timeout() values that overflow the 32-bit setTimeout range ' +
    '(greater than 2147483647) and thereby silently disable the timeout instead of applying it',
  'cookie-no-httponly':
    'Flag cookie/session configuration that disables the HttpOnly flag; matches an ' +
    'object property "httpOnly" set to the boolean literal false',
  'content-security-policy':
    'Flag helmet configuration that disables the Content Security Policy; matches an ' +
    'object property "contentSecurityPolicy" set to the boolean literal false',
  'certificate-transparency':
    'Flag helmet configuration that disables Certificate Transparency monitoring; ' +
    'matches an object property "expectCt" set to the boolean literal false',
  csrf:
    'Flag csurf middleware configuration that disables CSRF protection for unsafe ' +
    'HTTP methods; matches a csrf({ ignoreMethods: [...] }) call whose array lists ' +
    'a state-changing verb (POST/PUT/DELETE/PATCH)',
  'file-permissions':
    'Flag fs chmod-family calls or process.umask calls whose numeric mode grants ' +
    'permissions to "others"; only numeric-literal modes are checked (zero-false-positive)',
  'file-uploads':
    'Flag multer.diskStorage configuration that omits an explicit destination; ' +
    'matches a diskStorage({ ... }) call whose object-literal first argument has no ' +
    '"destination" property (uploaded files would default to the OS temporary directory)',
  cors:
    'Flag permissive CORS configurations that trust any origin with the wildcard "*"; ' +
    'matches setHeader("Access-Control-Allow-Origin", "*"), cors({ origin: "*" }), and ' +
    'a headers object literal with Access-Control-Allow-Origin set to "*"',
  'dns-prefetching':
    'Flag helmet configuration that re-enables DNS prefetching; matches a ' +
    'dnsPrefetchControl({ allow: true }) call whose allow property is the boolean ' +
    'literal true',
  'disabled-auto-escaping':
    "Flag disabling a template engine's HTML auto-escaping; matches an object property " +
    '"noEscape" set to the boolean literal true (Handlebars) and assigning to Mustache.escape',
  'aws-s3-bucket-granted-access':
    'Flag an AWS CDK S3 bucket configured with an access control that grants access beyond ' +
    'the owner; matches an "accessControl" property whose value is a BucketAccessControl member ' +
    'named PUBLIC_READ, PUBLIC_READ_WRITE, or AUTHENTICATED_READ',
  'aws-rds-unencrypted-databases':
    'Flag an AWS CDK RDS database or cluster created with encryption at rest disabled; ' +
    'matches an object property "storageEncrypted" set to the boolean literal false',
  'aws-iam-public-access':
    'Flag AWS CDK IAM policies that grant public access to all AWS accounts; matches a ' +
    'new-expression whose callee is the AnyPrincipal class (a bare AnyPrincipal identifier ' +
    'or a member expression such as iam.AnyPrincipal)',
  'hidden-files':
    'Flag static file servers configured to serve dotfiles; matches an object ' +
    'property "dotfiles" set to the string literal "allow" (the distinctive ' +
    'serve-static / express.static option that exposes hidden files like .env or .git)',
  'aws-sqs-unencrypted-queue':
    'Flag AWS CDK SQS queues created with server-side encryption disabled; matches an ' +
    'object property "encryption" set to a member expression ending in UNENCRYPTED ' +
    '(QueueEncryption.UNENCRYPTED) or "sqsManagedSseEnabled" set to the boolean literal false',
  'aws-apigateway-public-api':
    'Flag an AWS API Gateway method created without authorization; matches an object ' +
    'property "authorizationType" whose value is AuthorizationType.NONE (any member ' +
    'expression ending in NONE) or the string literal "NONE"',
  'aws-iam-all-privileges':
    'Flag AWS IAM policy statements that grant all privileges; matches an object property ' +
    '"actions" whose array literal value contains the wildcard string "*"',
  'aws-s3-bucket-versioning':
    'Flag AWS CDK S3 bucket configuration that explicitly disables versioning; matches an ' +
    'object property "versioned" set to the boolean literal false (omission is deliberately ' +
    'not flagged to stay zero-false-positive)',
  'aws-ec2-rds-dms-public':
    'Flag AWS resources (EC2 / RDS / DMS) made publicly accessible; matches an object ' +
    'property "publiclyAccessible" or "associatePublicIpAddress" whose value is the ' +
    'boolean literal true',
  'aws-s3-bucket-public-access':
    'Flag an AWS CDK S3 BlockPublicAccess configuration that disables public-access ' +
    'protection; matches an object property "blockPublicAcls", "blockPublicPolicy", ' +
    '"ignorePublicAcls", or "restrictPublicBuckets" whose value is the boolean literal false',
  'confidential-information-logging':
    'Flag a Signale logger configured without secret masking; matches a new-expression ' +
    'whose callee is Signale and whose first argument is an object literal with a "secrets" ' +
    'property set to an empty array literal',
  'aws-iam-all-resources-accessible':
    'Flag AWS IAM policy statements that grant access to all resources; matches an object ' +
    'property "resources" whose array literal value contains the wildcard string "*"',
  'aws-ec2-unencrypted-ebs-volume':
    'Flag an AWS CDK EBS Volume created without encryption; matches a new-expression whose ' +
    'callee is the Volume construct and one of whose arguments is an object literal with an ' +
    '"encrypted" property set to the boolean literal false (the implicit/absent form is ' +
    'deliberately not flagged to stay zero-false-positive)',
  'aws-efs-unencrypted':
    'Flag an AWS CDK EFS FileSystem created without encryption at rest; matches a ' +
    'new-expression whose callee names the FileSystem construct and one of whose arguments ' +
    'is an object literal with an "encrypted" property set to the boolean literal false',
  'aws-restricted-ip-admin-access':
    'Flag an AWS CDK security group ingress rule that opens an administration port (SSH 22 ' +
    'or RDP 3389) to all IP addresses; matches an addIngressRule call whose peer argument is ' +
    'anyIpv4()/anyIpv6() and whose port argument is Port.tcp(22) or Port.tcp(3389)',
  'redundant-type-aliases':
    'Flag a TypeScript type alias whose right-hand side merely renames an existing type; ' +
    'matches an alias with no type parameters whose declared type is a primitive/built-in ' +
    'keyword type or a bare type reference with no type arguments',
  'jsx-no-leaked-render':
    'Flag a logical-AND expression that conditionally renders JSX where the left operand is ' +
    'numeric (a `.length` member access or a numeric literal), e.g. `{items.length && <List/>}`; ' +
    'a falsy numeric `0` is rendered as the text "0" instead of nothing. Boolean comparisons ' +
    '(`> 0`, `!== 0`), plain identifiers, non-`length` members, the `||` operator and non-JSX ' +
    'right operands are not flagged to stay zero-false-positive',
  'no-uniq-key':
    'Flag a JSX key attribute whose value is a Math.random() or Date.now() call; ' +
    'such a value differs on every render, so React keys never match up between renders ' +
    'and the DOM is needlessly recreated (zero-false-positive syntactic subset of S6486)',
  'insecure-cookie':
    'Flag a cookie configuration object that sets "secure" to the boolean literal false, ' +
    'which lets the cookie be sent over unencrypted HTTP; gated to stay zero-false-positive ' +
    'by requiring a distinctive cookie-marker sibling key (httpOnly, sameSite, maxAge, domain, ' +
    'path, or signed) in the same object literal',
  'no-hook-setter-in-body':
    'Flag a React useState setter that is called directly in a component body (a direct ' +
    'top-level expression statement of the same function), which schedules a state update ' +
    'on every render and causes an infinite re-render loop; function-local detection scans ' +
    'only the direct statements of one body, so calls inside handlers, effects, callbacks, ' +
    'conditionals, loops, or JSX are not flagged (zero-false-positive subset of S6442)',
  'content-length':
    'Flag an upload-size limit larger than 8MB, expressed as a fileSize/maxFileSize object ' +
    'property or a .fileSize/.maxFileSize member assignment whose value is a numeric literal ' +
    'greater than 8000000; an excessive limit enables denial-of-service attacks (zero-false-' +
    'positive subset of S5693 that skips string limits and generic keys)',
  'unverified-certificate':
    'Flag a rejectUnauthorized object property whose value is the boolean literal false, which ' +
    'disables TLS server-certificate validation in Node.js https/tls/request options and exposes ' +
    'the connection to man-in-the-middle attacks; only the distinctive rejectUnauthorized key with ' +
    'a literal false value is reported (zero-false-positive subset of S4830)',
  'no-mime-sniff':
    'Flag an object property whose key is exactly "noSniff" set to the boolean literal false, ' +
    "which disables helmet's X-Content-Type-Options: nosniff protection and exposes the app " +
    'to MIME confusion attacks; the distinctive helmet key makes this a zero-false-positive ' +
    'subset of S5734',
  'no-ip-forward':
    'Flag an http-proxy / http-proxy-middleware configuration that enables client-IP ' +
    'forwarding, expressed as an object property whose key is exactly xfwd and whose value ' +
    'is the boolean literal true; forwarding the client IP can leak personal information and ' +
    'enable IP-based access-control bypass (zero-false-positive subset of S5759 keyed on the ' +
    'distinctive xfwd option)',
  'no-angular-bypass-sanitization':
    'Flag calls to Angular DomSanitizer bypassSecurityTrust* methods (bypassSecurityTrustHtml, ' +
    'bypassSecurityTrustStyle, bypassSecurityTrustScript, bypassSecurityTrustUrl, ' +
    "bypassSecurityTrustResourceUrl), which disable Angular's built-in XSS sanitization; the " +
    'method names are essentially unique to DomSanitizer so any such call is reported (S6268)',
  'insecure-jwt-token':
    'Flag a JWT options object property that disables signature protection: a key of algorithm ' +
    "whose value is the string literal 'none', or a key of algorithms whose array value contains " +
    "the string literal 'none' (both compared case-insensitively); the 'none' JWT algorithm " +
    'disables signature verification and lets tokens be forged (zero-false-positive subset of ' +
    'S5659 keyed on the distinctive algorithm/algorithms options)',
  'xml-parser-xxe':
    'Flag an object property whose key is the boolean option noent and whose value is the ' +
    'boolean literal true; libxmljs parseXmlString enables external entity expansion when ' +
    'noent: true is set, exposing the application to XML External Entity (XXE) attacks (zero-' +
    'false-positive subset of S2755 keyed on the distinctive noent option)',
  'no-useless-react-setstate':
    'Flag a React useState setter called with its own paired state variable ' +
    '(setV(v) where const [v, setV] = useState(...)); React bails out of the ' +
    're-render because the next value is identical, so the call is a no-op. The ' +
    'state binding is const and the pair is collected from the same destructuring, ' +
    'making an exact setter(state) match zero-false-positive (S6443)',
  'no-referrer-policy':
    "Flag a helmet referrerPolicy option set to a leaky Referrer-Policy value ('no-referrer-when-downgrade' " +
    "or 'unsafe-url'), which sends the full URL to other origins and can expose confidential data; flags a " +
    'policy property whose string value is one of those distinctive tokens (zero-false-positive subset of S5736)',
  'weak-ssl':
    'Flag a TLS/SSL options object configured with a weak, deprecated protocol version: a secureProtocol ' +
    "string of 'TLSv1_method', 'TLSv1_1_method', 'SSLv2_method', 'SSLv3_method', or 'SSLv23_method', or a " +
    "minVersion/maxVersion string of 'TLSv1' or 'TLSv1.1'. Those distinctive Node.js TLS constants make this " +
    'a zero-false-positive subset of S4423; use TLS 1.2 or higher instead',
  'no-weak-keys':
    'Flag a crypto.generateKeyPair/generateKeyPairSync call that generates a weak asymmetric key: an options ' +
    'object with a modulusLength numeric literal below 2048 (RSA/DSA/DH) or a namedCurve string literal naming ' +
    'a curve below 224 bits; the distinctive generateKeyPair* callee plus a literal weak parameter keeps this a ' +
    'zero-false-positive subset of S4426',
  'strict-transport-security':
    'Flag a weak helmet hsts configuration: an hsts call whose first object argument either disables ' +
    'includeSubDomains (set to false) or sets maxAge to a numeric literal below the recommended six-month ' +
    'minimum of 15552000 seconds; the distinctive hsts method name keeps this zero-false-positive (S5739)',
  'unverified-hostname':
    'Flag a checkServerIdentity option whose value is a function literal with a trivial always-pass body ' +
    '(an empty block, a bare return / return true / return undefined, or an arrow expression body of true), ' +
    'which disables TLS hostname verification; the distinctive key plus empty override is a zero-false-positive ' +
    'subset of S5527',
  'frame-ancestors':
    "Flag a helmet contentSecurityPolicy frameAncestors directive set to the array value \"'none'\" (the documented Noncompliant pattern of S5732); the distinctive frameAncestors key plus the CSP 'none' keyword keeps this a zero-false-positive subset",
  'no-table-as-layout':
    'Flag a JSX <table> element carrying role="presentation" or role="none" (the documented Noncompliant pattern of S5257), which marks a layout table that confuses screen readers; matching this exact shape keeps it zero-false-positive',
  'no-vue-bypass-sanitization':
    'Flag the Vue.js raw-HTML render patterns of S6299: a JSX domPropsInnerHTML attribute, or a domProps object literal containing an innerHTML property; both bypass Vue built-in escaping. The distinctive domProps/domPropsInnerHTML shapes keep this zero-false-positive (v-html templates are out of scope)',
  'os-command':
    'Flag a child_process spawn/spawnSync/execFile/execFileSync call passed an options object with shell:true (the shell-interpreter form of S4721), which risks OS command injection; exec/execSync are intentionally excluded to avoid colliding with RegExp.exec, keeping this zero-false-positive',
  'argument-type':
    'Flag a single-argument Math.* numeric call (abs, floor, sqrt, ...) whose argument is a boolean-producing expression — a comparison, a logical &&/||, or a logical-not — which is the documented type-mismatch bug of S3782 (e.g. Math.abs(x < 0.0042)); requiring a boolean-producing argument to a numeric Math method keeps this zero-false-positive',
  'aws-s3-bucket-insecure-http':
    'Flag an AWS CDK S3 bucket property enforceSSL:false (the explicit insecure-HTTP form of S6249), which authorizes cleartext HTTP access. The distinctive enforceSSL key keeps this zero-false-positive; omission is intentionally not flagged',
  'aws-s3-bucket-server-encryption':
    'Flag an AWS CDK S3 bucket encryption property set to BucketEncryption.UNENCRYPTED (or the string "UNENCRYPTED") — the explicit disable-encryption form of S6245. The distinctive encryption key plus UNENCRYPTED value keeps this zero-false-positive; omission is intentionally not flagged',
});

const ruleTypes = Object.freeze({
  'array-callback-without-return': 'suggestion',
  'declarations-in-global-scope': 'suggestion',
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
  'label-position': 'suggestion',
  'no-delete-var': 'problem',
  'constructor-for-side-effects': 'problem',
  'no-empty-character-class': 'problem',
  'no-empty-group': 'suggestion',
  'no-empty-alternatives': 'suggestion',
  'no-regex-spaces': 'suggestion',
  'no-control-regex': 'suggestion',
  'single-char-in-character-classes': 'suggestion',
  'duplicates-in-character-class': 'suggestion',
  'anchor-precedence': 'suggestion',
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
  'no-invariant-returns': 'suggestion',
  'no-same-line-conditional': 'suggestion',
  'no-nested-assignment': 'suggestion',
  'no-nested-incdec': 'suggestion',
  'no-useless-increment': 'suggestion',
  'class-name': 'suggestion',
  'function-name': 'suggestion',
  'max-lines': 'suggestion',
  'max-lines-per-function': 'suggestion',
  'nested-control-flow': 'suggestion',
  'no-duplicate-string': 'suggestion',
  'cyclomatic-complexity': 'suggestion',
  'no-collection-size-mischeck': 'suggestion',
  'index-of-compare-to-positive-number': 'suggestion',
  'no-nested-functions': 'suggestion',
  'too-many-break-or-continue-in-loop': 'suggestion',
  'code-eval': 'suggestion',
  'void-use': 'suggestion',
  'prefer-promise-shorthand': 'suggestion',
  'pseudo-random': 'suggestion',
  hashing: 'suggestion',
  'no-clear-text-protocols': 'suggestion',
  'no-hardcoded-ip': 'suggestion',
  'no-global-this': 'suggestion',
  'single-character-alternation': 'suggestion',
  'empty-string-repetition': 'suggestion',
  'no-misleading-array-reverse': 'suggestion',
  'no-alphabetical-sort': 'suggestion',
  'no-for-in-iterable': 'suggestion',
  'no-associative-arrays': 'suggestion',
  'bitwise-operators': 'suggestion',
  'no-same-argument-assert': 'suggestion',
  'inverted-assertion-arguments': 'suggestion',
  'no-incomplete-assertions': 'problem',
  'for-loop-increment-sign': 'suggestion',
  'no-equals-in-for-termination': 'suggestion',
  'reduce-initial-value': 'suggestion',
  'no-parameter-reassignment': 'suggestion',
  'no-wildcard-import': 'suggestion',
  'updated-loop-counter': 'suggestion',
  'misplaced-loop-counter': 'suggestion',
  'no-array-delete': 'suggestion',
  'no-literal-call': 'problem',
  'shorthand-property-grouping': 'suggestion',
  'process-argv': 'suggestion',
  'standard-input': 'suggestion',
  'no-code-after-done': 'suggestion',
  'function-inside-loop': 'suggestion',
  'no-useless-intersection': 'suggestion',
  'use-type-alias': 'suggestion',
  'public-static-readonly': 'suggestion',
  'call-argument-line': 'suggestion',
  'prefer-object-literal': 'suggestion',
  'no-undefined-argument': 'suggestion',
  'no-identical-functions': 'suggestion',
  'no-in-misuse': 'problem',
  'no-require-or-define': 'suggestion',
  'no-invalid-regexp': 'problem',
  'no-extra-arguments': 'problem',
  'link-with-target-blank': 'problem',
  'no-weak-cipher': 'suggestion',
  'no-hardcoded-passwords': 'suggestion',
  'no-ignored-exceptions': 'problem',
  'no-unused-function-argument': 'suggestion',
  'object-alt-content': 'problem',
  'no-use-of-empty-return-value': 'problem',
  'no-duplicated-branches': 'problem',
  'block-scoped-var': 'suggestion',
  'no-variable-usage-before-declaration': 'problem',
  'arguments-order': 'problem',
  'updated-const-var': 'problem',
  'unicode-aware-regex': 'problem',
  'no-undefined-assignment': 'suggestion',
  'no-empty-after-reluctant': 'problem',
  'no-ignored-return': 'problem',
  'file-name-differ-from-class': 'suggestion',
  'no-unenclosed-multiline-block': 'suggestion',
  'inconsistent-function-call': 'problem',
  'new-operator-misuse': 'problem',
  'no-empty-test-file': 'problem',
  deprecation: 'suggestion',
  'cognitive-complexity': 'suggestion',
  'expression-complexity': 'suggestion',
  'prefer-regexp-exec': 'suggestion',
  'no-fallthrough': 'problem',
  'no-commented-code': 'suggestion',
  'destructuring-assignment-syntax': 'suggestion',
  'no-element-overwrite': 'problem',
  'no-redundant-assignments': 'suggestion',
  'no-unused-collection': 'problem',
  'no-empty-collection': 'problem',
  'no-redundant-parentheses': 'suggestion',
  'bool-param-default': 'suggestion',
  'post-message': 'problem',
  'in-operator-type-error': 'problem',
  'different-types-comparison': 'problem',
  'operation-returning-nan': 'problem',
  'production-debug': 'problem',
  'no-hardcoded-secrets': 'problem',
  'concise-regex': 'suggestion',
  'no-misleading-character-class': 'problem',
  'slow-regex': 'problem',
  'web-sql-database': 'problem',
  'no-intrusive-permissions': 'problem',
  'encryption-secure-mode': 'problem',
  'no-unsafe-unzip': 'problem',
  'disabled-timeout': 'problem',
  'cookie-no-httponly': 'problem',
  'content-security-policy': 'problem',
  'certificate-transparency': 'problem',
  csrf: 'problem',
  'file-permissions': 'problem',
  'file-uploads': 'problem',
  cors: 'problem',
  'dns-prefetching': 'problem',
  'disabled-auto-escaping': 'problem',
  'aws-s3-bucket-granted-access': 'problem',
  'aws-rds-unencrypted-databases': 'problem',
  'aws-iam-public-access': 'problem',
  'hidden-files': 'problem',
  'aws-sqs-unencrypted-queue': 'problem',
  'aws-apigateway-public-api': 'problem',
  'aws-iam-all-privileges': 'problem',
  'aws-s3-bucket-versioning': 'problem',
  'aws-ec2-rds-dms-public': 'problem',
  'aws-s3-bucket-public-access': 'problem',
  'confidential-information-logging': 'problem',
  'aws-iam-all-resources-accessible': 'problem',
  'aws-ec2-unencrypted-ebs-volume': 'problem',
  'aws-efs-unencrypted': 'problem',
  'aws-restricted-ip-admin-access': 'problem',
  'redundant-type-aliases': 'suggestion',
  'jsx-no-leaked-render': 'problem',
  'no-uniq-key': 'problem',
  'insecure-cookie': 'problem',
  'no-hook-setter-in-body': 'problem',
  'content-length': 'problem',
  'unverified-certificate': 'problem',
  'no-mime-sniff': 'problem',
  'no-ip-forward': 'problem',
  'no-angular-bypass-sanitization': 'problem',
  'insecure-jwt-token': 'problem',
  'xml-parser-xxe': 'problem',
  'no-useless-react-setstate': 'problem',
  'no-referrer-policy': 'problem',
  'weak-ssl': 'problem',
  'no-weak-keys': 'problem',
  'strict-transport-security': 'problem',
  'unverified-hostname': 'problem',
  'frame-ancestors': 'problem',
  'no-table-as-layout': 'suggestion',
  'no-vue-bypass-sanitization': 'problem',
  'os-command': 'problem',
  'argument-type': 'problem',
  'aws-s3-bucket-insecure-http': 'problem',
  'aws-s3-bucket-server-encryption': 'problem',
});

const recommendedRuleConfig = Object.freeze({
  'array-callback-without-return': 'error',
  'declarations-in-global-scope': 'error',
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
  'label-position': 'error',
  'no-delete-var': 'error',
  'constructor-for-side-effects': 'error',
  'no-empty-character-class': 'error',
  'no-empty-group': 'error',
  'no-empty-alternatives': 'error',
  'no-regex-spaces': 'error',
  'no-control-regex': 'error',
  'single-char-in-character-classes': 'error',
  'duplicates-in-character-class': 'error',
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
  'no-invariant-returns': 'error',
  'no-same-line-conditional': 'error',
  'no-nested-assignment': 'error',
  'no-nested-incdec': 'error',
  'no-useless-increment': 'error',
  'class-name': 'error',
  'max-lines': 'error',
  'max-lines-per-function': 'error',
  'nested-control-flow': 'error',
  'no-duplicate-string': 'error',
  'anchor-precedence': 'error',
  'cyclomatic-complexity': 'error',
  'no-collection-size-mischeck': 'error',
  'index-of-compare-to-positive-number': 'error',
  'no-nested-functions': 'error',
  'too-many-break-or-continue-in-loop': 'error',
  'code-eval': 'error',
  'void-use': 'error',
  'prefer-promise-shorthand': 'error',
  'pseudo-random': 'error',
  hashing: 'error',
  'no-clear-text-protocols': 'error',
  'no-hardcoded-ip': 'error',
  'no-global-this': 'error',
  'single-character-alternation': 'error',
  'empty-string-repetition': 'error',
  'no-misleading-array-reverse': 'error',
  'no-alphabetical-sort': 'error',
  'no-for-in-iterable': 'error',
  'no-associative-arrays': 'error',
  'bitwise-operators': 'error',
  'no-same-argument-assert': 'error',
  'inverted-assertion-arguments': 'error',
  'no-incomplete-assertions': 'error',
  'for-loop-increment-sign': 'error',
  'no-equals-in-for-termination': 'error',
  'reduce-initial-value': 'error',
  'no-parameter-reassignment': 'error',
  'no-wildcard-import': 'error',
  'updated-loop-counter': 'error',
  'misplaced-loop-counter': 'error',
  'no-array-delete': 'error',
  'no-literal-call': 'error',
  'shorthand-property-grouping': 'error',
  'process-argv': 'error',
  'standard-input': 'error',
  'no-code-after-done': 'error',
  'function-inside-loop': 'error',
  'no-useless-intersection': 'error',
  'use-type-alias': 'error',
  'public-static-readonly': 'error',
  'call-argument-line': 'error',
  'prefer-object-literal': 'error',
  'no-undefined-argument': 'error',
  'no-identical-functions': 'error',
  'no-in-misuse': 'error',
  'no-require-or-define': 'error',
  'no-invalid-regexp': 'error',
  'no-extra-arguments': 'error',
  'link-with-target-blank': 'error',
  'no-weak-cipher': 'error',
  'no-hardcoded-passwords': 'error',
  'no-ignored-exceptions': 'error',
  'no-unused-function-argument': 'error',
  'object-alt-content': 'error',
  'no-use-of-empty-return-value': 'error',
  'no-duplicated-branches': 'error',
  'block-scoped-var': 'error',
  'no-variable-usage-before-declaration': 'error',
  'arguments-order': 'error',
  'updated-const-var': 'error',
  'unicode-aware-regex': 'error',
  'no-undefined-assignment': 'error',
  'no-empty-after-reluctant': 'error',
  'no-ignored-return': 'error',
  'file-name-differ-from-class': 'error',
  'no-unenclosed-multiline-block': 'error',
  'inconsistent-function-call': 'error',
  'new-operator-misuse': 'error',
  'no-empty-test-file': 'error',
  deprecation: 'error',
  'cognitive-complexity': 'error',
  'expression-complexity': 'error',
  'prefer-regexp-exec': 'error',
  'no-fallthrough': 'error',
  'no-commented-code': 'error',
  'destructuring-assignment-syntax': 'error',
  'no-element-overwrite': 'error',
  'no-redundant-assignments': 'error',
  'no-unused-collection': 'error',
  'no-empty-collection': 'error',
  'no-redundant-parentheses': 'error',
  'bool-param-default': 'error',
  'post-message': 'error',
  'in-operator-type-error': 'error',
  'different-types-comparison': 'error',
  'operation-returning-nan': 'error',
  'production-debug': 'error',
  'no-hardcoded-secrets': 'error',
  'concise-regex': 'error',
  'no-misleading-character-class': 'error',
  'slow-regex': 'error',
  'web-sql-database': 'error',
  'no-intrusive-permissions': 'error',
  'encryption-secure-mode': 'error',
  'no-unsafe-unzip': 'error',
  'disabled-timeout': 'error',
  'cookie-no-httponly': 'error',
  'content-security-policy': 'error',
  'certificate-transparency': 'error',
  csrf: 'error',
  'file-permissions': 'error',
  'file-uploads': 'error',
  cors: 'error',
  'dns-prefetching': 'error',
  'disabled-auto-escaping': 'error',
  'aws-s3-bucket-granted-access': 'error',
  'aws-rds-unencrypted-databases': 'error',
  'aws-iam-public-access': 'error',
  'hidden-files': 'error',
  'aws-sqs-unencrypted-queue': 'error',
  'aws-apigateway-public-api': 'error',
  'aws-iam-all-privileges': 'error',
  'aws-s3-bucket-versioning': 'error',
  'aws-ec2-rds-dms-public': 'error',
  'aws-s3-bucket-public-access': 'error',
  'confidential-information-logging': 'error',
  'aws-iam-all-resources-accessible': 'error',
  'aws-ec2-unencrypted-ebs-volume': 'error',
  'aws-efs-unencrypted': 'error',
  'aws-restricted-ip-admin-access': 'error',
  'redundant-type-aliases': 'error',
  'jsx-no-leaked-render': 'error',
  'no-uniq-key': 'error',
  'insecure-cookie': 'error',
  'no-hook-setter-in-body': 'error',
  'content-length': 'error',
  'unverified-certificate': 'error',
  'no-mime-sniff': 'error',
  'no-ip-forward': 'error',
  'no-angular-bypass-sanitization': 'error',
  'insecure-jwt-token': 'error',
  'xml-parser-xxe': 'error',
  'no-useless-react-setstate': 'error',
  'no-referrer-policy': 'error',
  'weak-ssl': 'error',
  'no-weak-keys': 'error',
  'strict-transport-security': 'error',
  'unverified-hostname': 'error',
  'frame-ancestors': 'error',
  'no-table-as-layout': 'error',
  'no-vue-bypass-sanitization': 'error',
  'os-command': 'error',
  'argument-type': 'error',
  'aws-s3-bucket-insecure-http': 'error',
  'aws-s3-bucket-server-encryption': 'error',
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
  if (ruleName === 'no-duplicate-string') {
    return [
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'cyclomatic-complexity') {
    return [
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'no-nested-functions') {
    return [
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'function-name') {
    return [
      {
        type: 'object',
        properties: { format: { type: 'string' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'cognitive-complexity') {
    return [
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'expression-complexity') {
    return [
      {
        type: 'object',
        properties: { threshold: { type: 'integer' } },
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
  if (ruleName === 'no-duplicate-string' && Number.isInteger(raw.threshold)) {
    options.noDuplicateStringThreshold = raw.threshold;
  }
  if (ruleName === 'cyclomatic-complexity' && Number.isInteger(raw.threshold)) {
    options.cyclomaticComplexityThreshold = raw.threshold;
  }
  if (ruleName === 'no-nested-functions' && Number.isInteger(raw.threshold)) {
    options.noNestedFunctionsThreshold = raw.threshold;
  }
  if (ruleName === 'function-name' && typeof raw.format === 'string') {
    options.functionNameFormat = raw.format;
  }
  if (ruleName === 'cognitive-complexity' && Number.isInteger(raw.threshold)) {
    options.cognitiveComplexityThreshold = raw.threshold;
  }
  if (ruleName === 'expression-complexity' && Number.isInteger(raw.threshold)) {
    options.expressionComplexityThreshold = raw.threshold;
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
