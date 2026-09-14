import { describe, expect, it } from "vitest";
import metadataSchema from "../../../schemas/watchdog-metadata.schema.json";

type JsonSchema = {
  title?: string;
  type?: string;
  default?: unknown;
  examples?: unknown[];
  $ref?: string;
  description?: string;
  markdownDescription?: string;
  properties?: Record<string, JsonSchema>;
  required?: string[];
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

  it("states supported values in every metadata property description", () => {
    const missing: string[] = [];
    const visit = (schema: JsonSchema, path = "$") => {
      for (const [name, property] of Object.entries(schema.properties ?? {})) {
        const propertyPath = `${path}.properties.${name}`;
        if (!/supported value/i.test(property.description ?? "")) missing.push(propertyPath);
        visit(property, propertyPath);
      }
      for (const [name, definition] of Object.entries(schema.$defs ?? {})) {
        visit(definition, `${path}.$defs.${name}`);
      }
      schema.oneOf?.forEach((item, index) => {
        const variantPath = `${path}.oneOf[${index}]`;
        if (!/supported/i.test(item.description ?? "")) missing.push(variantPath);
        visit(item, variantPath);
      });
      schema.anyOf?.forEach((item, index) => visit(item, `${path}.anyOf[${index}]`));
      schema.allOf?.forEach((item, index) => visit(item, `${path}.allOf[${index}]`));
    };

    visit(metadataSchema as unknown as JsonSchema);
    expect(missing).toEqual([]);
  });

  it("formats complex editor help with Markdown sections and line breaks", () => {
    const missing: string[] = [];
    const visit = (schema: JsonSchema, path = "$") => {
      if (
        /supported|required|default/i.test(schema.description ?? "") &&
        !schema.markdownDescription?.includes("\n\n")
      ) {
        missing.push(path);
      }
      for (const [name, property] of Object.entries(schema.properties ?? {})) {
        visit(property, `${path}.properties.${name}`);
      }
      for (const [name, definition] of Object.entries(schema.$defs ?? {})) {
        visit(definition, `${path}.$defs.${name}`);
      }
      schema.oneOf?.forEach((item, index) => visit(item, `${path}.oneOf[${index}]`));
      schema.anyOf?.forEach((item, index) => visit(item, `${path}.anyOf[${index}]`));
      schema.allOf?.forEach((item, index) => visit(item, `${path}.allOf[${index}]`));
      schema.prefixItems?.forEach((item, index) => visit(item, `${path}.prefixItems[${index}]`));
      if (schema.items && typeof schema.items === "object") visit(schema.items, `${path}.items`);
    };

    visit(metadataSchema as unknown as JsonSchema);
    expect(missing).toEqual([]);
  });

  it("documents every declared default in the field description", () => {
    const missing: string[] = [];
    const visit = (schema: JsonSchema, path = "$") => {
      if (Object.prototype.hasOwnProperty.call(schema, "default") && !/(default|omit)/i.test(schema.description ?? schema.markdownDescription ?? "")) {
        missing.push(path);
      }
      for (const [name, property] of Object.entries(schema.properties ?? {})) visit(property, `${path}.properties.${name}`);
      for (const [name, definition] of Object.entries(schema.$defs ?? {})) visit(definition, `${path}.$defs.${name}`);
      schema.oneOf?.forEach((item, index) => visit(item, `${path}.oneOf[${index}]`));
      schema.anyOf?.forEach((item, index) => visit(item, `${path}.anyOf[${index}]`));
      schema.allOf?.forEach((item, index) => visit(item, `${path}.allOf[${index}]`));
      schema.prefixItems?.forEach((item, index) => visit(item, `${path}.prefixItems[${index}]`));
      if (schema.items && typeof schema.items === "object") visit(schema.items, `${path}.items`);
      if (schema.additionalProperties && typeof schema.additionalProperties === "object") visit(schema.additionalProperties, `${path}.additionalProperties`);
    };
    visit(metadataSchema as unknown as JsonSchema);
    expect(missing).toEqual([]);
  });

  it("allows a window target with only type and capture_area", () => {
    const schema = metadataSchema as unknown as JsonSchema;
    expect(schema.properties?.target).toBeDefined();
    const target = schema.$defs!.captureTarget;
    const windowTarget = target.oneOf![1];
    expect(windowTarget.properties?.capture_area).toBeDefined();
    expect(windowTarget.anyOf).toBeUndefined();
  });

  it("documents top-level client window sizing and excludes it from node overrides", () => {
    const schema = metadataSchema as unknown as JsonSchema;
    const windowTarget = schema.$defs!.captureTarget.oneOf![1];
    expect(windowTarget.properties?.window_size?.prefixItems).toHaveLength(2);
    expect(windowTarget.properties?.window_size?.description).toMatch(/client\/content size/i);
    const nodeTarget = schema.$defs!.nodeFields.properties!.target.oneOf![0];
    expect(JSON.stringify(nodeTarget)).toContain("window_size");
    expect(JSON.stringify(nodeTarget)).toContain("not");
  });

  it("publishes effective runtime defaults at editor-visible field sites", () => {
    const schema = metadataSchema as unknown as JsonSchema;
    const defs = schema.$defs!;
    const byTitle = (schema: JsonSchema, title: string) =>
      schema.oneOf!.find((variant) => variant.title === title)!;

    const zeroRegion = [0, 0, 0, 0];
    expect(schema.properties!.target.default).toBeNull();
    expect(schema.properties!.nodes.default).toEqual({});
    expect(schema.properties!.edges.default).toEqual({});
    expect(defs.captureTarget.oneOf![0].properties!.index.default).toBe(0);
    expect(defs.captureTarget.oneOf![1].properties!.capture_area.default).toBe("frame");

    for (const title of ["Direct hit", "Template match", "OCR", "Color match", "Custom recognizer"]) {
      const properties = byTitle(defs.recognition, title).properties!;
      expect(properties.roi.default, `${title}.roi`).toEqual(zeroRegion);
      expect(properties.roi_offset.default, `${title}.roi_offset`).toEqual(zeroRegion);
      expect(properties.candidate_selection.default, `${title}.candidate_selection`).toEqual({
        order: "native",
        index: 0,
      });
    }

    expect(byTitle(defs.recognition, "Template match").properties!.threshold.default).toBe(0.7);
    expect(byTitle(defs.recognition, "OCR").properties!.threshold.default).toBe(0.3);
    expect(byTitle(defs.recognition, "Custom recognizer").properties!.param.default).toBeNull();
    expect(defs.recognition.default).toEqual({ type: "DirectHit" });

    for (const title of ["Click", "Long press", "Scroll"]) {
      const properties = byTitle(defs.action, title).properties!;
      expect(properties.target.default, `${title}.target`).toBe(true);
      expect(properties.target_offset.default, `${title}.target_offset`).toEqual(zeroRegion);
    }

    const swipe = byTitle(defs.action, "Swipe").properties!;
    expect(swipe.begin.default).toBe(true);
    expect(swipe.end.default).toBe(true);
    expect(swipe.begin_offset.default).toEqual(zeroRegion);
    expect(swipe.end_offset.default).toEqual(zeroRegion);
    expect(byTitle(defs.action, "Long press").properties!.duration.default).toBe(1000);
    expect(swipe.duration.default).toBe(200);
    expect(byTitle(defs.action, "Scroll").properties!.dx.default).toBe(0);
    expect(byTitle(defs.action, "Scroll").properties!.dy.default).toBe(0);
    expect(byTitle(defs.action, "Custom action").properties!.param.default).toBeNull();
    expect(defs.action.default).toEqual({ type: "DoNothing" });

    expect(defs.parameter.properties!.placeholder.default).toBe("");
    expect(defs.parameter.properties!.default.default).toBeNull();
    expect(defs.parameter.properties!.options.default).toEqual([]);
    expect(defs.nodeFields.properties!.recognition.default).toEqual({ type: "DirectHit" });
    expect(defs.nodeFields.properties!.actions.default).toEqual([]);
    expect(defs.nodeFields.properties!.params.default).toEqual([]);
    expect(defs.nodeFields.properties!.target.default).toBeNull();
    expect(defs.nodeFields.properties!.timeout.default).toBe(20000);
    expect(defs.nodeFields.properties!.attach.default).toBeNull();
    expect(defs.edgeMetadata.properties!.edge_type.default).toBe("next");
  });

  it("allows node recognition to use its DirectHit engine default", () => {
    const nodeFields = (metadataSchema as unknown as JsonSchema).$defs!.nodeFields;
    expect(JSON.stringify(nodeFields.required ?? [])).not.toContain("recognition");
    expect(nodeFields.properties!.recognition.default).toEqual({ type: "DirectHit" });
  });

  it("documents every supported action target form at each action field", () => {
    const defs = (metadataSchema as unknown as JsonSchema).$defs!;
    expect(defs.actionTarget.examples).toEqual([
      true,
      "RecognizeButton",
      [500, 300],
      [100, 200, 400, 300],
    ]);

    const byTitle = (title: string) =>
      defs.action.oneOf!.find((variant) => variant.title === title)!.properties!;
    const targetFields = [
      byTitle("Click").target,
      byTitle("Long press").target,
      byTitle("Swipe").begin,
      byTitle("Swipe").end,
      byTitle("Scroll").target,
    ];

    for (const field of targetFields) {
      expect(field.description).toContain("true | string");
      expect(field.description).toContain("[integer, integer]");
      expect(field.description).toContain("[integer, integer, integer, integer]");
      expect(field.markdownDescription).toContain("`true`");
      expect(field.markdownDescription).toContain("`\"NodeId\"`");
      expect(field.markdownDescription).toContain("`[x, y]`");
      expect(field.markdownDescription).toContain("`[x, y, width, height]`");
    }

    for (const variant of defs.actionTarget.oneOf!) {
      expect(variant.markdownDescription).toContain("**Supported action targets**\n\n- `true`");
      expect(variant.markdownDescription).toContain("**Selected:**");
      expect(variant.markdownDescription).toContain("**Default:** `true`");
    }
    expect(defs.point.markdownDescription).toContain("\n\n- `true`");
    expect(defs.region.markdownDescription).toContain("\n\n- `true`");
  });
});
