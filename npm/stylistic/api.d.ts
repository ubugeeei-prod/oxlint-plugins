export type NativeLintRange = {
  start: number;
  end: number;
};

export type NativeLintFix = {
  range: NativeLintRange;
  replacementText: string;
};

export type NativeLintSuggestion = {
  messageId: string;
  message: string;
  fixes: NativeLintFix[];
};

export type NativeLintDiagnostic = {
  ruleName: string;
  messageId: string;
  message: string;
  range: NativeLintRange;
  suggestions?: NativeLintSuggestion[];
};

export type NativeNodeMetadataDepth = {
  minDepth: number;
  maxDepth: number;
};

export type NativeRuleBridgeRequirements = {
  maxDepth: number;
  typeTexts?: NativeNodeMetadataDepth;
  propertyNames?: NativeNodeMetadataDepth;
  text?: NativeNodeMetadataDepth;
};

export type NativeLintRuleMeta = {
  name: string;
  docsDescription: string;
  messages: Record<string, string>;
  hasSuggestions: boolean;
  listeners: string[];
  requiresTypeTexts: boolean;
  bridge: NativeRuleBridgeRequirements;
};

export type NativeStylisticRuleConfig = {
  name: string;
  options?: unknown;
};

export type NativeStylisticRunConfig = {
  rules: NativeStylisticRuleConfig[];
};

export declare function runNativeStylisticLint(
  sourceText: string,
  config: NativeStylisticRunConfig,
): NativeLintDiagnostic[];

export declare function nativeStylisticRuleMetas(): NativeLintRuleMeta[];
