# @dprint/bun

npm distribution of [dprint-plugin-bun](https://github.com/dprint/dprint-plugin-bun) which is an adapter plugin for the formatter of [Bun](https://github.com/oven-sh/bun) that formats JavaScript, TypeScript, JSON, CSS, SCSS, Less, GraphQL, YAML, Markdown, MDX, HTML, Vue, and Handlebars.

Use this with [@dprint/formatter](https://github.com/dprint/js-formatter) or just use @dprint/formatter and download the [dprint-plugin-bun Wasm file](https://github.com/dprint/dprint-plugin-bun/releases).

## Example

```ts
import { getBuffer } from "@dprint/bun";
import { createFromBuffer } from "@dprint/formatter";

const formatter = createFromBuffer(getBuffer());

console.log(
  formatter.formatText("test.js", "console.log(  5  )"),
);
```
