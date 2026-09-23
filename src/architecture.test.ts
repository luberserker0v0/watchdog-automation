import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

function productionSources(root: string): string[] {
  return readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const path = join(root, entry.name);
    if (entry.isDirectory()) return productionSources(path);
    if (!/\.tsx?$/.test(entry.name) || entry.name.includes(".test.")) return [];
    return [path];
  });
}

function forbiddenImports(root: string, forbidden: RegExp): string[] {
  return productionSources(root).flatMap((path) => {
    const matches = readFileSync(path, "utf8")
      .split(/\r?\n/)
      .filter((line) => line.startsWith("import ") && forbidden.test(line));
    return matches.map((line) => `${relative(process.cwd(), path)}: ${line.trim()}`);
  });
}

describe("frontend feature boundaries", () => {
  it("keeps capture independent from pipeline runtime and testing", () => {
    expect(forbiddenImports(join(process.cwd(), "src/modules/capture"), /\/(pipeline\/runtime|testing)\//)).toEqual([]);
  });

  it("keeps authoring independent from runtime and testing", () => {
    expect(forbiddenImports(join(process.cwd(), "src/modules/pipeline/authoring"), /\/(runtime|testing)\//)).toEqual([]);
  });

  it("keeps observability independent from feature modules", () => {
    expect(forbiddenImports(join(process.cwd(), "src/modules/observability"), /\/(capture|pipeline|testing|recorder|settings)\//)).toEqual([]);
  });
});
