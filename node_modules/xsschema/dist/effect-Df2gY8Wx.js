import { t as tryImport } from './index-DoHiaFQM.js';

const getToJsonSchemaFn = async () => {
  const { JSONSchema } = await tryImport(import('effect'), "effect");
  return (schema) => JSONSchema.make(schema);
};

export { getToJsonSchemaFn };
