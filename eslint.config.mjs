// @ts-check

import eslint from '@eslint/js';
import { defineConfig } from 'eslint/config';
import tseslint from 'typescript-eslint';
import eslintConfigPrettier from 'eslint-config-prettier/flat';
import globals from 'globals';
import react from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';

export default defineConfig(
  eslint.configs.recommended,
  tseslint.configs.recommended,
  eslintConfigPrettier,
  {
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: {
        ...globals.browser,
        ...globals.node,
        ...globals.es2022,
      },
      parserOptions: {
        ecmaFeatures: { jsx: true },
      },
    },
    rules: {
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-empty-object-type': 'off',
      'no-irregular-whitespace': 'error',
    },
  },
  {
    files: ['apps/web/**/*.{ts,tsx}', 'apps/site/**/*.{ts,tsx}', 'packages/ui/**/*.{ts,tsx}'],
    plugins: {
      react,
      'react-hooks': reactHooks,
    },
    settings: {
      // Pinned, not 'detect': eslint-plugin-react@7's 'detect' path calls an API
      // ESLint 10 removed. An explicit semver skips detection.
      react: { version: '19.2' },
    },
    rules: {
      ...react.configs.flat.recommended.rules,
      ...react.configs.flat['jsx-runtime'].rules,
      'react/react-in-jsx-scope': 'off',
      'react/prop-types': 'off',
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'error',
    },
  },
  {
    // react-three-fiber elements take three.js props the DOM rule does not know.
    files: ['apps/web/src/features/viewer/**/*.tsx'],
    rules: {
      'react/no-unknown-property': [
        'error',
        { ignore: ['object', 'attach', 'args', 'position', 'intensity'] },
      ],
    },
  },
  {
    // The site's workbench lights take three.js props as well.
    files: ['apps/site/src/scene/**/*.tsx'],
    rules: {
      'react/no-unknown-property': [
        'error',
        {
          ignore: [
            'object',
            'attach',
            'args',
            'position',
            'intensity',
            'castShadow',
            'distance',
            'decay',
            'angle',
            'penumbra',
          ],
        },
      ],
    },
  },
  {
    ignores: [
      '**/node_modules/**',
      '**/dist/**',
      '**/coverage/**',
      '**/target/**',
      '**/generated/**',
      'apps/desktop/src-tauri/gen/**',
      'packages/ui/src/icons/paths.ts',
      'apps/web/public/basis/**',
      'apps/site/.astro/**',
    ],
  }
);
