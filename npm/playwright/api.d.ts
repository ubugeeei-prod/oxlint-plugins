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
    method?: string;
    restriction?: string;
    role?: string;
  };
  loc: PlaywrightDiagnosticLoc;
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
};

export function implementedPlaywrightRuleNames(): string[];
export function scanPlaywright(
  sourceText: string,
  filename?: string,
  options?: PlaywrightScanOptions,
): PlaywrightDiagnostic[];
