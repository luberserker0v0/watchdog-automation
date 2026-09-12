import { describe, expect, it } from "vitest";
import metadataSchema from "../../../schemas/watchdog-metadata.schema.json";

type JsonSchema = {
  description?: string;
  markdownDescription?: string;
  properties?: Record<string, JsonSchema>;
  prefixItems?: JsonSchema[];
  items?: JsonSchema | boolean;
  additionalProperties?: JsonSchema | boolean;
  oneOf?: JsonSchema[];
  anyOf?: JsonSchema[];
  allOf?: JsonSchema[];
  $defs?: Record<string, JsonSchema>;
};

const hasDescription = (schema: JsonSchema) =>
  Boolean(schema.description?.trim() || schema.markdownDescription?.trim());

function collectUndocumentedFields(
  schema: JsonSchema,
  path = "$",
  undocumented: string[] = [],
): string[] {
  for (const [name, property] of Object.entries(schema.properties ?? {})) {
    const propertyPath = `${path}.properties.${name}`;
    if (!hasDescription(property)) undocumented.push(propertyPath);
    collectUndocumentedFields(property, propertyPath, undocumented);
  }

  schema.prefixItems?.forEach((item, index) => {
    const itemPath = `${path}.prefixItems[${index}]`;
    if (!hasDescription(item)) undocumented.push(itemPath);
    collectUndocumentedFields(item, itemPath, undocumented);
  });

  if (schema.items && typeof schema.items === "object") {
    const itemPath = `${path}.items`;
    if (!hasDescription(schema.items)) undocumented.push(itemPath);
    collectUndocumentedFields(schema.items, itemPath, undocumented);
  }

  if (schema.additionalProperties && typeof schema.additionalProperties === "object") {
    collectUndocumentedFields(
      schema.additionalProperties,
      `${path}.additionalProperties`,
      undocumented,
    );
  }

  for (const keyword of ["oneOf", "anyOf", "allOf"] as const) {
    schema[keyword]?.forEach((variant, index) =>
      collectUndocumentedFields(variant, `${path}.${keyword}[${index}]`, undocumented),
    );
  }

  for (const [name, definition] of Object.entries(schema.$defs ?? {})) {
    collectUndocumentedFields(definition, `${path}.$defs.${name}`, undocumented);
  }

  return undocumented;
}

describe("WatchDog metadata schema", () => {
  it("gives every metadata field and tuple/array item editor documentation", () => {
    expect(collectUndocumentedFields(metadataSchema as unknown as JsonSchema)).toEqual([]);
  });
});
