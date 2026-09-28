import { beforeEach, expect, mock, test } from "bun:test";

const invoke = mock(async (_command: string, _args?: unknown): Promise<unknown> => null);
const listen = mock(async (_name: string, _handler: unknown) => () => {});
const once = mock(async (_name: string, _handler: unknown) => () => {});
const emit = mock(async (_name: string, _payload: unknown) => {});
const emitTo = mock(async (_target: unknown, _name: string, _payload: unknown) => {});

mock.module("@tauri-apps/api/core", () => ({ invoke, Channel: class {} }));
mock.module("@tauri-apps/api/event", () => ({ listen, once, emit, emitTo }));

const { commands, events } = await import("./generated/bindings");

beforeEach(() => {
  invoke.mockClear();
  listen.mockClear();
  emit.mockClear();
});

test("renamed command uses the exact wire name and argument casing", async () => {
  invoke.mockResolvedValueOnce("42");
  expect(await commands.systemPing({ request_id: "42" })).toBe("42");
  expect(invoke).toHaveBeenCalledWith("system_ping", { request_id: "42" });
});

test("success responses pass through unchanged", async () => {
  const profile = { display_name: "Ada", score: 42 };
  invoke.mockResolvedValueOnce(profile);
  expect(await commands.lookup({ userName: "Ada" })).toBe(profile);
});

test("structured Rust errors and transport failures retain their identity", async () => {
  for (const failure of [{ kind: "permission_denied" }, new Error("IPC unavailable")]) {
    invoke.mockRejectedValueOnce(failure);
    await expect(commands.lookup({ userName: "Ada" })).rejects.toBe(failure);
  }
});

test("events use the same name for listen and emit", async () => {
  const handler = () => {};
  await events["download-progress"].listen(handler);
  expect(listen).toHaveBeenCalledWith("download-progress", handler);
  await events["download-progress"].emit({ percent: 100 });
  expect(emit).toHaveBeenCalledWith("download-progress", { percent: 100 });
});

// These assertions are checked by tsc and never executed.
function invalidCalls() {
  commands.directional({ payload: { incoming: "value" } });
  commands.echoJson({ value: { array: [null, true, 1, "value"] } });
  commands.nestedResult({ value: [{ Ok: { display_name: "Ada", score: 42 } }, { Err: { kind: "permission_denied" } }] });
  // @ts-expect-error JSON cannot contain a function.
  commands.echoJson({ value: () => {} });
  // @ts-expect-error Nested Result uses Serde's capitalized variant name.
  commands.nestedResult({ value: [{ ok: { display_name: "Ada", score: 42 } }] });
  // @ts-expect-error Rust's user_name is sent as userName.
  commands.lookup({ user_name: "Ada" });
  // @ts-expect-error An event payload must match its Rust definition.
  events["download-progress"].emit({ percent: "100" });
  // @ts-expect-error The input uses the deserialize-side field name.
  commands.directional({ payload: { outgoing: "value" } });
}
void invalidCalls;

const configured = await import("./generated/configured");

test("configured positional commands preserve status unions and transport failures", async () => {
  const profile = { display_name: "Ada", score: 42 };
  invoke.mockResolvedValueOnce(profile);
  expect(await configured.commands.lookup("Ada")).toEqual({status: "ok", data: profile});
  expect(invoke).toHaveBeenCalledWith("lookup", {userName: "Ada"});
  const failure = {kind: "not_found", detail: "missing"} as const;
  invoke.mockRejectedValueOnce(failure);
  expect(await configured.commands.lookup("missing")).toEqual({status: "error", error: failure});
  const transport = new Error("IPC failed");
  invoke.mockRejectedValueOnce(transport);
  await expect(configured.commands.lookup("Ada")).rejects.toBe(transport);
});

test("configured camelCase events retain their wire names", async () => {
  await configured.events.downloadProgress.emit({percent: 100});
  expect(emit).toHaveBeenCalledWith("download-progress", {percent: 100});
});
