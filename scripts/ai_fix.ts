/**
 * Reconciles this plugin with a new Bun release using OpenAI Codex, in two
 * stages that each run Codex in a session of its own:
 *
 *   1. a Codex agentic session edits the source and runs the checks until they
 *      pass (fixing the patch and the stand-ins that make Bun's formatter
 *      build for Wasm, wiring up new formatter options, and/or adding support
 *      for languages Bun gained a formatter for), then
 *   2. a SEPARATE reviewer session reviews the result with Codex, so it can
 *      investigate (read the upstream Bun source, grep the repo). It is
 *      instructed to review only and not modify anything. If it finds blocking
 *      issues they are fed back to the stage-1 Codex for another pass, bounded
 *      by `REVIEW_MAX_ROUNDS`. If it still isn't approved, this throws so the
 *      workflow fails and nothing is published.
 *
 * Two situations call this:
 *   - an update that failed the setup or the checks (fix the breakage), and
 *   - any minor bump (review for new/renamed/removed options and newly
 *     supported languages even when it still builds).
 *
 * Both stages use the default model of the Codex CLI, which is installed at
 * its latest version on every run, so they move to a new model when Codex
 * does. Set `CODEX_MODEL` or `REVIEW_MODEL` to pin the model of a stage.
 *
 * Codex must NOT commit or push -- `update.ts` captures the working tree
 * changes into the existing Bun bump commit afterwards.
 */
import { $ } from "automation";

export interface AiFixOptions {
  /** true for a patch bump, false for a minor/major bump. */
  isPatchBump: boolean;
  /** Bun release that Cargo.toml had (before the bump). */
  fromVersion: string;
  /** Bun release being upgraded to. */
  toVersion: string;
  /** Whether the setup and the checks (test + clippy + wasm build) already passed. */
  checksPassed: boolean;
  /** Combined output of the failing checks (empty when `checksPassed`). */
  checkOutput: string;
}

/** Max number of times the reviewer may send changes back to Codex. */
const REVIEW_MAX_ROUNDS = 2;

export async function aiFixBunUpdate(options: AiFixOptions): Promise<void> {
  const apiKey = requireApiKey();
  await ensureCodexInstalled();
  await codexLogin(apiKey);

  // stage 1: let Codex reconcile the update.
  await runCodex(buildFixPrompt(options));

  // stage 2: independent second-model review, with a bounded refix loop.
  for (let round = 1;; round++) {
    const review = await reviewChanges(options);
    logReview(review);
    if (review.approved) {
      return;
    }
    if (round > REVIEW_MAX_ROUNDS) {
      const blocking = review.issues.filter((i) => i.severity === "blocking");
      throw new Error(
        `AI reviewer did not approve after ${REVIEW_MAX_ROUNDS} refix round(s). Blocking issues:\n`
          + blocking.map((i) => `  - ${i.description}`).join("\n"),
      );
    }
    $.logStep(`Reviewer requested changes — Codex refix round ${round}...`);
    await runCodex(buildRefixPrompt(review));
  }
}

// stage 1: Codex ---------------------------------------------------------------

async function runCodex(prompt: string): Promise<void> {
  const args = ["exec", "--dangerously-bypass-approvals-and-sandbox", "--skip-git-repo-check"];
  const model = Deno.env.get("CODEX_MODEL");
  if (model) {
    args.push("--model", model);
  }
  args.push(prompt);

  $.logStep("Running Codex...");
  await $`codex ${args}`;
}

