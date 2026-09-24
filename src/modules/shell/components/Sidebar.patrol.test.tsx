import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it } from "vitest";
import Sidebar from "./Sidebar";
import { useUiStore } from "../stores/useUiStore";

beforeEach(() => useUiStore.setState({ activeView: "pipeline", sidebarOpen: false }));

it("opens Patrol Chat Room from the third sidebar item", async () => {
  render(<Sidebar />);
  await userEvent.setup().click(screen.getByRole("button", { name: "Patrol Chat Room" }));
  expect(useUiStore.getState().activeView).toBe("patrolChat");
  expect(screen.getByRole("button", { name: "Patrol Chat Room" })).toHaveClass("active");
});

it("hides Node Test in the production sidebar", () => {
  render(<Sidebar />);
  expect(screen.queryByRole("button", { name: "Node Test" })).not.toBeInTheDocument();
});

it("shows Node Test when the Tester entry point enables it", () => {
  render(<Sidebar showNodeTest />);
  expect(screen.getByRole("button", { name: "Node Test" })).toBeInTheDocument();
});
