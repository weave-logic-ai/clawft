#!/usr/bin/env node
"use strict"; function _interopRequireWildcard(obj) { if (obj && obj.__esModule) { return obj; } else { var newObj = {}; if (obj != null) { for (var key in obj) { if (Object.prototype.hasOwnProperty.call(obj, key)) { newObj[key] = obj[key]; } } } newObj.default = obj; return newObj; } } function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }
// src/bin/fastmcp.ts
var _execa = require('execa');
var _yargs = require('yargs'); var _yargs2 = _interopRequireDefault(_yargs);
var _helpers = require('yargs/helpers');
await _yargs2.default.call(void 0, _helpers.hideBin.call(void 0, process.argv)).scriptName("fastmcp").command(
  "dev <file>",
  "Start a development server",
  (yargs2) => {
    return yargs2.positional("file", {
      demandOption: true,
      describe: "The path to the server file",
      type: "string"
    }).option("watch", {
      alias: "w",
      default: false,
      describe: "Watch for file changes and restart server",
      type: "boolean"
    }).option("verbose", {
      alias: "v",
      default: false,
      describe: "Enable verbose logging",
      type: "boolean"
    });
  },
  async (argv) => {
    try {
      const command = argv.watch ? `npx @wong2/mcp-cli npx tsx --watch ${argv.file}` : `npx @wong2/mcp-cli npx tsx ${argv.file}`;
      if (argv.verbose) {
        console.log(`[FastMCP] Starting server: ${command}`);
        console.log(`[FastMCP] File: ${argv.file}`);
        console.log(
          `[FastMCP] Watch mode: ${argv.watch ? "enabled" : "disabled"}`
        );
      }
      await _execa.execa.call(void 0, {
        shell: true,
        stderr: "inherit",
        stdin: "inherit",
        stdout: "inherit"
      })`${command}`;
    } catch (error) {
      console.error(
        "[FastMCP Error] Failed to start development server:",
        error instanceof Error ? error.message : String(error)
      );
      if (argv.verbose && error instanceof Error && error.stack) {
        console.error("[FastMCP Debug] Stack trace:", error.stack);
      }
      process.exit(1);
    }
  }
).command(
  "inspect <file>",
  "Inspect a server file",
  (yargs2) => {
    return yargs2.positional("file", {
      demandOption: true,
      describe: "The path to the server file",
      type: "string"
    });
  },
  async (argv) => {
    try {
      await _execa.execa.call(void 0, {
        stderr: "inherit",
        stdout: "inherit"
      })`npx @modelcontextprotocol/inspector npx tsx ${argv.file}`;
    } catch (error) {
      console.error(
        "[FastMCP Error] Failed to inspect server:",
        error instanceof Error ? error.message : String(error)
      );
      process.exit(1);
    }
  }
).command(
  "validate <file>",
  "Validate a FastMCP server file for syntax and basic structure",
  (yargs2) => {
    return yargs2.positional("file", {
      demandOption: true,
      describe: "The path to the server file",
      type: "string"
    }).option("strict", {
      alias: "s",
      default: false,
      describe: "Enable strict validation (type checking)",
      type: "boolean"
    });
  },
  async (argv) => {
    try {
      const { existsSync } = await Promise.resolve().then(() => _interopRequireWildcard(require("fs")));
      const { resolve } = await Promise.resolve().then(() => _interopRequireWildcard(require("path")));
      const filePath = resolve(argv.file);
      if (!existsSync(filePath)) {
        console.error(`[FastMCP Error] File not found: ${filePath}`);
        process.exit(1);
      }
      console.log(`[FastMCP] Validating server file: ${filePath}`);
      const command = argv.strict ? `npx tsc --noEmit --strict ${filePath}` : `npx tsc --noEmit ${filePath}`;
      try {
        await _execa.execa.call(void 0, {
          shell: true,
          stderr: "pipe",
          stdout: "pipe"
        })`${command}`;
        console.log("[FastMCP] \u2713 TypeScript compilation successful");
      } catch (tsError) {
        console.error("[FastMCP] \u2717 TypeScript compilation failed");
        if (tsError instanceof Error && "stderr" in tsError) {
          console.error(tsError.stderr);
        }
        process.exit(1);
      }
      try {
        await _execa.execa.call(void 0, {
          shell: true,
          stderr: "pipe",
          stdout: "pipe"
        })`node -e "
            (async () => {
              try {
                const { FastMCP } = await import('fastmcp');
                await import('file://${filePath}');
                console.log('[FastMCP] ✓ Server structure validation passed');
              } catch (error) {
                console.error('[FastMCP] ✗ Server structure validation failed:', error.message);
                process.exit(1);
              }
            })();
          "`;
      } catch (e) {
        console.error("[FastMCP] \u2717 Server structure validation failed");
        console.error("Make sure the file properly imports and uses FastMCP");
        process.exit(1);
      }
      console.log(
        "[FastMCP] \u2713 All validations passed! Server file looks good."
      );
    } catch (error) {
      console.error(
        "[FastMCP Error] Validation failed:",
        error instanceof Error ? error.message : String(error)
      );
      process.exit(1);
    }
  }
).help().parseAsync();
//# sourceMappingURL=fastmcp.cjs.map