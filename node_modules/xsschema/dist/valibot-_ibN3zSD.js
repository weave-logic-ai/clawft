import { t as tryImport } from './index-DoHiaFQM.js';

const getToJsonSchemaFn = async () => {
  const { toJsonSchema } = await tryImport(import('@valibot/to-json-schema'), "@valibot/to-json-schema");
  return (schema) => toJsonSchema(schema);
};

export { getToJsonSchemaFn };
