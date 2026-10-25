/**
 * Conventional Commits, always scoped. The scopes mirror the folders: one per
 * app, package and crate, plus the repo-level ones.
 */
export default {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'header-max-length': [2, 'always', 100],
    'subject-case': [2, 'never', ['sentence-case', 'pascal-case', 'upper-case', 'start-case']],
    'scope-empty': [2, 'never'],
    'scope-enum': [
      2,
      'always',
      ['desktop', 'web', 'site', 'ui', 'core', 'workspace', 'deps', 'docs', 'scripts'],
    ],
  },
};
