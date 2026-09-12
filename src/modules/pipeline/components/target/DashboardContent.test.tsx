import { fireEvent, render } from "@testing-library/react";
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
