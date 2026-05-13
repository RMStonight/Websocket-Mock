import { describe, expect, it } from "vitest";
import { compactJson, formatJson, makeEndpoint, validateJson, validateJsonTemplate } from "./json";

describe("json helpers", () => {
  it("validates JSON text", () => {
    expect(validateJson('{"ok":true}').valid).toBe(true);
    expect(validateJson("{bad").valid).toBe(false);
  });

  it("formats and compacts JSON", () => {
    expect(formatJson('{"ok":true}')).toContain("\n");
    expect(compactJson('{"ok": true }')).toBe('{"ok":true}');
  });

  it("validates supported JSON template placeholders", () => {
    expect(validateJsonTemplate('{"echo":"{{message}}","raw":{{jsonMessage}}}').valid).toBe(true);
  });

  it("normalizes websocket endpoints", () => {
    expect(makeEndpoint("127.0.0.1", 9001, "mock")).toBe("ws://127.0.0.1:9001/mock");
    expect(makeEndpoint("127.0.0.1", 9001, "/mock")).toBe("ws://127.0.0.1:9001/mock");
  });
});
