export const platform = (navigator.userAgent.includes("Win")
  ? "windows"
  : navigator.userAgent.includes("Mac")
    ? "macos"
    : "linux") as "windows" | "macos" | "linux";

export const isMac = platform === "macos";

export const cmdKey = isMac ? "⌘" : "Ctrl";
export const altKey = isMac ? "⌥" : "Alt";
export const delKey = isMac ? "⌫" : "Del";

export function formatShortcut(example: string): string {
  if (isMac) return example;
  return example
    .replace(/⌘/g, "Ctrl")
    .replace(/⌥/g, "Alt")
    .replace(/⇧/g, "Shift")
    .replace(/ ?(Ctrl|Alt|Shift) ?/g, " $1 + ")
    .replace(/\s+/g, " ")
    .trim();
}
