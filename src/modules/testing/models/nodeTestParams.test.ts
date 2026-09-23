import { describe, expect, it } from "vitest";
import { defaultNodeParams } from "./nodeTestParams";

describe("defaultNodeParams", () => {
  it("hydrates every supported field value and skips missing defaults", () => {
    expect(defaultNodeParams({
      params: [
        { type: "text", key: "account", label: "Account", default: "admin" },
        { type: "number", key: "retries", label: "Retries", default: 3 },
        { type: "switch", key: "enabled", label: "Enabled", default: true },
        { type: "text", key: "optional", label: "Optional" },
      ],
    })).toEqual({ account: "admin", retries: 3, enabled: true });
  });
});
