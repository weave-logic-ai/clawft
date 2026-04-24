import eslint from "@eslint/js";
import eslintConfigPrettier from "eslint-config-prettier/flat";
import perfectionist from "eslint-plugin-perfectionist";
import { defineConfig } from "eslint/config";
import tseslint from "typescript-eslint";

export default defineConfig(
  eslint.configs.recommended,
  perfectionist.configs["recommended-alphabetical"],
  eslintConfigPrettier,
  {
    extends: [
      tseslint.configs.recommended,
    ],
    rules: {
      "@typescript-eslint/naming-convention": [
        "error",
        {
          format: ["camelCase"],
          selector: "property",
        },
        {
          format: null,
          modifiers: ["requiresQuotes"],
          selector: "property",
        },
      ],
    },
  },
  {
    ignores: [
      "**/*.js",
      "**/*.d.ts",
      "dist/**",
      "node_modules/**",
      "pnpm-lock.yaml",
    ],
  },
);