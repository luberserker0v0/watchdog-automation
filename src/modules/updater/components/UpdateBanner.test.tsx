import { render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { resetUpdateStoreForTests, useUpdateStore } from "../stores/useUpdateStore";
import UpdateBanner from "./UpdateBanner";

beforeEach(resetUpdateStoreForTests);

it("shows a persistent localized banner only when an update is available", () => {
  const open = vi.fn();
  const { rerender } = render(<UpdateBanner onOpenSettings={open} />);
  expect(screen.queryByRole("status")).toBeNull();
  useUpdateStore.getState().setState({
    phase: "available",
    update: { currentVersion: "1.0.0", version: "1.1.0", publicationDate: null, notes: "Fixes" },
  });
  rerender(<UpdateBanner onOpenSettings={open} />);
  expect(screen.getByText("WatchDog 1.1.0 is available")).toBeTruthy();
});
