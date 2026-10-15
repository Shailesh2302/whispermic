// Download-button wiring tests — runs the REAL js/main.js with stubbed DOM.
// Verifies: local mode → direct downloads/ links (win/mac) + guide kept (linux),
// releases mode → GitHub asset URLs, OS row gets highlighted.
import { describe, test, expect, beforeEach } from "bun:test";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

type El = {
  href: string;
  textContent: string;
  attrs: Record<string, string>;
  getAttribute(k: string): string | null;
  classList: { add(c: string): void; contains(c: string): boolean };
};
function anchor(attrs: Record<string, string> = {}, href = ""): El {
  const cls = new Set<string>();
  return {
    href,
    textContent: "",
    attrs,
    getAttribute: (k) => attrs[k] ?? null,
    classList: { add: (c) => void cls.add(c), contains: (c) => cls.has(c) },
  };
}

let dl: El[] = [];
let files: El[] = [];
let labels: El[] = [];
let rows: El[] = [];
const listeners: Record<string, Function[]> = {};

(globalThis as any).window = { APP_CONFIG: { VERSION: "0.1.0", RELEASES_URL: "" } };
(globalThis as any).navigator = { platform: "Win32", userAgent: "Windows NT" };
(globalThis as any).document = {
  addEventListener: (ev: string, fn: Function) => void ((listeners[ev] ||= []).push(fn)),
  querySelectorAll: (sel: string) =>
    sel === "[data-dl]" ? dl : sel === "[data-dl-file]" ? files
    : sel === "[data-os-dl]" ? labels : sel === "[data-os-row]" ? rows : [],
  getElementById: () => null,
};

await Bun.file(path.join(root, "js/config.js")).text(); // parsed by lint; config below is authoritative
eval(await Bun.file(path.join(root, "js/main.js")).text());

const fire = () => {
  for (const fn of listeners["DOMContentLoaded"] || []) fn();
};
const cfg = () => (globalThis as any).window.APP_CONFIG;

beforeEach(() => {
  dl = [anchor({}, "install.html")];
  files = [anchor({ "data-dl-file": "WhisperMic_0.1.0_amd64.deb" }, "downloads/WhisperMic_0.1.0_amd64.deb")];
  labels = [anchor()];
  rows = [anchor({ "data-os-row": "windows" }), anchor({ "data-os-row": "linux" })];
  cfg().RELEASES_URL = "";
  (globalThis as any).navigator = { platform: "Win32", userAgent: "Windows NT" };
});

describe("downloads", () => {
  test("windows local mode: hero downloads .msi directly", () => {
    fire();
    expect(dl[0].href).toBe("downloads/WhisperMic_0.1.0_x64.msi");
    expect(labels[0].textContent).toBe("Windows (.msi)");
  });

  test("linux local mode: keeps guide link, highlights linux rows", () => {
    (globalThis as any).navigator = { platform: "Linux x86_64", userAgent: "Linux" };
    fire();
    expect(dl[0].href).toBe("install.html");
    expect(files[0].href).toBe("downloads/WhisperMic_0.1.0_amd64.deb");
    expect(rows[1].classList.contains("picked")).toBe(true);
    expect(rows[0].classList.contains("picked")).toBe(false);
  });

  test("releases mode: buttons + files point at release assets", () => {
    cfg().RELEASES_URL = "https://github.com/acme/whispermic/releases";
    fire();
    expect(dl[0].href).toBe("https://github.com/acme/whispermic/releases");
    expect(files[0].href).toBe(
      "https://github.com/acme/whispermic/releases/download/v0.1.0/WhisperMic_0.1.0_amd64.deb"
    );
  });
});
