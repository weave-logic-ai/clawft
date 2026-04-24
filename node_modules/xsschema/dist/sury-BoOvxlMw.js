import { t as tryImport } from './index-DoHiaFQM.js';

const getToJsonSchemaFn = async () => {
  const { toJSONSchema } = await tryImport(import('sury'), "sury");
  return (schema) => toJSONSchema(schema);
};

export { getToJsonSchemaFn };
