import js from "@eslint/js";
import nounsanitized from "eslint-plugin-no-unsanitized";
import globals from "globals";

const authoredFiles = [
  "eslint.config.mjs",
  "playwright.config.js",
  "scripts/**/*.mjs",
  "tests/frontend/**/*.mjs",
  "web/**/*.{js,jsx}",
];

export default [
  {
    ignores: [
      "node_modules/**",
      "playwright-report/**",
      "target/**",
      "test-results/**",
      "web/dist/**",
      "web/runtime-dist/**",
    ],
  },
  {
    ...js.configs.recommended,
    files: authoredFiles,
    languageOptions: {
      ecmaVersion: "latest",
      parserOptions: { ecmaFeatures: { jsx: true } },
      globals: {
        ...globals.browser,
        ...globals.node,
      },
      sourceType: "module",
    },
    linterOptions: {
      reportUnusedDisableDirectives: "error",
    },
    rules: {
      ...js.configs.recommended.rules,
      "eol-last": ["error", "always"],
      "no-alert": "error",
      "no-eval": "error",
      "no-implied-eval": "error",
      "no-new-func": "error",
      "no-script-url": "error",
      "no-tabs": "error",
      "no-trailing-spaces": "error",
      "unicode-bom": ["error", "never"],
    },
  },
  {
    files: ["web/**/*.{js,jsx}"],
    ignores: ["web/dist/**", "web/runtime-dist/**"],
    plugins: { nounsanitized },
    rules: {
      "no-restricted-globals": [
        "error",
        { name: "fetch", message: "Use modules/http/client.js." },
        { name: "XMLHttpRequest", message: "Use modules/upload/transport.js." },
      ],
      "no-restricted-syntax": [
        "error",
        {
          selector: "Property[key.name='dangerouslySetInnerHTML']",
          message: "React HTML injection is not allowed.",
        },
        {
          selector: "NewExpression[callee.name='DOMParser']",
          message: "DOMParser is not allowed in authored browser code.",
        },
        {
          selector: "CallExpression[callee.property.name='createContextualFragment']",
          message: "HTML fragment parsing is not allowed.",
        },
        {
          selector: "MemberExpression[property.name='setHTMLUnsafe']",
          message: "Unsafe HTML parsing is not allowed.",
        },
      ],
      "nounsanitized/method": "error",
      "nounsanitized/property": "error",
    },
  },
  {
    files: ["web/modules/http/client.js"],
    rules: {
      "no-restricted-globals": [
        "error",
        { name: "XMLHttpRequest", message: "Use modules/upload/transport.js." },
      ],
    },
  },
  {
    files: ["web/modules/upload/transport.js"],
    rules: {
      "no-restricted-globals": [
        "error",
        { name: "fetch", message: "Use modules/http/client.js." },
      ],
    },
  },
];
