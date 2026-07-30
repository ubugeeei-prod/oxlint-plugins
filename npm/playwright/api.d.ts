export type PlaywrightDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type PlaywrightDiagnostic = {
  ruleName: string;
  messageId: string;
  data: {
    message: string;
    amount?: string;
    count?: string;
    depth?: string;
    max?: string;
    method?: string;
    restriction?: string;
    role?: string;
    functionName?: string;
    pattern?: string;
    tag?: string;
    word?: string;
    s?: string;
  };
  loc: PlaywrightDiagnosticLoc;
  fix?: {
    start: number;
    end: number;
    replacement: string;
  };
};

export type PlaywrightRestrictedLocator =
  | string
  | {
      type: string;
      message?: string;
    };

export type PlaywrightRestrictedRole =
  | string
  | {
      role: string;
      message?: string;
    };

export type PlaywrightScanOptions = {
  noRestrictedLocators?: PlaywrightRestrictedLocator[];
  noRestrictedMatchers?: Record<string, string | null>;
  noRestrictedRoles?: PlaywrightRestrictedRole[];
  expectAliases?: string[];
  testAliases?: string[];
  validTitle?: PlaywrightValidTitleOptions;
  validTestTags?: PlaywrightValidTestTagsOptions;
  maxExpects?: number;
  maxNestedDescribe?: number;
  maxTopLevelDescribes?: number;
};

export type PlaywrightTitlePattern =
  | string
  | [pattern: string]
  | [pattern: string, message: string];

export type PlaywrightValidTitleOptions = {
  disallowedWords?: string[];
  ignoreSpaces?: boolean;
  ignoreTypeOfDescribeName?: boolean;
  ignoreTypeOfStepName?: boolean;
  ignoreTypeOfTestName?: boolean;
  mustMatch?:
    | PlaywrightTitlePattern
    | Partial<Record<'describe' | 'step' | 'test', PlaywrightTitlePattern>>;
  mustNotMatch?:
    | PlaywrightTitlePattern
    | Partial<Record<'describe' | 'step' | 'test', PlaywrightTitlePattern>>;
};

export type PlaywrightTagPattern = string | RegExp | { source: string; flags?: string };

export type PlaywrightValidTestTagsOptions = {
  allowedTags?: PlaywrightTagPattern[];
  disallowedTags?: PlaywrightTagPattern[];
};

export function implementedPlaywrightRuleNames(): string[];
export function scanPlaywright(
  sourceText: string,
  filename?: string,
  options?: PlaywrightScanOptions,
): PlaywrightDiagnostic[];
