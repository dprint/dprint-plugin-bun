// @ts-check
const assert = require("assert");
const createFromBuffer = require("@dprint/formatter").createFromBuffer;
const getBuffer = require("./index").getBuffer;

assert.deepStrictEqual(WebAssembly.Module.imports(new WebAssembly.Module(getBuffer())), []);

const formatter = createFromBuffer(getBuffer());
const result = formatter.formatText({
  filePath: "file.js",
  fileText: "console.log (   5 )",
});

assert.strictEqual(result, "console.log(5);\n");

// ensure the other formatters work in the Wasm build
for (
  const [filePath, fileText, expected] of [
    ["file.json", "{\"a\":1}", "{ \"a\": 1 }\n"],
    ["file.css", "a{b:c}", "a {\n  b: c;\n}\n"],
    ["file.graphql", "{a}", "{\n  a\n}\n"],
    ["file.yaml", "a:   1", "a: 1\n"],
    ["file.md", "#   Title\n\n```js\nconst   a=1\n```\n", "# Title\n\n```js\nconst a = 1;\n```\n"],
    ["file.html", "<div><p>a</p></div>", "<div><p>a</p></div>\n"],
    ["file.vue", "<script>\nconst   a=1\n</script>\n", "<script>\nconst a = 1;\n</script>\n"],
    ["file.ts", "const a = css`a{b:c}`;", "const a = css`\n  a {\n    b: c;\n  }\n`;\n"],
    ["file.hbs", "<div   class=\"a\"></div>", "<div class=\"a\"></div>"],
    ["file.svelte", "<script>let a=1</script><p>{a}</p>", "<script>\n  let a = 1;\n</script>\n\n<p>{a}</p>\n"],
    // nested deep enough to need most of the stack
    ["file.js", "a(".repeat(500) + ")".repeat(500), null],
  ]
) {
  const actual = formatter.formatText({ filePath, fileText });
  if (expected != null) {
    assert.strictEqual(actual, expected, filePath);
  }
}

// the regular expressions and the collation that sorting imports needs
const sorting = createFromBuffer(getBuffer());
sorting.setConfig({}, {
  plugins: ["@ianvs/prettier-plugin-sort-imports"],
  importOrder: ["<THIRD_PARTY_MODULES>", "", "^[./]"],
});
assert.deepStrictEqual(sorting.getConfigDiagnostics(), []);
assert.strictEqual(
  sorting.formatText({ filePath: "file.ts", fileText: "import b from \"./b\";\nimport a from \"a\";\n" }),
  "import a from \"a\";\n\nimport b from \"./b\";\n",
);

// a syntax error is an error with where it is
assert.throws(
  () => formatter.formatText({ filePath: "file.ts", fileText: "const = ;" }),
  /error:[\s\S]*file\.ts:1:7/,
);
