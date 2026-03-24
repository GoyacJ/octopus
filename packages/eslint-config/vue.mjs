import pluginVue from 'eslint-plugin-vue'
import tseslint from 'typescript-eslint'
import base from './base.mjs'

export default [
  ...base,
  ...pluginVue.configs['flat/recommended'],
  {
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.vue'],
      },
    },
    rules: {
      'no-undef': 'off',
    },
  },
]
