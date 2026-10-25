// @ts-check

import eslint from '@eslint/js';
import { defineConfig } from 'eslint/config';
import tseslint from 'typescript-eslint';
import eslintConfigPrettier from 'eslint-config-prettier/flat';
import globals from 'globals';
import react from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import architecture from '@noctcore/eslint-plugin-architecture';
import asyncSafety from '@noctcore/eslint-plugin-async-safety';
import codeQuality from '@noctcore/eslint-plugin-code-quality';
import contracts from '@noctcore/eslint-plugin-contracts';
import monorepo from '@noctcore/eslint-plugin-monorepo';
import observability from '@noctcore/eslint-plugin-observability';
import noctcoreReact from '@noctcore/eslint-plugin-react';
import security from '@noctcore/eslint-plugin-security';

// The TypeScript the house presets cover: the React app and the UI kit.
const APP_TS = ['apps/web/**/*.{ts,tsx}', 'packages/ui/**/*.{ts,tsx}'];
const typescript = { files: APP_TS, languageOptions: { parser: tseslint.parser } };

// Features every other feature may import: the shared dialogs and chrome, the
// workspace config fields and the 3D studio.
const SHARED_FEATURES = ['common', 'config', 'viewer'];

// Composition roots assemble sibling features by design: the shell renders
// every view and dialog, onboarding and Settings stack other features' fields.
// They keep every rule except no-cross-feature-imports, which exists to keep
// the leaf features decoupled.
const COMPOSITION_ROOTS = ['shell', 'onboarding', 'settings'];

// Adoption ledger. Areas listed here are held to the full rule set; the rest
// keep the rules in DEFERRED_RULES off until they are migrated. One entry per
// migration commit; this list, DEFERRED_RULES and the block that uses them are
// deleted once every area is in.
const MIGRATED = [
  'apps/web/src/state/**',
  'apps/web/src/ipc/**',
  'apps/web/src/lib/**',
  'apps/web/src/test/**',
  'apps/web/src/features/board/**',
];

// Preset rules the unmigrated areas do not pass yet, with the count measured
// when the presets went in (2026-10-25).
const DEFERRED_RULES = {
  'noctcore-react/no-state-in-component-body': 'off', // 446
  'noctcore-react/no-jsx-computation': 'off', // 84
  'noctcore-architecture/no-cross-feature-imports': 'off', // 63
  'noctcore-code-quality/no-bare-date-now': 'off', // 57
  'noctcore-code-quality/prefer-early-return': 'off', // 18
  'noctcore-architecture/filename-matches-export': 'off', // 8
  'noctcore-contracts/restrict-throw-to-taxonomy': 'off', // 6
  'noctcore-react/max-props-per-component': 'off', // 5
  'noctcore-react/component-props-naming': 'off', // 4
  'noctcore-react/no-prop-drilling': 'off', // 1
  'noctcore-code-quality/no-vacuous-expect': 'off', // 1
};

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
    files: [...APP_TS, 'apps/site/**/*.{ts,tsx}'],
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

  // The house presets, every rule at error. Each is spread whole so a new
  // preset rule arrives with the version bump instead of being missed.
  { ...codeQuality.configs.recommended, ...typescript },
  { ...asyncSafety.configs.recommended, ...typescript },
  { ...contracts.configs.recommended, ...typescript },
  { ...security.configs.recommended, ...typescript },
  { ...observability.configs.recommended, ...typescript },
  { ...noctcoreReact.configs.recommended, ...typescript },
  { ...architecture.configs.recommended, ...typescript },
  { ...monorepo.configs.recommended, ...typescript },
  {
    files: APP_TS,
    rules: {
      // Workspace packages are used through what their exports map publishes.
      'noctcore-monorepo/no-unexported-subpath-import': ['error', { scopes: ['@maquette'] }],
      'noctcore-monorepo/no-deep-package-imports': ['error', { scopes: ['@maquette'] }],
      // A component's hooks and types files are named after the component they
      // serve, not after the one hook or type they export.
      'noctcore-architecture/filename-matches-export': [
        'error',
        { ignore: ['**/*.hooks.ts', '**/*.types.ts'] },
      ],
    },
  },
  {
    // apps/web is organised by feature under src/features.
    files: ['apps/web/**/*.{ts,tsx}'],
    rules: {
      'noctcore-architecture/component-folder-structure': [
        'error',
        { componentRoot: 'features', requiredSiblings: ['.test.tsx', 'index.ts'] },
      ],
      'noctcore-architecture/no-cross-feature-imports': [
        'error',
        { featureRoot: 'features', alias: '@/features', sharedFeatures: SHARED_FEATURES },
      ],
      'noctcore-architecture/max-import-depth': ['error', { alias: { src: '@' } }],
    },
  },
  {
    files: COMPOSITION_ROOTS.map(f => `apps/web/src/features/${f}/**/*.{ts,tsx}`),
    rules: {
      'noctcore-architecture/no-cross-feature-imports': 'off',
    },
  },
  {
    // The UI kit groups primitives by kind (shell, status, inputs); a group is
    // a shelf, not a feature, and primitives are built from each other.
    files: ['packages/ui/**/*.{ts,tsx}'],
    rules: {
      'noctcore-architecture/no-cross-feature-imports': 'off',
    },
  },
  {
    // The mock backend rejects with the { code, message } payload the Tauri bridge
    // sends (errorMessage reads it through isErrorPayload), not with an Error.
    files: [
      'apps/web/src/ipc/mock/agents.ts',
      'apps/web/src/ipc/mock/layout.ts',
      'apps/web/src/ipc/mock/runner.ts',
      'apps/web/src/state/quit.test.ts',
    ],
    rules: { 'noctcore-contracts/restrict-throw-to-taxonomy': 'off' },
  },
  {
    files: APP_TS,
    ignores: MIGRATED,
    rules: DEFERRED_RULES,
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
