import { marked } from "marked";

marked.setOptions({ breaks: true, gfm: true });

const ALLOWED_TAGS = new Set([
  "B", "STRONG", "I", "EM", "UL", "OL", "LI", "H1", "H2", "P", "BR", "A",
]);

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// Parses with DOMParser (no browsing context — never fetches resources or
// runs scripts, unlike assigning untrusted HTML to a live element's
// innerHTML) and rebuilds using only the tags/attributes this app's editor
// can ever produce or round-trip. Everything else is dropped, keeping only
// its text content — this is what makes it safe to assign the result to
// innerHTML afterward.
function sanitizeHtml(html: string): string {
  const doc = new DOMParser().parseFromString(html, "text/html");
  return sanitizeWalk(doc.body);
}

function sanitizeWalk(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) {
    return escapeHtml(node.textContent ?? "");
  }
  if (node.nodeType !== Node.ELEMENT_NODE) return "";

  const el = node as HTMLElement;
  const inner = Array.from(el.childNodes).map(sanitizeWalk).join("");

  // Checklist items: only a bare type="checkbox" survives (this is the one
  // place a non-toolbar tag, <input>, is allowed through) — every other
  // attribute (disabled, id, onclick, ...) is dropped, and any other input
  // type is dropped entirely, since it has no text content to preserve.
  if (el.tagName === "INPUT") {
    if (el.getAttribute("type") !== "checkbox") return "";
    return el.hasAttribute("checked") ? '<input type="checkbox" checked>' : '<input type="checkbox">';
  }

  if (!ALLOWED_TAGS.has(el.tagName)) return inner;
  if (el.tagName === "BR") return "<br>";

  if (el.tagName === "A") {
    const href = el.getAttribute("href") ?? "";
    const safe = /^(https?:|mailto:)/i.test(href) ? href : "#";
    return `<a href="${safe.replace(/"/g, "&quot;")}">${inner}</a>`;
  }

  const tag = el.tagName.toLowerCase();
  return `<${tag}>${inner}</${tag}>`;
}

export function mdToHtml(md: string): string {
  const raw = marked.parse(md, { async: false }) as string;
  return sanitizeHtml(raw);
}

export function mdToPlainText(md: string): string {
  const html = mdToHtml(md);
  const doc = new DOMParser().parseFromString(html, "text/html");
  return doc.body.textContent ?? "";
}

// Serializes exactly the tags NoteEditor's toolbar can produce. Not a
// general HTML-to-Markdown converter — anything outside this tag set
// (e.g. pasted-in <table>, <img>) falls through to its text content.
//
// Accepts the live editor element directly (preferred) or an HTML string.
// A checkbox's checked *state* lives in the DOM property, not the `checked`
// *attribute* — toggling one in the UI never touches the attribute, so
// serializing to a string first and re-parsing that string (as this used to
// do unconditionally) silently drops every checkbox's live checked state.
export function htmlToMd(source: string | HTMLElement): string {
  let container: HTMLElement;
  if (typeof source === "string") {
    container = document.createElement("div");
    container.innerHTML = source;
  } else {
    container = source;
  }
  return walk(container).trim() + "\n";
}

function walk(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) {
    return node.textContent ?? "";
  }
  if (node.nodeType !== Node.ELEMENT_NODE) return "";

  const el = node as HTMLElement;
  const inner = Array.from(el.childNodes).map(walk).join("");

  switch (el.tagName) {
    case "B":
    case "STRONG":
      return `**${inner}**`;
    case "I":
    case "EM":
      return `*${inner}*`;
    case "H1":
      return `# ${inner}\n\n`;
    case "H2":
      return `## ${inner}\n\n`;
    case "UL":
      return Array.from(el.children).map((li) => {
        const checkbox = li.querySelector(':scope > input[type="checkbox"]') as HTMLInputElement | null;
        if (checkbox) {
          const mark = checkbox.checked ? "x" : " ";
          const rest = Array.from(li.childNodes)
            .filter((n) => n !== checkbox)
            .map(walk)
            .join("")
            .trim();
          return `- [${mark}] ${rest}\n`;
        }
        return `- ${walk(li).trim()}\n`;
      }).join("") + "\n";
    case "OL":
      return Array.from(el.children).map((li, i) => `${i + 1}. ${walk(li).trim()}\n`).join("") + "\n";
    case "LI":
      return inner;
    case "A": {
      const href = el.getAttribute("href") ?? "";
      return `[${inner}](${href})`;
    }
    case "P":
      return `${inner}\n\n`;
    case "BR":
      return "\n";
    case "DIV":
      // contenteditable wraps each line in a <div> in most webviews on Enter
      return `${inner}\n`;
    default:
      return inner;
  }
}
