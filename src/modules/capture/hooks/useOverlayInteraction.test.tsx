import { useEffect } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useOverlayInteraction } from "./useOverlayInteraction";

function Harness() {
  const interaction = useOverlayInteraction();
  useEffect(() => {
    Object.defineProperty(interaction.containerRef.current, "getBoundingClientRect", {
      value: () => ({ left: 0, top: 0, width: 400, height: 300, right: 400, bottom: 300, x: 0, y: 0, toJSON: () => ({}) }),
    });
    Object.defineProperty(interaction.imgRef.current, "getBoundingClientRect", {
      value: () => ({ left: 40, top: 30, width: 320, height: 240, right: 360, bottom: 270, x: 40, y: 30, toJSON: () => ({}) }),
    });
  }, [interaction.containerRef]);
  return (
    <div
      ref={interaction.containerRef}
      data-testid="preview"
      className={`interaction-${interaction.interactionMode}${interaction.isPanning ? " is-panning" : ""}`}
      onMouseDown={interaction.handleMouseDown}
      onMouseMove={interaction.handleMouseMove}
      onMouseUp={interaction.handleMouseUp}
    >
      <img ref={interaction.imgRef} alt="frame" style={interaction.imageStyle} />
      <output>{interaction.zoom.toFixed(2)}</output>
      {interaction.renderSidePanel(true)}
    </div>
  );
}

describe("preview wheel zoom", () => {
  it("zooms in with wheel up and returns to fit with wheel down", async () => {
    render(<Harness />);
    const preview = screen.getByTestId("preview");

    fireEvent.wheel(preview, { deltaY: -300, clientX: 300, clientY: 150 });
    await waitFor(() => expect(Number(screen.getByText(/\d+\.\d+/).textContent)).toBeGreaterThan(1));
    expect(screen.getByAltText("frame").style.transform).toContain("scale(");

    fireEvent.wheel(preview, { deltaY: 5000, clientX: 300, clientY: 150 });
    await waitFor(() => expect(screen.getByText("1.00")).toBeInTheDocument());
    expect(screen.getByAltText("frame")).toHaveStyle({ transform: "translate(0px, 0px) scale(1)" });
  });

  it("switches between hand, rectangle, and cursor modes with active icons", async () => {
    render(<Harness />);
    const preview = screen.getByTestId("preview");
    const select = screen.getByRole("button", { name: "Select mode" });
    const drag = screen.getByRole("button", { name: "Drag mode" });
    const normal = screen.getByRole("button", { name: "Normal mode" });

    expect(select).toHaveAttribute("aria-pressed", "true");
    expect(select.querySelector("svg rect")).toHaveAttribute("fill", "currentColor");

    fireEvent.click(normal);
    expect(normal).toHaveAttribute("aria-pressed", "true");
    expect(normal.querySelector("svg path")).toHaveAttribute("fill", "currentColor");
    expect(preview).toHaveClass("interaction-normal");

    fireEvent.wheel(preview, { deltaY: -500, clientX: 200, clientY: 150 });
    await waitFor(() => expect(Number(screen.getByText(/\d+\.\d+/).textContent)).toBeGreaterThan(1));
    fireEvent.click(drag);
    const before = screen.getByAltText("frame").style.transform;
    fireEvent.mouseDown(preview, { clientX: 200, clientY: 150 });
    fireEvent.mouseMove(preview, { clientX: 230, clientY: 170 });
    await waitFor(() => expect(screen.getByAltText("frame").style.transform).not.toBe(before));
    fireEvent.mouseUp(preview, { clientX: 230, clientY: 170 });
    expect(drag).toHaveAttribute("aria-pressed", "true");
    expect(drag.querySelector("svg path")).toHaveAttribute("fill", "currentColor");

    fireEvent.click(screen.getByRole("button", { name: "Fit image" }));
    await waitFor(() => expect(screen.getByText("1.00")).toBeInTheDocument());
    expect(screen.getByAltText("frame")).toHaveStyle({ transform: "translate(0px, 0px) scale(1)" });
  });
});
