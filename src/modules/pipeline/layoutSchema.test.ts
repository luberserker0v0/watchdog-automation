import { describe, expect, it } from "vitest";
import layoutSchema from "../../../schemas/watchdog-layout.schema.json";

describe("WatchDog pipeline layout schema", () => {
  it("keeps presentation data in a small node-position-only document", () => {
    expect(layoutSchema.required).toEqual(["nodes"]);
    expect(layoutSchema.additionalProperties).toBe(false);
    expect(layoutSchema.properties.nodes.default).toEqual({});
    expect(layoutSchema.properties.nodes.additionalProperties.$ref).toBe("#/$defs/nodePosition");
    expect(layoutSchema.$defs.nodePosition.required).toEqual(["x", "y"]);
    expect(layoutSchema.$defs.nodePosition.additionalProperties).toBe(false);
    expect(layoutSchema.$defs.nodePosition.properties.x.type).toBe("integer");
    expect(layoutSchema.$defs.nodePosition.properties.y.type).toBe("integer");
  });

  it("explains supported values and automatic-layout defaults in editor help", () => {
    expect(layoutSchema.properties.nodes.description).toMatch(/supported value/i);
    expect(layoutSchema.properties.nodes.description).toMatch(/default/i);
    expect(layoutSchema.$defs.nodePosition.properties.x.markdownDescription).toContain("**Supported value**");
    expect(layoutSchema.$defs.nodePosition.properties.y.markdownDescription).toContain("**Required:**");
  });
});
