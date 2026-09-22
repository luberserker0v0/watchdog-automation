import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DashboardContent } from "./CombinedPage";

describe("Dashboard carousel", () => {
  it("converts a pointer swipe into horizontal scrolling", () => {
    const { container } = render(<DashboardContent />);
    const carousel = container.querySelector<HTMLElement>('[data-target="swipe:carousel"]');
    expect(carousel).not.toBeNull();

    fireEvent.pointerDown(carousel!, { button: 0, pointerId: 7, pointerType: "mouse", clientX: 200 });
    fireEvent.pointerMove(window, { pointerId: 7, pointerType: "mouse", clientX: 120 });

    expect(carousel!.scrollLeft).toBe(80);
    expect(carousel).toHaveClass("dragging");
  });

  it("supports document-level mouse dragging in both directions", () => {
    const { container } = render(<DashboardContent />);
    const carousel = container.querySelector<HTMLElement>('[data-target="swipe:carousel"]')!;
    carousel.scrollTo = ((options: ScrollToOptions) => {
      carousel.scrollLeft = options.left ?? carousel.scrollLeft;
    }) as typeof carousel.scrollTo;

    fireEvent.mouseDown(carousel, { button: 0, clientX: 200 });
    fireEvent.mouseMove(window, { clientX: 120 });
    expect(carousel.scrollLeft).toBe(80);
    fireEvent.mouseUp(window);

    carousel.scrollLeft = 100;
    fireEvent.mouseDown(carousel, { button: 0, clientX: 100 });
    fireEvent.mouseMove(window, { clientX: 160 });
    expect(carousel.scrollLeft).toBe(40);
    fireEvent.mouseUp(window);
  });
});

describe("Dashboard key detection", () => {
  it("replaces shortcut buttons with a persistent normalized key result", () => {
    render(<DashboardContent />);

    expect(screen.queryByText("Key Shortcuts")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Escape" })).not.toBeInTheDocument();
    expect(screen.getByRole("status", { name: "Key detect area" })).toHaveTextContent("Press any keyboard key");

    fireEvent.keyDown(window, { key: "Escape", code: "Escape" });

    expect(screen.getByRole("status", { name: "Key detect area" })).toHaveTextContent("Detected key");
    expect(screen.getByText("esc")).toHaveAttribute("data-target", "key:esc");
  });

  it("normalizes printable and special key names", () => {
    render(<DashboardContent />);

    fireEvent.keyDown(window, { key: "A", code: "KeyA" });
    expect(screen.getByText("a")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: " ", code: "Space" });
    expect(screen.getByText("space")).toBeInTheDocument();
  });
});

describe("Dashboard recovery test", () => {
  it("can ignore one armed click and recover on retry", () => {
    render(<DashboardContent showRecoveryLab />);

    expect(screen.getByText("Ready")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Arm first miss" }));
    expect(screen.getByText("First click will be ignored")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Apply recovery" }));
    expect(screen.getByText("Retry required")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Apply recovery" }));
    expect(screen.getByText("Recovered")).toBeInTheDocument();
  });

  it("keeps the recovery test out of the combined dashboard", () => {
    render(<DashboardContent />);
    expect(screen.queryByRole("region", { name: "Recovery Test" })).not.toBeInTheDocument();
  });
});
