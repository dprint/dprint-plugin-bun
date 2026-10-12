// Runs a WASI module with Node.js, which is how `cargo test` runs the tests:
// the plugin is only made to build for Wasm, so its tests are Wasm as well.
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";

const [modulePath, ...args] = process.argv.slice(2);
const wasi = new WASI({
  version: "preview1",
  args: [modulePath, ...args],
  env: process.env,
  // the tests read their specs from the directory that cargo runs them in
  preopens: { ".": process.cwd() },
});
const module = await WebAssembly.compile(await readFile(modulePath));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
process.exitCode = wasi.start(instance);
