export type StorybookRuleName =
  | 'await-interactions'
  | 'context-in-play-function'
  | 'csf-component'
  | 'default-exports'
  | 'hierarchy-separator'
  | 'meta-inline-properties'
  | 'meta-satisfies-type'
  | 'no-redundant-story-name'
  | 'no-renderer-packages'
  | 'no-stories-of'
  | 'no-title-property-in-meta'
  | 'no-uninstalled-addons'
  | 'prefer-pascal-case'
  | 'story-exports'
  | 'use-storybook-expect'
  | 'use-storybook-testing-library';

export type DiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type DiagnosticFix = {
  start: number;
  end: number;
  replacement: string;
};

export type DiagnosticData = {
  method?: string;
  metaTitle?: string;
  property?: string;
  rendererPackage?: string;
  suggestions?: string;
  library?: string;
  addonName?: string;
  packageJsonPath?: string;
  name?: string;
};

export type Diagnostic = {
  ruleName: StorybookRuleName;
  messageId: string;
  data: DiagnosticData;
  loc: DiagnosticLoc;
  fixes?: DiagnosticFix[];
};

export type ScanStorybookOptions = {
  ruleNames?: readonly string[];
  installedAddons?: readonly string[];
  ignoredAddons?: readonly string[];
  packageJsonPath?: string;
};

export function implementedStorybookRuleNames(): StorybookRuleName[];

export function scanStorybook(
  sourceText: string,
  filename?: string,
  options?: ScanStorybookOptions,
): Diagnostic[];

declare const api: {
  implementedStorybookRuleNames: typeof implementedStorybookRuleNames;
  scanStorybook: typeof scanStorybook;
};

export default api;
