# dprint-plugin-bun

[![npm version](https://img.shields.io/npm/v/@dprint/bun.svg)](https://www.npmjs.com/package/@dprint/bun) [![CI](https://github.com/dprint/dprint-plugin-bun/workflows/CI/badge.svg)](https://github.com/dprint/dprint-plugin-bun/actions?query=workflow%3ACI)

Adapter for the formatter of [Bun](https://github.com/oven-sh/bun) (`bun format`) for use as a formatting plugin in [dprint](https://github.com/dprint/dprint).

`bun format` prints what [Prettier](https://prettier.io) prints, so this plugin is a way to get Prettier's formatting in dprint from a Wasm plugin.

## Supported languages

- JavaScript and TypeScript (including JSX and Flow)
- JSON, JSONC, and JSON5
- CSS, SCSS, and Less
- GraphQL
- YAML
- Markdown and MDX
- HTML, Vue, and MJML
- Handlebars
- TOML (with the flavor `"oxfmt"`)

Code embedded in these is formatted too (ex. CSS in a JS template literal, code blocks in Markdown, or the scripts and styles of an HTML or Vue file).

If you only want to use this plugin for some of these languages, [disable the other languages](#disabling-languages).

## Install

[Install](https://dprint.dev/install/) and [setup](https://dprint.dev/setup/) dprint.

Then in your project's directory with a dprint.json file, run:

```shellsession
dprint add bun
```

Note: You do not need Bun installed as dprint will run Bun's formatter from the .wasm file in a sandboxed environment.

## Configuration

To add configuration, specify a `"bun"` key in your dprint.json:

```jsonc
{
  "bun": {
    "printWidth": 100,
    "semi": false,
    "singleQuote": true,
  },
  "plugins": [
    // ...etc...
  ],
}
```

The options are the ones that `bun format` reads in a configuration file, with the same names and values: [Prettier's options](https://prettier.io/docs/options), or with `"flavor": "oxfmt"` the ones of an `.oxfmtrc.json`. For an overview of the config, see https://dprint.dev/plugins/bun/config/

### Disabling languages

Every language is formatted by default. To have the plugin leave the files of a language alone, set its `<language>.enabled` option to `false`:

```jsonc
{
  "bun": {
    "markdown.enabled": false,
    "yaml.enabled": false,
  },
}
```

The languages are `javascript`, `typescript`, `json`, `jsonc`, `json5`, `css`, `scss`, `less`, `yaml`, `markdown`, `mdx`, `graphql`, `handlebars`, `html`, `vue`, `mjml`, and `toml`. JSX is part of `javascript` and TSX is part of `typescript`. TOML is only formatted with `"flavor": "oxfmt"`, whatever `toml.enabled` is.

A disabled language's files are no longer matched by this plugin, so dprint formats them with the next plugin that matches them, if any. This only decides which files are formatted: code embedded in a file of an enabled language is still formatted (ex. CSS in a JS template literal when `css.enabled` is `false`).

Note: The plugin does not read a `.prettierrc`, an `.oxfmtrc.json`, an `.editorconfig`, a `tsconfig.json`, or a `bunfig.toml` because it runs sandboxed in a Wasm runtime—it has no access to the file system.

### Sorting imports

As in `bun format`, the three plugins of Prettier that sort imports are built in. Name one in `plugins` to sort like it, with its options:

```jsonc
{
  "bun": {
    "plugins": ["@ianvs/prettier-plugin-sort-imports"],
    "importOrder": [
      "<BUILTIN_MODULES>",
      "",
      "<THIRD_PARTY_MODULES>",
      "",
      "^@/(.*)$",
      "^[./]"
    ],
  },
}
```

- `@trivago/prettier-plugin-sort-imports` and `@ianvs/prettier-plugin-sort-imports`, with the `importOrder*` options.
- `prettier-plugin-organize-imports`, which also removes unused imports, with the `organizeImports*` options. It reads the JSX settings of a `tsconfig.json`, which the plugin cannot do, so set them as `tsconfig.jsx`, `tsconfig.jsxFactory`, `tsconfig.jsxFragmentFactory` and `tsconfig.reactNamespace` if a project does not use the defaults.

### The flavor of oxfmt

With `"flavor": "oxfmt"` the plugin prints what [oxfmt](https://oxc.rs/docs/guide/usage/formatter) prints where that is not what Prettier prints, and has the options of oxfmt: lines are 100 wide by default, `package.json` files are sorted (`sortPackageJson`), TOML is formatted, and `sortImports`, `jsdoc` and `insertFinalNewline` are available.

```jsonc
{
  "bun": {
    "flavor": "oxfmt",
    "sortImports": true,
    "jsdoc": true,
  },
}
```

### What is not there

These need the file system or JavaScript packages, which a Wasm plugin has neither of:

- `overrides`. Use more than one configuration of dprint instead.
- `sortTailwindcss`, which asks the `tailwindcss` package of the project for the order of the classes.
- Plugins of Prettier other than the three above, and so the languages that only a plugin reads (ex. Svelte and Astro).

## JS Formatting API

- [JS Formatter](https://github.com/dprint/js-formatter) - Browser/Deno and Node
- [npm package](https://www.npmjs.com/package/@dprint/bun)

## Versioning

This repo automatically upgrades to the latest version of Bun once a day. You can check which version of Bun is being used by looking at the `[package.metadata.bun]` table of the Cargo.toml file in this repo:

https://github.com/dprint/dprint-plugin-bun/blob/main/Cargo.toml

At the moment, the version of this plugin does not reflect the version of Bun.

## Development

Bun's formatter is not published as a crate and is not made to be built on its own or for Wasm. So this repo checks out Bun's source at the pinned commit into `vendor/bun` and prepares it for that (see [scripts/setup.ts](scripts/setup.ts)). This needs [Deno](https://deno.com), and the tests need [Node.js](https://nodejs.org):

```shellsession
# check out and prepare Bun's source (again after the `rev` or the patch changes)
deno task setup
# the tests are built for Wasm and run in Node.js
cargo test
# build the plugin
cargo build --target wasm32-unknown-unknown --features wasm --release
```

What makes Bun's formatter build for Wasm:

- [patches/bun.patch](patches/bun.patch) has the changes to Bun's source that are not made by rule. To change it, edit the files in `vendor/bun` and run `deno task patch`.
- [patches/entry_points](patches/entry_points) has the functions that the plugin calls, which are added to the modules of the driver of `bun format` because what they call is private there.
- [shims](shims) has crates that stand in for the `libc` crate and for Bun's crate of system calls.
- [src/shims](src/shims) has the functions of the C and C++ libraries that Bun's formatter calls (mimalloc, simdutf, Highway, WebKit's number conversion and regular expression engine, and ICU's collator), written in Rust. The order of ICU's collator, which sorting imports uses, is only approximated: it is right for ASCII and the letters of Latin-1.

The Wasm module can use at most 512 MiB of memory (see [.cargo/config.toml](.cargo/config.toml)).
