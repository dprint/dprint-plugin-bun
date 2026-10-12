/**
 * Checks out the commit of Bun that Cargo.toml names into `vendor/bun` and
 * prepares it for being built as a Wasm module.
 *
 * Bun's formatter is not written to be built on its own or for Wasm, so this:
 *
 *   1. points the crates of the system at the stand-ins in `shims/`,
 *   2. gates the compile time assertions of the layouts of the 64-bit targets,
 *   3. adds the entry points of the plugin to the driver of `bun format`,
 *   4. applies `patches/bun.patch`, and
 *   5. generates the source files that Bun's build generates.
 *
 * The first three are done by rule instead of by patch, so that they keep
 * working when Bun changes. To change the patch, edit the files in vendor/bun
 * and run `deno task patch`.
 */
import { $ } from "automation";

const rootDir = $.path(import.meta.dirname!).parentOrThrow();
const vendorDir = rootDir.join("vendor/bun");
const patchFile = rootDir.join("patches/bun.patch");
const stampFile = vendorDir.join(".dprint-plugin-bun");

const rev = getBunRev(rootDir.join("Cargo.toml").readTextSync());
const stamp = await getStamp(rev);

if (!Deno.args.includes("--force") && stampFile.existsSync() && stampFile.readTextSync() === stamp) {
  $.log(`vendor/bun is already at ${rev}.`);
  Deno.exit(0);
}

$.logStep(`Checking out Bun ${rev}...`);
await checkout(rev);

$.logStep("Pointing the crates of the system at the stand-ins...");
updateWorkspaceManifest();

$.logStep("Gating the layout assertions...");
$.logLight(`Gated ${gateLayoutAssertions()} assertions.`);

$.logStep("Adding the entry points of the plugin...");
addPluginEntryPoints();

// what the patch is a diff against: `deno task patch` writes the patch again
// from what has been changed in vendor/bun since
await $`git add --all`.cwd(vendorDir);
await $`git -c user.name=dprint-plugin-bun -c user.email=dprint-plugin-bun@localhost commit --quiet --message ${"What scripts/setup.ts does by rule"}`
  .cwd(vendorDir);

$.logStep("Applying patches/bun.patch...");
await $`git apply --whitespace=nowarn ${patchFile}`.cwd(vendorDir);

$.logStep("Generating the source files of Bun's build...");
await generateSources(rev);

stampFile.writeTextSync(stamp);
$.log("Done.");

function getBunRev(cargoTomlText: string) {
  const rev = cargoTomlText.match(/\[package\.metadata\.bun\]\s*version = "[^"]+"\s*rev = "([0-9a-f]{40})"/)?.[1];
  if (rev == null) {
    throw new Error("Could not find the rev of Bun in Cargo.toml.");
  }
  return rev;
}

/** What `vendor/bun` is made from: when it changes, the directory is made again. */
async function getStamp(rev: string) {
  const inputs = [rev, patchFile.readTextSync(), $.path(import.meta.filename!).readTextSync()];
  const hash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(inputs.join("\0")));
  return Array.from(new Uint8Array(hash)).map(byte => byte.toString(16).padStart(2, "0")).join("");
}

async function checkout(rev: string) {
  if (vendorDir.existsSync()) {
    await Deno.remove(vendorDir.toString(), { recursive: true });
  }
  vendorDir.mkdirSync({ recursive: true });
  const git = (...args: string[]) => $`git ${args}`.cwd(vendorDir).quiet("stdout");
  await git("init", "--quiet");
  await git("remote", "add", "origin", "https://github.com/oven-sh/bun");
  // the paths of some of Bun's tests are too long for Windows
  await git("config", "core.longpaths", "true");
  await git("config", "core.autocrlf", "false");
  // only the crates and the generators of the build are needed
  await git(
    "sparse-checkout",
    "set",
    "--no-cone",
    "/Cargo.toml",
    "/Cargo.lock",
    "/LICENSE.md",
    "/package.json",
    "/rust-toolchain.toml",
    "/src/",
    "/scripts/build/",
  );
  await git("fetch", "--quiet", "--depth", "1", "--filter=blob:none", "origin", rev);
  await git("checkout", "--quiet", "FETCH_HEAD");
}

function updateWorkspaceManifest() {
  const manifestFile = vendorDir.join("Cargo.toml");
  let text = manifestFile.readTextSync();
  text = replaceOnce(text, /^libc = .*$/m, `libc = { package = "bun_wasm_libc", path = "../../shims/libc" }`);
  text = replaceOnce(text, /^bun_sys = .*$/m, `bun_sys = { path = "../../shims/bun_sys" }`);
  // Bun denies warnings in its own crates, and code that is only unused on
  // Wasm is not something to fail on
  const lintsStart = text.indexOf("[workspace.lints.rust]");
  if (lintsStart === -1) {
    throw new Error("Could not find the lints in Bun's Cargo.toml.");
  }
  const lintsEnd = text.indexOf("\n[", lintsStart + 1);
  const lints = text.slice(lintsStart, lintsEnd).replace(/"deny"/g, `"allow"`);
  manifestFile.writeTextSync(text.slice(0, lintsStart) + lints + text.slice(lintsEnd));
}

