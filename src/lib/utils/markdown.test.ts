import { htmlToMd, mdToHtml } from "./markdown";

// Run in a browser with Vite running:
// await (await import('/src/lib/utils/markdown.test.ts')).runMarkdownTests()
export function runMarkdownTests(): string[] {
  const passed: string[] = [];
  function test(name: string, run: () => void): void {
    run();
    passed.push(name);
  }
  function equal(actual: unknown, expected: unknown): void {
    if (actual !== expected) {
      throw new Error(`Expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
  }
  function editor(html: string): HTMLDivElement {
    const element = document.createElement("div");
    element.innerHTML = html;
    return element;
  }

  test("GFM tasks render as enabled checkboxes", () => {
    const element = editor(mdToHtml("- [ ] Pending\n- [x] Done\n"));
    const boxes = element.querySelectorAll<HTMLInputElement>("input");
    equal(boxes.length, 2);
    equal(boxes[0].checked, false);
    equal(boxes[1].checked, true);
    equal(boxes[0].disabled, false);
    equal(boxes[1].disabled, false);
  });

  test("live checkbox state wins over stale HTML attributes and survives reload", () => {
    const element = editor(mdToHtml("- [ ] Pending\n- [x] Done\n"));
    const boxes = element.querySelectorAll<HTMLInputElement>("input");
    boxes[0].checked = true;
    boxes[1].checked = false;
    equal(boxes[0].hasAttribute("checked"), false);
    equal(boxes[1].hasAttribute("checked"), true);
    const saved = htmlToMd(element);
    equal(saved, "- [x] Pending\n- [ ] Done\n");
    equal(htmlToMd(editor(mdToHtml(saved))), saved);
  });

  test("string input preserves task state and inline formatting", () => {
    equal(htmlToMd('<ul><li><input type="checkbox" checked> <strong>Done</strong></li><li>Plain</li></ul>'),
      "- [x] **Done**\n- Plain\n");
  });

  test("checkbox sanitizer strips handlers and unrelated attributes", () => {
    const element = editor(mdToHtml('<input type="checkbox" checked disabled id="unsafe" onclick="alert(1)" style="color:red" name="secret">'));
    const box = element.querySelector<HTMLInputElement>("input")!;
    equal(box.type, "checkbox");
    equal(box.checked, true);
    equal(Array.from(box.attributes).map((attr) => attr.name).sort().join(","), "checked,type");
  });

  test("sanitizer removes other controls, executable tags, and unsafe links", () => {
    const element = editor(mdToHtml('<p><input type="text" value="secret"><img src="x" onerror="alert(1)"><script>alert(1)</script><a href="javascript:alert(1)" onclick="alert(1)">Link</a></p>'));
    equal(element.querySelector("input,img,script"), null);
    equal(element.querySelector("a")?.getAttribute("href"), "#");
    equal(element.querySelector("a")?.hasAttribute("onclick"), false);
  });

  test("regular lists remain unchanged", () => {
    equal(htmlToMd(mdToHtml("- First\n- Second\n")), "- First\n- Second\n");
    equal(htmlToMd(mdToHtml("1. First\n2. Second\n")), "1. First\n2. Second\n");
  });
  return passed;
}