function buildFixPrompt(options: AiFixOptions): string {
  const { isPatchBump, fromVersion, toVersion, checksPassed, checkOutput } = options;

  const situation = checksPassed
    ? `Bun was upgraded from ${fromVersion} to ${toVersion} (a ${
      isPatchBump ? "patch" : "minor"
    } bump). The setup and the checks pass, but a new Bun version may have ADDED, RENAMED, or REMOVED formatter options that should be surfaced by this plugin, or ADDED a formatter for a NEW LANGUAGE that this plugin should start formatting.`
    : `Bun was upgraded from ${fromVersion} to ${toVersion} and \`deno task setup\` or the checks fail. This is almost always because Bun's source changed in a way that (a) \`patches/bun.patch\` no longer applies, (b) a rule of \`scripts/setup.ts\` no longer finds what it changes, (c) Bun's crates name something that the stand-ins in \`shims/\` do not have, or call a C/C++ function that \`src/shims/\` does not have, or (d) new code in Bun's crates does not build for 32-bit Wasm.`;

  const failureOutput = checkOutput.trim().length > 0
    ? [
      ``,
      `The setup or the checks currently fail with the output below. Use it as your starting point instead of re-running them just to rediscover the errors:`,
      "```",
      truncateHead(checkOutput.trim()),
      "```",
    ]
    : [];

  return [
    `You are updating the "dprint-plugin-bun" Rust crate, a dprint plugin that builds the formatter of Bun (\`bun format\`, in \`src/format\` of https://github.com/oven-sh/bun) as a Wasm module to format: ${describeLanguages()}.`,
    ``,
    situation,
    ...failureOutput,
    ``,
    `Your goal: make the setup and the checks pass AND keep this plugin in sync with \`bun format\`, meaning (a) its configuration surface matches the formatting options of \`bun format\` that need nothing from the file system and (b) it formats every language that \`bun format\` formats itself. Do NOT commit or push; only edit files in the working tree.`,
    ``,
    describeWiring(),
    ``,
    `Bun's source at the new commit is in \`vendor/bun\` once \`deno task setup\` got as far as checking it out (the directory is not committed). Read:`,
    `- \`vendor/bun/src/format/CLAUDE.md\` for how the formatter is built up, and https://github.com/oven-sh/bun/blob/main/docs/runtime/format.mdx for what \`bun format\` does.`,
    `- \`vendor/bun/src/format/options.rs\` for \`FormatOptions\` and \`FormatOptions::set\`, which takes the names and values of Prettier's options, and compare them against \`src/options.rs\`.`,
    `- \`vendor/bun/src/lint/driver/fmt/mod.rs\` for the driver whose \`format\` function the plugin calls through the entry points that \`scripts/setup.ts\` appends to that file, and \`vendor/bun/src/lint/driver/fmt/files.rs\` for the names of the files of each language, and compare them against \`src/file_matching.rs\`.`,
    ``,
    `Rules:`,
    `1. If \`patches/bun.patch\` does not apply, apply what still applies (\`git -C vendor/bun apply --reject ../../patches/bun.patch\`), make the rejected changes by hand in \`vendor/bun\`, and run \`deno task patch\` to write the patch again. Keep the patch as small as possible, gate what is only for Wasm with \`#[cfg(target_family = "wasm")]\` or \`target_pointer_width\`, and never change what the formatter prints. \`deno task setup\` starts over from the patch, so only run it again once the patch is written.`,
    `2. If Bun's crates name something of \`bun_sys\` or \`libc\` that the stand-ins in \`shims/\` do not have, add it there with the signature of the real one (\`vendor/bun/src/sys\`). A stand-in for a system call fails; it does not do anything.`,
    `3. If the Wasm link fails with an undefined symbol, it is a function of a C or C++ library that Bun links. Add it to the module of that library in \`src/shims/\`, written in Rust with the exact behavior of the original (read its declaration in Bun's crates and its definition in \`src/jsc/bindings\` of Bun's repository on GitHub), with unit tests.`,
    `4. If an option was RENAMED or REMOVED in \`FormatOptions::set\`, update \`src/options.rs\` and the other layers. If a formatting option that needs nothing from the file system was ADDED, expose it across ALL of: \`configuration.rs\`, \`resolve_config.rs\`, \`options.rs\`, and \`deployment/schema.json\`, AND add a spec test under \`tests/specs/\` that exercises it. Use the name and the values that \`bun format\` (so Prettier) uses.`,
    `5. If \`bun format\` formats a NEW LANGUAGE itself, add its extensions to \`src/file_matching.rs\`, a spec under \`tests/specs/\`, a case in \`deployment/npm/index.test.js\`, the language to \`describeLanguages()\` in \`scripts/ai_fix.ts\`, and to the lists of languages in \`README.md\`, \`deployment/npm/README.md\` and the description in \`Cargo.toml\`.`,
    `6. Do NOT edit \`README.md\` except to add a newly supported language to its list of supported languages.`,
    `7. Preserve the existing code style. Keep non-test code above test modules. New comments start lowercase unless multiple sentences.`,
    `8. When done, ALL of these must pass (CI denies clippy warnings, and the wasm build is what actually ships) — iterate until they are all clean:`,
    `     deno task setup`,
    `     cargo test`,
    `     cargo clippy --all-targets --all-features -- -D warnings`,
    `     cargo build --target wasm32-unknown-unknown --features wasm --release`,
    `     (cd deployment/npm && npm install && node setup.js && npm run test)`,
    `   The last one runs the Wasm module that ships, which must not import anything: an import is a function that is missing in \`src/shims/\`.`,
    `9. Do not change the plugin's own version in Cargo.toml, do not run git commit, and do not push.`,
    ``,
    `Spec test format: each file in \`tests/specs/\` optionally starts with a \`-- file.ext --\` line that sets the file name (and so the language) of its specs, which defaults to \`file.ts\`. Then comes an optional \`~~ optionKey: value ~~\` config header line, then \`== short description ==\` (\`!! short description !!\` in a spec whose file name ends with \`.md\`), the input code, a line containing \`[expect]\`, and the expected formatted output. Add or update specs for options you add or rename (follow the existing files as examples and the \`Config<Name>.txt\` naming), delete the spec for any removed option, and run \`cargo test\` to confirm they pass.`,
  ].join("\n");
}