/**
 * Bun asserts the sizes that its types have on a 64-bit target, which are
 * not the ones they have in Wasm. Returns how many assertions were gated.
 */
function gateLayoutAssertions() {
  const assertion = /^(\s*)(?:pub(?:\([a-z]+\))? )?const _[A-Za-z0-9_]*: \(\) = (?:assert!\(|\{|::core::assert!\()/;
  let count = 0;
  for (const file of rustFiles(vendorDir.join("src").toString())) {
    const lines = Deno.readTextFileSync(file).split("\n");
    const newLines: string[] = [];
    for (const line of lines) {
      const indent = line.match(assertion)?.[1];
      if (indent != null && !newLines.at(-1)?.includes("target_pointer_width")) {
        newLines.push(`${indent}#[cfg(target_pointer_width = "64")]`);
        count++;
      }
      newLines.push(line);
    }
    if (newLines.length !== lines.length) {
      Deno.writeTextFileSync(file, newLines.join("\n"));
    }
  }
  return count;
}

/**
 * The driver of `bun format` has what formats a file in private functions,
 * and only tells a caller that a file has a syntax error, not which one.
 */
function addPluginEntryPoints() {
  const driverFile = vendorDir.join("src/lint/driver/fmt/mod.rs");
  driverFile.writeTextSync(
    driverFile.readTextSync().trimEnd() + `

/// Whether the file at \`path\` is in a language that the formatter has. For dprint-plugin-bun.
pub fn is_formatted_for_plugin(path: &[u8], options: &FormatOptions) -> bool {
    Kind::with_options(path, options).is_some()
}

/// The text of the file at \`path\` formatted with \`options\`, the way \`bun format\` does it, or else why
/// the file is left as it is. For dprint-plugin-bun.
pub fn format_for_plugin(
    path: &[u8],
    text: &[u8],
    options: &FormatOptions,
) -> Result<Vec<u8>, Vec<u8>> {
    let resolved = Resolved {
        options: FormatOptions {
            format_javascript: Some(format_javascript(true)),
            parse_javascript: Some(parse_javascript),
            ..options.clone()
        },
        ..Resolved::default()
    };
    let names = Session::new();
    format(
        path,
        text,
        &resolved,
        (&Interner::new_in(&names), &names),
        &mut Scratches::default(),
        true,
    )
    .map(|(formatted, _)| formatted)
    .map_err(|failure| match failure {
        Failure::Syntax(message) | Failure::Unread(message) => message,
        Failure::Bug(what) | Failure::Loss(what) => what.as_bytes().to_vec(),
    })
}
`,
  );
}

/** Runs the generators of Bun's build that write what the crates include. */
async function generateSources(rev: string) {
  // where the build scripts of the crates look when nothing tells them otherwise
  const codegenDir = vendorDir.join("build/debug/codegen").toString();
  const importGenerator = (name: string) => import(vendorDir.join("scripts/build", name).toFileUrl().toString());
  const { generateBuildOptionsRs } = await importGenerator("buildOptionsRs.ts");
  const { generateJsonByteClass } = await importGenerator("jsonByteClass.ts");
  const { generateXmlByteClass } = await importGenerator("xmlByteClass.ts");
  generateBuildOptionsRs({
    codegenDir,
    version: JSON.parse(vendorDir.join("package.json").readTextSync()).version,
    revision: rev,
    // not something that the formatter reads
    nodejsVersion: "0.0.0",
    assertions: false,
    canary: false,
    canaryRevision: "0",
    fuzzilli: false,
    // the paths are only used by the runtime
    cwd: ".",
  });
  generateJsonByteClass({ codegenDir });
  generateXmlByteClass({ codegenDir });
  // the generated file would otherwise have the path of this machine in it
  const buildOptionsFile = $.path(codegenDir).join("build_options.rs");
  buildOptionsFile.writeTextSync(
    buildOptionsFile.readTextSync().replace(
      /^pub const CODEGEN_PATH: &\[u8\] = .*$/m,
      `pub const CODEGEN_PATH: &[u8] = ".".as_bytes();`,
    ),
  );
}

function* rustFiles(dir: string): Generator<string> {
  for (const entry of Deno.readDirSync(dir)) {
    const path = `${dir}/${entry.name}`;
    if (entry.isDirectory) {
      yield* rustFiles(path);
    } else if (entry.name.endsWith(".rs")) {
      yield path;
    }
  }
}

function replaceOnce(text: string, pattern: RegExp, replacement: string) {
  if (!pattern.test(text)) {
    throw new Error(`Could not find ${pattern} in Bun's Cargo.toml.`);
  }
  return text.replace(pattern, replacement);
}
