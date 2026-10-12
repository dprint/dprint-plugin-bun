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

Code embedded in these is formatted too (ex. CSS in a JS template literal, code blocks in Markdown, or the scripts and styles of an HTML or Vue file).

If you only want to use this plugin for some of these languages, list the plugins you want to format the other languages before this one in your dprint.json, since dprint formats a file with the first plugin that matches it.

## Install

[Install](https://dprint.dev/install/) and [setup](https://dprint.dev/setup/) dprint.

Then in your project's directory with a dprint.json file, run:

```shellsession
dprint config add bun
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

The options are the ones of `bun format`, which have the names and the values of [Prettier's options](https://prettier.io/docs/options). For an overview of the config, see https://dprint.dev/plugins/bun/config/

Note: The plugin does not read a `.prettierrc`, an `.oxfmtrc.json`, an `.editorconfig`, or a `bunfig.toml` because it runs sandboxed in a Wasm runtime—it has no access to the file system. For the same reason it has none of what `bun format` asks installed packages for: sorting imports with a Prettier plugin, sorting the classes of Tailwind CSS, and the languages of Prettier plugins (ex. Svelte and Astro).

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
- [shims](shims) has crates that stand in for the `libc` crate and for Bun's crate of system calls.
- [src/shims](src/shims) has the functions of the C and C++ libraries that Bun's formatter calls (mimalloc, simdutf, Highway, WebKit's number conversion and regular expression engine, and ICU's collator), written in Rust. The order of ICU's collator is only approximated: it is right for ASCII and the letters of Latin-1.

The Wasm module can use at most 512 MiB of memory (see [.cargo/config.toml](.cargo/config.toml)).
