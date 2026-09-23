import { expect, it } from "vitest";
import { formatShortcut } from "./ShortcutCaptureInput";

it("normalizes captured keyboard combinations for Tauri global shortcuts", () => {
  expect(formatShortcut({ key: "r", code: "KeyR", ctrlKey: true, metaKey: false, altKey: true, shiftKey: false }))
    .toBe("CommandOrControl+Alt+R");
  expect(formatShortcut({ key: "F9", code: "F9", ctrlKey: false, metaKey: false, altKey: false, shiftKey: false }))
    .toBe("F9");
  expect(formatShortcut({ key: "r", code: "KeyR", ctrlKey: false, metaKey: true, altKey: false, shiftKey: false }))
    .toBe("Super+R");
  expect(formatShortcut({ key: "r", code: "KeyR", ctrlKey: false, metaKey: false, altKey: false, shiftKey: false }))
    .toBeNull();
});
