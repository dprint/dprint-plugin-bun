/**
 * This script checks for any Bun updates and then automatically
 * publishes a new version of the plugin if so.
 */
import { $, CargoToml, semver } from "automation";
import { Octokit } from "octokit";
import { aiFixBunUpdate } from "./ai_fix.ts";

const rootDirPath = $.path(import.meta.dirname!).parentOrThrow();
const cargoToml = new CargoToml(rootDirPath.join("Cargo.toml"));
const cargoTomlVersion = getCargoTomlBunVersion(cargoToml.text());

$.logStep("Getting latest version...");
const latestTag = await getLatestTag();
if (semver.compare(latestTag.version, cargoTomlVersion.version) <= 0) {
  $.log("No new update found. Exiting.");
  Deno.exit(0);
}

$.log("Found new version.");

$.logStep("Updating rust-toolchain.toml...");
await updateRustToolchain(latestTag.tag);

$.logStep("Updating Cargo.toml...");
const isPatchBump = cargoTomlVersion.version.major === latestTag.version.major
  && cargoTomlVersion.version.minor === latestTag.version.minor;
const fromVersion = semver.format(cargoTomlVersion.version);
const toVersion = semver.format(latestTag.version);
cargoToml.replaceAll(
  `version = "${fromVersion}"\nrev = "${cargoTomlVersion.rev}"`,
  `version = "${toVersion}"\nrev = "${latestTag.rev}"`,
);

// Verify the update. A clean patch bump publishes exactly as before. A minor
// bump always gets an AI review (Bun may have added options without breaking
// the build), and a bump that fails the setup or the checks gets an AI fix
// attempt.
$.logStep("Running setup and checks (test + clippy + wasm build + npm test)...");
const checks = await runChecks();

if (!isPatchBump || !checks.passed) {
  if (checks.passed) {
    $.logStep("Minor Bun update — running AI review for new/changed options...");
  } else {
    $.logStep("Update failed the setup or the checks — running AI fix...");
  }
  await aiFixBunUpdate({
    isPatchBump,
    fromVersion,
    toVersion,
    checksPassed: checks.passed,
    // hand the failing output to the AI so it can go straight to fixing
    // instead of re-running the checks just to rediscover the errors.
    checkOutput: checks.output,
  });

  // the AI must leave the project in a passing state, otherwise fail the
  // workflow (nothing gets published and the maintainer is notified).
  $.logStep("Re-running setup and checks after AI changes...");
  await assertChecks();
}

if (Deno.args.includes("--skip-publish")) {
  Deno.exit(0);
}

$.logStep(`Committing bun version bump commit...`);
await $`git add .`;
const message = `${isPatchBump ? "fix" : "feat"}: update to Bun ${toVersion}`;
await $`git commit -m ${message}`;

$.logStep("Bumping version in Cargo.toml...");
// reload from disk before bumping: the AI may have edited Cargo.toml, and the
// in-memory copy loaded at startup is stale. Writing the stale copy here
// would clobber those edits.
const releaseCargoToml = new CargoToml(rootDirPath.join("Cargo.toml"));
releaseCargoToml.bumpCargoTomlVersion(isPatchBump ? "patch" : "minor");

// release
const newVersion = releaseCargoToml.version();
$.logStep(`Committing and publishing ${newVersion}...`);
await $`git add .`;
await $`git commit -m ${newVersion}`;
await $`git push origin main`;
await $`git tag ${newVersion}`;
await $`git push origin ${newVersion}`;

// the setup and the checks that must pass before publishing. clippy is
// included because Codex runs it with warnings denied, so a clippy warning is
// as breaking as a test failure, and the wasm release build and the test of
// the npm package are included because that is what actually ships.
// `inheritPiped` + `captureCombined` streams the output to the CI log live
// while also capturing it so a failure can be handed to the AI.
async function runChecks(): Promise<{ passed: boolean; output: string }> {
  const results = [];
  for (const command of checkCommands()) {
    const result = await capture(command());
    results.push(result);
    // nothing builds when Bun's source could not be prepared
    if (result.code !== 0 && results.length === 1) {
      break;
    }
  }
  const failures = results.filter((r) => r.code !== 0);
  return {
    passed: failures.length === 0,
    output: failures.map((r) => r.combined).join("\n\n"),
  };
}

