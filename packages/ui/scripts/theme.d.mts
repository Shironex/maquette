export declare const EXTRA_COLORS: { name: string; value: string; usage: string }[];
export declare function buildTheme(tokens: unknown): string;
export declare function themeNames(tokens: unknown): {
  colors: string[];
  text: string[];
  radius: string[];
  shadow: string[];
};
