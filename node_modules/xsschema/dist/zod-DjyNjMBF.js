import { m as missingDependenciesUrl } from './index-DoHiaFQM.js';

const getToJsonSchemaFn = async () => {
  let zodV4toJSONSchema = (_schema) => {
    throw new Error(`xsschema: Missing zod v4 dependencies "zod". see ${missingDependenciesUrl}`);
  };
  let zodV3ToJSONSchema = (_schema) => {
    throw new Error(`xsschema: Missing zod v3 dependencies "zod-to-json-schema". see ${missingDependenciesUrl}`);
  };
  try {
    const { toJSONSchema } = await import('zod/v4/core');
    zodV4toJSONSchema = ((schema) => toJSONSchema(schema, { target: "draft-7" }));
  } catch (err) {
    if (err instanceof Error)
      console.error(err.message);
  }
  try {
    const { zodToJsonSchema } = await import('zod-to-json-schema');
    zodV3ToJSONSchema = zodToJsonSchema;
  } catch (err) {
    if (err instanceof Error)
      console.error(err.message);
  }
  return async (schema) => {
    if ("_zod" in schema)
      return zodV4toJSONSchema(schema);
    else
      return zodV3ToJSONSchema(schema);
  };
};

export { getToJsonSchemaFn };