function buildRefixPrompt(review: ReviewResult): string {
  return [
    `An independent reviewer examined your changes to dprint-plugin-bun and found issues that must be fixed. Address every blocking issue below, keeping \`cargo test\` green. Do not commit or push.`,
    ``,
    `Reviewer summary: ${review.summary}`,
    ``,
    `Issues:`,
    ...review.issues.map((i) => `  - [${i.severity}] ${i.description}`),
    ``,
    describeWiring(),
  ].join("\n");
}

// the languages the plugin formats. Keep this in sync with
// `src/file_matching.rs` (the fixer is told to update it when adding a language).
function describeLanguages(): string {
  return "JavaScript/TypeScript/JSX/Flow, JSON/JSONC/JSON5, CSS/SCSS/Less, GraphQL, YAML, Markdown/MDX, HTML/Vue/MJML, and Handlebars. It does not format TOML, which `bun format` only formats for a project of oxfmt, and none of what `bun format` hands to installed packages (the languages of Prettier plugins, sorting imports with a Prettier plugin, the classes of Tailwind CSS)";
}

function describeWiring(): string {
  return [
    `How the plugin is wired (keep all of these consistent with each other):`,
    `- \`Cargo.toml\` -> \`[package.metadata.bun]\` has the release and the commit of Bun, and Bun's crates are path dependencies into \`vendor/bun\`, which \`scripts/setup.ts\` (\`deno task setup\`) checks out and prepares: it points Bun's \`libc\` and \`bun_sys\` at the stand-ins in \`shims/\`, gates the compile time layout assertions of the 64-bit targets, appends the entry points \`is_formatted_for_plugin\` and \`format_for_plugin\` to \`vendor/bun/src/lint/driver/fmt/mod.rs\`, commits that in \`vendor/bun\` as the base of the patch, applies \`patches/bun.patch\`, and runs the generators of Bun's build. \`rust-toolchain.toml\` has the nightly toolchain of Bun, which \`scripts/update.ts\` syncs.`,
    `- \`.cargo/config.toml\` -> cargo builds for \`wasm32-wasip1\` by default because Bun's crates only build for Wasm here, so \`cargo test\` runs the tests in Node.js (\`scripts/run_wasi.mjs\`). The plugin itself is built with \`--target wasm32-unknown-unknown\`. The linker limits the memory to 512 MiB because Bun keeps flags in the three highest bits of some pointers.`,
    `- \`shims/libc\` and \`shims/bun_sys\` -> crates that stand in for the \`libc\` crate and for Bun's crate of system calls in Bun's crates.`,
    `- \`src/shims/*.rs\` -> the functions of the C and C++ libraries that Bun's formatter calls (mimalloc, simdutf, Highway, WTF's number conversion, Yarr on top of the \`regress\` crate, ICU's collator, and a few hooks of Bun's runtime), written in Rust.`,
    `- \`src/format_text.rs\` -> \`format_text\` calls the entry points in Bun's driver, which decide by the name of the file whether and how it is formatted. An error is the message that \`bun format\` prints.`,
    `- \`src/options.rs\` -> \`build_format_options\` sets each option of the plugin's \`Configuration\` with \`FormatOptions::set\`, by the name and the value that Prettier has for it.`,
    `- \`src/file_matching.rs\` -> the extensions and the file names that the plugin tells dprint it formats.`,
    `- \`src/configuration/configuration.rs\` -> the plugin's own \`Configuration\` struct and enums.`,
    `- \`src/configuration/resolve_config.rs\` -> reads each dprint config key (camelCase, the names of Prettier's options) into \`Configuration\`, with dprint's global configuration as the default of \`printWidth\`, \`tabWidth\`, \`useTabs\` and \`endOfLine\`.`,
    `- \`deployment/schema.json\` -> the JSON schema of config options shown to users.`,
    `- \`tests/specs/*.txt\` -> spec tests (run by \`tests/test.rs\`, which \`build.rs\` lists them for) that exercise each language and config option.`,
    `- \`deployment/npm/index.test.js\` -> runs the Wasm module that ships.`,
  ].join("\n");
}

