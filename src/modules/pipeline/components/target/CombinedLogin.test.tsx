import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import CombinedLogin from "./CombinedLogin";

it("retains the target login workflow after extracting local form state", async () => {
  const user = userEvent.setup();
  const success = vi.fn();
  const { getByRole, getByPlaceholderText, getByText } = render(<CombinedLogin onSuccess={success} />);
  await user.type(getByPlaceholderText("Enter account..."), "wrong");
  await user.type(getByPlaceholderText("Enter password..."), "bad");
  await user.click(getByRole("button", { name: "Login" }));
  expect(getByText("Invalid account or password")).toBeInTheDocument();
  await user.clear(getByPlaceholderText("Enter account..."));
  await user.clear(getByPlaceholderText("Enter password..."));
  await user.type(getByPlaceholderText("Enter account..."), "admin");
  await user.type(getByPlaceholderText("Enter password..."), "123456");
  await user.click(getByRole("button", { name: "Login" }));
  expect(success).toHaveBeenCalledOnce();
});