function capture(command: ReturnType<typeof $>) {
  return command
    .stdout("inheritPiped")
    .stderr("inheritPiped")
    .captureCombined()
    .noThrow();
}

// same as `runChecks`, but throws on the first failure so the workflow
// aborts before anything is committed, tagged, or published.
async function assertChecks(): Promise<void> {
  for (const command of checkCommands()) {
    await command();
  }
}

// a function (not a top-level const) so it is hoisted -- `runChecks` is called
// from top-level code above before a const declared here would be initialized.
function checkCommands() {
  const npmDir = rootDirPath.join("deployment/npm");
  return [
    // checks out the new commit of Bun and prepares it
    () => $`deno run -A scripts/setup.ts`,
    () => $`cargo test`,
    () => $`cargo clippy --all-targets --all-features -- -D warnings`,
    () => $`cargo build --target wasm32-unknown-unknown --features wasm --release`,
    () => $`npm install && node setup.js && npm run test`.cwd(npmDir),
  ];
}

function getCargoTomlBunVersion(text: string) {
  const match = text.match(/\[package\.metadata\.bun\]\s*version = "([^"]+)"\s*rev = "([0-9a-f]{40})"/);
  if (match == null) {
    throw new Error("Could not find the version and the rev of Bun in Cargo.toml.");
  }
  $.logLight("Found version in Cargo.toml:", match[1]);
  return {
    version: semver.parse(match[1]),
    rev: match[2],
  };
}

async function getLatestTag() {
  const client = new Octokit();
  const tags = await client.paginate("GET /repos/{owner}/{repo}/tags", {
    owner: "oven-sh",
    repo: "bun",
    per_page: 100,
  });
  const versionWithTag = tags
    .filter(tag => /^bun-v[0-9]+\.[0-9]+\.[0-9]+$/.test(tag.name))
    .map(tag => ({ tag: tag.name, rev: tag.commit.sha, version: semver.parse(tag.name.replace(/^bun-v/, "")) }));
  versionWithTag.sort((a, b) => semver.compare(a.version, b.version));
  const latestTag = versionWithTag.at(-1);
  if (latestTag == null) {
    throw new Error("Could not find tag.");
  }
  $.logLight("Latest tag:", latestTag.tag);
  return latestTag;
}

// Bun's crates use unstable features, so they only build with Bun's toolchain
async function updateRustToolchain(tag: string) {
  const client = new Octokit();
  const response = await client.rest.repos.getContent({
    owner: "oven-sh",
    repo: "bun",
    path: "rust-toolchain.toml",
    ref: tag,
  });
  if (!("content" in response.data)) {
    throw new Error("Could not fetch rust-toolchain.toml from bun repo.");
  }
  const content = atob(response.data.content);
  const match = content.match(/channel\s*=\s*"([^"]+)"/);
  if (match == null) {
    throw new Error("Could not find channel in bun's rust-toolchain.toml.");
  }
  const bunChannel = match[1];
  const toolchainPath = rootDirPath.join("rust-toolchain.toml");
  const localContent = toolchainPath.readTextSync();
  const localMatch = localContent.match(/channel\s*=\s*"([^"]+)"/);
  if (localMatch == null) {
    throw new Error("Could not find channel in local rust-toolchain.toml.");
  }
  if (localMatch[1] !== bunChannel) {
    $.log(`Updating Rust toolchain: ${localMatch[1]} -> ${bunChannel}`);
    toolchainPath.writeTextSync(localContent.replace(localMatch[0], `channel = "${bunChannel}"`));
  } else {
    $.log(`Rust toolchain already at ${bunChannel}.`);
  }
}
