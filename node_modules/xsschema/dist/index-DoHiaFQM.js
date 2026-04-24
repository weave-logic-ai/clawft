const jsonSchema = (schema) => schema;
const strictJsonSchema = (schema) => ({
  ...schema,
  additionalProperties: false,
  properties: schema.properties != null ? Object.fromEntries(
    Object.entries(schema.properties).map(([k, v]) => [
      k,
      v != null && typeof v === "object" && "type" in v && v.type === "object" ? strictJsonSchema(v) : v
    ])
  ) : schema.properties
});

const isStandardJSONSchemaV1 = (schema) => "jsonSchema" in schema["~standard"];

const missingDependenciesUrl = "https://xsai.js.org/docs/packages-top/xsschema#missing-dependencies";
const tryImport = async (result, name) => {
  try {
    return await result;
  } catch {
    throw new Error(`xsschema: Missing dependencies "${name}". see ${missingDependenciesUrl}`);
  }
};
const getToJsonSchemaFn = async (vendor) => {
  switch (vendor) {
    case "arktype":
      return import('./arktype-C-GObzDh.js').then(async ({ getToJsonSchemaFn: getToJsonSchemaFn2 }) => getToJsonSchemaFn2());
    case "effect":
      return import('./effect-Df2gY8Wx.js').then(async ({ getToJsonSchemaFn: getToJsonSchemaFn2 }) => getToJsonSchemaFn2());
    case "sury":
      return import('./sury-BoOvxlMw.js').then(async ({ getToJsonSchemaFn: getToJsonSchemaFn2 }) => getToJsonSchemaFn2());
    case "valibot":
      return import('./valibot-_ibN3zSD.js').then(async ({ getToJsonSchemaFn: getToJsonSchemaFn2 }) => getToJsonSchemaFn2());
    case "zod":
      return import('./zod-DjyNjMBF.js').then(async ({ getToJsonSchemaFn: getToJsonSchemaFn2 }) => getToJsonSchemaFn2());
    default:
      throw new Error(`xsschema: Unsupported schema vendor "${vendor}". see https://xsai.js.org/docs/packages-top/xsschema#unsupported-schema-vendor`);
  }
};

const toJsonSchema = async (schema) => isStandardJSONSchemaV1(schema) ? schema["~standard"].jsonSchema.input({ target: "draft-07" }) : getToJsonSchemaFn(schema["~standard"].vendor).then(async (toJsonSchema2) => toJsonSchema2(schema));

const validate = async (schema, input) => {
  let result = schema["~standard"].validate(input);
  if (result instanceof Promise)
    result = await result;
  if (result.issues)
    throw new Error(JSON.stringify(result.issues, null, 2));
  return result.value;
};

export { toJsonSchema as a, jsonSchema as j, missingDependenciesUrl as m, strictJsonSchema as s, tryImport as t, validate as v };
