// Auth flow tests — bun:test style, runs the REAL js/auth.js + js/config.js
// with stubbed browser APIs. Run: bun test  (or `turbo run test --filter=@whispermic/website`)
import { describe, test, expect, beforeEach } from "bun:test";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

const store: Record<string, string> = {};
(globalThis as any).localStorage = {
  getItem: (k: string) => (k in store ? store[k] : null),
  setItem: (k: string, v: string) => {
    store[k] = String(v);
  },
  removeItem: (k: string) => {
    delete store[k];
  },
};
(globalThis as any).window = {};
(globalThis as any).document = {
  addEventListener: () => {},
  querySelectorAll: () => [],
  getElementById: () => null,
};

const src =
  (await Bun.file(path.join(root, "js/config.js")).text()) +
  "\n" +
  (await Bun.file(path.join(root, "js/auth.js")).text()) +
  "\nglobalThis.Auth=Auth;globalThis.apiSignup=apiSignup;globalThis.apiLogin=apiLogin;";
eval(src);

const g = globalThis as any;
const apiSignup = g.apiSignup as (n: string, e: string, p: string) => Promise<any>;
const apiLogin = g.apiLogin as (e: string, p: string) => Promise<any>;
const Auth = g.Auth as { get(): any; set(u: any): void; clear(): void };

beforeEach(() => {
  for (const k of Object.keys(store)) delete store[k];
});

describe("auth", () => {
  test("signup returns user", async () => {
    const u = await apiSignup("Ada", "ada@x.com", "secret1");
    expect(u.email).toBe("ada@x.com");
  });

  test("duplicate signup rejected", async () => {
    await apiSignup("Ada", "ada@x.com", "secret1");
    await expect(apiSignup("Ada", "ada@x.com", "secret1")).rejects.toThrow(/already registered/);
  });

  test("login returns stored name", async () => {
    await apiSignup("Ada", "ada@x.com", "secret1");
    const l = await apiLogin("ada@x.com", "secret1");
    expect(l.name).toBe("Ada");
  });

  test("wrong password rejected", async () => {
    await apiSignup("Ada", "ada@x.com", "secret1");
    await expect(apiLogin("ada@x.com", "wrong")).rejects.toThrow(/Invalid/);
  });

  test("unknown user rejected", async () => {
    await expect(apiLogin("no@x.com", "secret1")).rejects.toThrow(/Invalid/);
  });

  test("session persists + logout clears", async () => {
    await apiSignup("Ada", "ada@x.com", "secret1");
    const l = await apiLogin("ada@x.com", "secret1");
    Auth.set(l);
    expect(Auth.get().email).toBe("ada@x.com");
    Auth.clear();
    expect(Auth.get()).toBeNull();
  });
});
