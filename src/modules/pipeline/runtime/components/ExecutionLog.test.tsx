import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { useLogStore } from "../../../capture/stores/useLogStore";
import ExecutionLog from "./ExecutionLog";

beforeEach(() => useLogStore.setState({ entries: [], nextId: 1 }));

describe("ExecutionLog Patrol policy tags", () => {
  it("renders hint-only and redirect modes as explicit badges", () => {
    useLogStore.getState().appendLog({
      source: "pipeline",
      label: "Patrol active",
      tag: "hint",
      detail: "Advisory only",
      result: "info",
    });
    useLogStore.getState().appendLog({
      source: "pipeline",
      label: "Patrol · Redirect",
      tag: "redirect",
      detail: "Accepted",
      result: "success",
    });

    render(<ExecutionLog />);

    expect(screen.getByText("HINT ONLY")).toHaveClass("execution-log-tag", "hint");
    expect(screen.getByText("REDIRECT")).toHaveClass("execution-log-tag", "redirect");
  });
});
