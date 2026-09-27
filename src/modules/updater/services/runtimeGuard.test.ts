import { beforeEach, expect, it } from "vitest";
import { activateUpdateInstallationGuard, assertRuntimeStartAllowed, releaseUpdateInstallationGuard } from "./runtimeGuard";

beforeEach(releaseUpdateInstallationGuard);

it("blocks new runtime work only while installation owns the guard", () => {
  expect(() => assertRuntimeStartAllowed()).not.toThrow();
  expect(activateUpdateInstallationGuard()).toBe(true);
  expect(activateUpdateInstallationGuard()).toBe(false);
  expect(() => assertRuntimeStartAllowed()).toThrow(/update is being installed/i);
  releaseUpdateInstallationGuard();
  expect(() => assertRuntimeStartAllowed()).not.toThrow();
});