// keep the tail of long check output -- the errors and the "could not compile"
// summary are at the end, which is the most useful part for the AI.
function truncateHead(text: string, max = 20_000): string {
  return text.length > max ? `... (truncated)\n${text.slice(text.length - max)}` : text;
}

// stage 2: independent reviewer ------------------------------------------------

interface ReviewResult {
  approved: boolean;
  summary: string;
  issues: ReviewIssue[];
}

interface ReviewIssue {
  severity: "blocking" | "nit";
  description: string;
}

// The reviewer runs Codex too so it can actually investigate (read Bun's
// source in vendor/bun, grep the repo, verify option names against the real
// upstream API). Its verdict is captured as JSON via --output-last-message.
//
// NOTE: we deliberately do NOT use `--sandbox read-only` here. That makes Codex
// wrap commands in bubblewrap, which cannot set up its network namespace on the
// GitHub Actions runner ("bwrap: loopback: Failed RTM_NEWADDR: Operation not
// permitted"), so every command the reviewer runs fails before executing. We
// use the same no-sandbox mode as the fixer and enforce read-only via the
// prompt instead. (CI is itself a throwaway VM, and the final check gate in
// update.ts re-verifies whatever ends up in the working tree.)
async function reviewChanges(options: AiFixOptions): Promise<ReviewResult> {
  // the reviewer starts without the context of the fixer, which is what makes
  // it a second opinion. It uses the default model of Codex unless one is set.
  const model = Deno.env.get("REVIEW_MODEL");

  // record intent-to-add so any files Codex created also show in `git diff`.
  await $`git add -N .`.quiet();

  const outputFile = await Deno.makeTempFile({ prefix: "codex-review-", suffix: ".json" });
  try {
    $.logStep(`Reviewing changes with Codex (${model ?? "default model"})...`);
    const args = ["exec", "--skip-git-repo-check", "--dangerously-bypass-approvals-and-sandbox"];
    if (model) {
      args.push("--model", model);
    }
    args.push("--output-last-message", outputFile, buildReviewPrompt(options));
    await $`codex ${args}`;
    return parseReview(await Deno.readTextFile(outputFile));
  } finally {
    await Deno.remove(outputFile).catch(() => {});
  }
}

