export type ForbiddenIdentifierOptions = {
  names?: string[];
};

export declare function scanForbiddenIdentifiers(
  sourceText: string,
  options?: ForbiddenIdentifierOptions,
): string[];

export declare function isForbiddenIdentifierName(
  name: string,
  options?: ForbiddenIdentifierOptions,
): boolean;
