// node --test

import assert from "node:assert";
import { test } from "node:test";

import { WasmPlugin } from "./jsoncodegen-wasm32-wasip1.ts";

// cargo run --bin wasm-serve -- target/wasm32-wasip1/wasm/jsoncodegen*.wasm --port 7357
const wasmServer = "http://localhost:7357/";

// list of wasm files that are served by the wasm-server is listed at "/"
const response = await fetch(wasmServer);
const filenames = await response.json();

for (const filename of filenames) {
  const url = new URL(filename, wasmServer);

  test(url.href, async () => {
    const plugin = await WasmPlugin.load(url);
    const output = plugin.run("{}");
    assert.ok(output, "Plugin should return a string");
  });
}
