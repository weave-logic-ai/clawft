/**
 * Sandbox types for the Claude Code Agent SDK
 *
 * This file is the single source of truth for sandbox configuration types.
 * Both the SDK and the settings validation import from here.
 */
import { z } from 'zod/v4';
/**
 * Network configuration schema for sandbox.
 */
export declare const SandboxNetworkConfigSchema: z.ZodOptional<z.ZodObject<{
    allowedDomains: z.ZodOptional<z.ZodArray<z.ZodString>>;
    allowUnixSockets: z.ZodOptional<z.ZodArray<z.ZodString>>;
    allowAllUnixSockets: z.ZodOptional<z.ZodBoolean>;
    allowLocalBinding: z.ZodOptional<z.ZodBoolean>;
    httpProxyPort: z.ZodOptional<z.ZodNumber>;
    socksProxyPort: z.ZodOptional<z.ZodNumber>;
}, z.core.$strip>>;
/**
 * Sandbox settings schema.
 */
export declare const SandboxSettingsSchema: z.ZodObject<{
    enabled: z.ZodOptional<z.ZodBoolean>;
    autoAllowBashIfSandboxed: z.ZodOptional<z.ZodBoolean>;
    allowUnsandboxedCommands: z.ZodOptional<z.ZodBoolean>;
    network: z.ZodOptional<z.ZodObject<{
        allowedDomains: z.ZodOptional<z.ZodArray<z.ZodString>>;
        allowUnixSockets: z.ZodOptional<z.ZodArray<z.ZodString>>;
        allowAllUnixSockets: z.ZodOptional<z.ZodBoolean>;
        allowLocalBinding: z.ZodOptional<z.ZodBoolean>;
        httpProxyPort: z.ZodOptional<z.ZodNumber>;
        socksProxyPort: z.ZodOptional<z.ZodNumber>;
    }, z.core.$strip>>;
    ignoreViolations: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodArray<z.ZodString>>>;
    enableWeakerNestedSandbox: z.ZodOptional<z.ZodBoolean>;
    excludedCommands: z.ZodOptional<z.ZodArray<z.ZodString>>;
    ripgrep: z.ZodOptional<z.ZodObject<{
        command: z.ZodString;
        args: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>>;
}, z.core.$loose>;
export type SandboxSettings = z.infer<typeof SandboxSettingsSchema>;
export type SandboxNetworkConfig = NonNullable<z.infer<typeof SandboxNetworkConfigSchema>>;
export type SandboxIgnoreViolations = NonNullable<SandboxSettings['ignoreViolations']>;
