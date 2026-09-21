/** Match a window title exactly when constructing a target filter. */
export function escapeExactRegex(value: string): string {
  return `^${value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`;
}
