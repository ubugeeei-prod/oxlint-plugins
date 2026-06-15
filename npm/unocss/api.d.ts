export type UnocssDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type UnocssDiagnosticFix = {
  start: number;
  end: number;
  replacement: string;
};

export type UnocssDiagnostic = {
  ruleName: string;
  messageId: string;
  loc: UnocssDiagnosticLoc;
  fix?: UnocssDiagnosticFix | null;
  /** Blocklisted utility name (set for `blocklist` diagnostics). */
  name?: string | null;
  /**
   * Blocklist reason, echoed verbatim from the matched `blocklist` option
   * entry's `reason` (empty string when none). The bundled plugin supplies it
   * pre-formatted with a leading `": "` so it splices into the default
   * `"{{name}}" is in blocklist{{reason}}` template; a direct API caller gets
   * back exactly the string it passed.
   */
  reason?: string | null;
  prefix?: string | null;
};

export type UnocssBlocklistEntry = {
  name: string;
  reason?: string;
};

export type UnocssScanOptions = {
  unoFunctions?: string[];
  unoVariables?: string[];
  blocklist?: Array<string | UnocssBlocklistEntry | [string, { message?: string }]>;
  classCompilePrefix?: string;
  classCompileEnableFix?: boolean;
};

export function implementedUnocssRuleNames(): string[];
export function scanUnocss(
  sourceText: string,
  filename?: string,
  options?: UnocssScanOptions,
): UnocssDiagnostic[];
