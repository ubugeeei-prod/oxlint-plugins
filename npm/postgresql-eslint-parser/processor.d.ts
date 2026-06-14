export interface PlProcessorOptions {
  /**
   * Maps a lower-cased `LANGUAGE` clause to a virtual-file extension (which MUST
   * start with `.`, e.g. `".js"`, `".py"`, `".plpgsql"`).
   */
  languages: Record<string, string>;
  /**
   * What to do with a body whose language is not in `languages`. `"skip"`
   * (default) drops it; `"error"` throws.
   */
  unknown?: 'skip' | 'error';
}

export interface FixDescriptor {
  range: [number, number];
  text: string;
}

/** A minimal subset of ESLint's `Linter.LintMessage` shape. */
export interface ProcessorMessage {
  ruleId?: string | null;
  severity?: number;
  message?: string;
  line?: number;
  column?: number;
  endLine?: number;
  endColumn?: number;
  fix?: FixDescriptor;
  [key: string]: unknown;
}

export interface PlProcessor {
  meta: { name: string; version: string };
  supportsAutofix: boolean;
  preprocess: (text: string, filename: string) => Array<{ text: string; filename: string }>;
  postprocess: (messageLists: ProcessorMessage[][], filename: string) => ProcessorMessage[];
}

/** Create an ESLint processor that lints embedded PL function bodies. */
export declare function createPlProcessor(options: PlProcessorOptions): PlProcessor;
