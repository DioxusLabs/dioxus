import { defineConfig, globalIgnores } from 'eslint/config';
import js from '@eslint/js';
import ts from 'typescript-eslint';

export default defineConfig([
  js.configs.recommended,
  ...ts.configs.strictTypeChecked,
  globalIgnores(['dist/*', 'pkg/*']),
  {
    languageOptions: {
      parserOptions: {
        project: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
]);
