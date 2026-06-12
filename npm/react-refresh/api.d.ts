export declare function defaultHocs(): string[];
export declare function isReactComponentName(name: string): boolean;
export declare function shouldScanFilename(filename: string, checkJS?: boolean): boolean;
export declare function isConstantExportExpressionKind(kind: string): boolean;
export type OnlyExportComponentsOptions = {
  extraHOCs?: string[];
  allowExportNames?: string[];
  allowConstantExport?: boolean;
  checkJS?: boolean;
};
export type ReactRefreshDiagnostic = {
  messageId: string;
  loc: {
    startLine: number;
    startColumn: number;
    endLine: number;
    endColumn: number;
  };
};
export declare function scanOnlyExportComponents(
  sourceText: string,
  filename: string,
  options?: OnlyExportComponentsOptions,
): ReactRefreshDiagnostic[];