function buildReviewPrompt(options: AiFixOptions): string {
  return [
    `You are an independent reviewer for dprint-plugin-bun, a dprint plugin that builds the formatter of Bun (\`bun format\`) as a Wasm module to format: ${describeLanguages()}. Bun was just upgraded from ${options.fromVersion} to ${options.toVersion} and another AI edited this plugin to reconcile it. Review the UNCOMMITTED working-tree changes.`,
    `IMPORTANT: this is REVIEW ONLY. Read any files and run read-only commands (git diff, cat, grep, find, etc.) to investigate, but you MUST NOT edit, create, or delete any files, and MUST NOT run git commit or git push. Leave the working tree exactly as you found it.`,
    ``,
    `Investigate as needed:`,
    `- Run \`git --no-pager diff\` (and \`git status\`) to see exactly what changed, including any new files.`,
    `- Verify against the REAL Bun ${options.toVersion} source in \`vendor/bun\`: \`src/format/options.rs\` for \`FormatOptions::set\`, \`src/lint/driver/fmt/mod.rs\` for the driver, and \`src/lint/driver/fmt/files.rs\` for the files of each language. \`git -C vendor/bun diff\` is what \`patches/bun.patch\` has to be.`,
    `- Read the plugin files to confirm they are consistent with each other and with upstream.`,
    ``,
    describeWiring(),
    ``,
    `Verify specifically:`,
    `- \`patches/bun.patch\` only makes Bun's crates build for Wasm: it must not change what the formatter prints, and it is as small as it can be. A change that alters formatting is a blocking issue.`,
    `- What was added to \`shims/\` and \`src/shims/\` has the signature and the behavior of what it stands in for. A function in \`src/shims/\` that formatting can reach and that returns something else than the original would is a blocking issue.`,
    `- Every option that \`src/options.rs\` sets still exists in \`FormatOptions::set\` with that exact name and those values (catch removed/renamed options).`,
    `- Any formatting option newly added upstream that needs nothing from the file system is exposed through ALL layers: configuration.rs, resolve_config.rs, options.rs, and deployment/schema.json. A partial addition is a blocking issue.`,
    `- Every language that \`bun format\` ${options.toVersion} formats itself has its extensions in \`src/file_matching.rs\`, a spec under tests/specs/, and is named in \`describeLanguages()\` in scripts/ai_fix.ts.`,
    `- Every config option or language added or renamed in this change has a spec test under \`tests/specs/\` exercising it (and any removed option's spec is deleted). A missing spec test is a blocking issue.`,
    `- Naming conventions are consistent (Rust snake_case fields, camelCase dprint keys, matching the schema).`,
    `- README.md was NOT modified other than adding a newly supported language to its list of supported languages; flag any other README.md change as a blocking issue.`,
    `- No obvious correctness bugs, and code style matches the surrounding code.`,
    ``,
    `When done, your FINAL message must be ONLY a JSON object (no markdown code fences, no extra prose) of exactly this shape:`,
    `{"approved": true|false, "summary": "one sentence", "issues": [{"severity": "blocking"|"nit", "description": "..."}]}`,
    `Set approved=false if there is any blocking issue; nits alone do not block.`,
  ].join("\n");
}

function parseReview(message: string): ReviewResult {
  const review = JSON.parse(extractJsonObject(message)) as ReviewResult;
  if (typeof review.approved !== "boolean" || !Array.isArray(review.issues)) {
    throw new Error(`Codex review returned malformed JSON:\n${message}`);
  }
  return review;
}

// Codex is instructed to emit only JSON, but tolerate an accidental code fence
// or surrounding prose by extracting the outermost { ... } object.
function extractJsonObject(text: string): string {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start === -1 || end === -1 || end < start) {
    throw new Error(`Could not find a JSON object in Codex review output:\n${text}`);
  }
  return text.slice(start, end + 1);
}

function logReview(review: ReviewResult): void {
  $.log(`Review: ${review.approved ? "approved" : "changes requested"} — ${review.summary}`);
  for (const issue of review.issues) {
    $.logLight(`  [${issue.severity}] ${issue.description}`);
  }
}

// setup ------------------------------------------------------------------------

function requireApiKey(): string {
  const apiKey = Deno.env.get("OPENAI_API_KEY");
  if (!apiKey) {
    throw new Error("OPENAI_API_KEY is not set. It is required to run the AI fixer and reviewer.");
  }
  return apiKey;
}

async function ensureCodexInstalled(): Promise<void> {
  const found = await $`codex --version`.noThrow().quiet();
  if (found.code === 0) {
    return;
  }
  $.logStep("Installing OpenAI Codex CLI...");
  await $`npm install -g @openai/codex`;
}

// `codex exec` does not read OPENAI_API_KEY on its own -- it needs credentials
// stored via `codex login` first, otherwise its requests go out with no auth
// header and the API returns 401. The key is piped over stdin so it never
// appears in the process arguments.
async function codexLogin(apiKey: string): Promise<void> {
  $.logStep("Authenticating Codex with the OpenAI API key...");
  await $`codex login --with-api-key`.stdinText(apiKey);
}
