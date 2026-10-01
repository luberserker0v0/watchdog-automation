import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const readJson = async (path) => JSON.parse(await readFile(resolve(projectRoot, path), "utf8"));
const errors = [];

const packageJson = await readJson("package.json");
const production = await readJson("src-tauri/tauri.conf.json");
const tester = await readJson("src-tauri/tauri.conf.tester.json");
const target = await readJson("src-tauri/tauri.conf.target.json");
const cargo = await readFile(resolve(projectRoot, "src-tauri/Cargo.toml"), "utf8");
const runtimeInstaller = await readFile(resolve(projectRoot, "scripts/install-onnxruntime.ps1"), "utf8");
const signingScript = await readFile(resolve(projectRoot, "scripts/build-signed-windows.ps1"), "utf8");
const signingWorkflow = await readFile(resolve(projectRoot, ".github/workflows/release-windows-signed.yml"), "utf8");
const changelog = await readFile(resolve(projectRoot, "CHANGELOG.md"), "utf8");
const tagArgumentIndex = process.argv.indexOf("--tag");
const requestedTag = tagArgumentIndex >= 0 ? process.argv[tagArgumentIndex + 1] : undefined;

const cargoField = (name) => cargo.match(new RegExp(`^${name}\\s*=\\s*"([^"]+)"`, "m"))?.[1];
const versions = new Map([
  ["package.json", packageJson.version],
  ["Cargo.toml", cargoField("version")],
  ["production Tauri config", production.version],
  ["Tester Tauri config", tester.version],
  ["Target Tauri config", target.version],
]);
for (const [source, version] of versions) {
  if (version !== packageJson.version) errors.push(`${source} version ${version ?? "missing"} does not match ${packageJson.version}`);
}

if (packageJson.license !== "Apache-2.0") errors.push("package.json must declare Apache-2.0");
if (cargoField("license") !== "Apache-2.0") errors.push("Cargo.toml must declare Apache-2.0");

const pairedTauriPlugins = new Map([
  ["fs", "tauri-plugin-fs"],
  ["log", "tauri-plugin-log"],
  ["process", "tauri-plugin-process"],
  ["store", "tauri-plugin-store"],
  ["updater", "tauri-plugin-updater"],
]);
for (const [frontendName, rustName] of pairedTauriPlugins) {
  const frontendVersion = packageJson.dependencies?.[`@tauri-apps/plugin-${frontendName}`];
  const rustVersion = cargoField(rustName);
  if (!/^\d+\.\d+\.\d+$/.test(frontendVersion ?? "")) {
    errors.push(`@tauri-apps/plugin-${frontendName} must use an exact version`);
  }
  if (!/^=\d+\.\d+\.\d+$/.test(rustVersion ?? "")) {
    errors.push(`${rustName} must use an exact version`);
  }
  if (frontendVersion !== rustVersion?.slice(1)) {
    errors.push(`@tauri-apps/plugin-${frontendName} ${frontendVersion ?? "missing"} does not match ${rustName} ${rustVersion ?? "missing"}`);
  }
}

for (const [name, config] of [["production", production], ["Tester", tester], ["Target", target]]) {
  if (config.bundle?.license !== "Apache-2.0") errors.push(`${name} bundle must declare Apache-2.0`);
  if (config.bundle?.licenseFile !== "../LICENSE") errors.push(`${name} bundle must include ../LICENSE`);
  if (!config.bundle?.publisher) errors.push(`${name} bundle publisher is missing`);
  if (!config.bundle?.homepage) errors.push(`${name} bundle homepage is missing`);
}

const resourceSources = (config) => {
  const resources = config.bundle?.resources ?? [];
  return Array.isArray(resources) ? resources : Object.keys(resources);
};

for (const [name, config] of [["production", production], ["Tester", tester]]) {
  const resources = resourceSources(config);
  if (!resources.some((resource) => resource === "resources/" || resource === "resources/**/*")) {
    errors.push(`${name} bundle is missing application resources`);
  }
  if (!resources.includes("native/windows-x64")) errors.push(`${name} bundle is missing the native runtime resource directory`);
  if (!config.build?.beforeBuildCommand?.includes("install-onnxruntime.ps1")) {
    errors.push(`${name} build does not stage ONNX Runtime before bundling`);
  }
}

const productionResources = production.bundle?.resources;
if (Array.isArray(productionResources) || productionResources?.["companions/windows-x64/"] !== "") {
  errors.push("Production bundle must install the Sample Target companion beside watch-dog.exe");
}

if (!runtimeInstaller.includes('Microsoft.ML.OnnxRuntime.DirectML')) {
  errors.push("Runtime installer does not declare the ONNX Runtime DirectML package");
}
if (!runtimeInstaller.includes('Microsoft.AI.DirectML')) {
  errors.push("Runtime installer does not declare the DirectML dependency package");
}

for (const script of ["build:all-targets", "build:all-targets:debug", "build:installer", "package:windows", "package:windows:signed", "package:windows:debug"]) {
  if (!packageJson.scripts?.[script]) errors.push(`package.json script '${script}' is missing`);
}

if (!packageJson.scripts?.["build:installer"]?.includes("build:target")) {
  errors.push("build:installer must build the Sample Target before creating the production installer");
}
if (!packageJson.scripts?.["build:installer"]?.includes("stage-sample-target.ps1")) {
  errors.push("build:installer must stage the Sample Target into the production bundle resources");
}

for (const config of [production, tester, target]) {
  if (config.bundle?.windows?.certificateThumbprint) {
    errors.push("Ordinary Tauri configs must remain unsigned; use the temporary signing overlay");
  }
}
const updater = production.plugins?.updater;
if (updater?.endpoints?.[0] !== "https://github.com/luberserker0v0/watchdog-automation/releases/latest/download/latest.json") {
  errors.push("Production updater endpoint must use the stable GitHub latest.json feed");
}
if (!updater?.pubkey || updater.pubkey.includes("__WATCHDOG")) errors.push("Production updater public key is missing");
else {
  try {
    const publicDocument = Buffer.from(updater.pubkey, "base64").toString("utf8");
    const publicLine = publicDocument.trim().split(/\r?\n/).at(-1);
    if (!publicLine || Buffer.from(publicLine, "base64").length !== 42) {
      errors.push("Production updater public key has an invalid Minisign payload");
    }
  } catch {
    errors.push("Production updater public key is not a valid Tauri base64 wrapper");
  }
}
if (updater?.windows?.installMode !== "passive") errors.push("Windows updater install mode must be passive");
if (production.bundle?.createUpdaterArtifacts) errors.push("Ordinary production config must not create updater artifacts");
for (const required of [
  "WINDOWS_CERTIFICATE_BASE64",
  "WINDOWS_CERTIFICATE_PASSWORD",
  "WINDOWS_TIMESTAMP_URL",
  "TAURI_SIGNING_PRIVATE_KEY",
  "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
  "createUpdaterArtifacts",
  "FromBase64String($signatureWrapper)",
  "minisign",
  "Get-AuthenticodeSignature",
  "finally",
  "watch-dog.exe",
  "watch-dog-tester.exe",
  "watch-dog-target.exe",
]) {
  if (!signingScript.includes(required)) errors.push(`Signed build script is missing '${required}'`);
}
if (!signingScript.includes("Authenticode is optional")) errors.push("Signed build must permit updater-only signing when Authenticode is not configured");
if (signingScript.includes("npm run build:all-targets")) errors.push("Signed build must not compile production before its bundling build");
for (const command of ["npm run build:tester", "npm run build:target"]) {
  if (!signingScript.includes(command)) errors.push(`Signed build is missing '${command}'`);
}
if (!signingWorkflow.includes("Swatinem/rust-cache@v2")) errors.push("Signed workflow must cache Rust dependencies and build output");
if (!signingWorkflow.includes("cache-on-failure: true")) errors.push("Signed workflow must preserve the Rust cache after late release failures");
if (!signingWorkflow.includes("src-tauri/.cache/native-runtime")) errors.push("Signed workflow must cache downloaded native runtimes");
if (!signingWorkflow.includes("if: github.event_name == 'workflow_dispatch'")) errors.push("Tag publishing must not consume Actions artifact storage");
if (!signingWorkflow.includes('tags:') || !signingWorkflow.includes('"v*"')) errors.push("Signed workflow must publish from v* tags");
if (!signingWorkflow.includes("gh release create")) errors.push("Signed workflow must publish GitHub Release assets");
if (!signingWorkflow.includes("secrets.TAURI_SIGNING_PRIVATE_KEY")) errors.push("Signed workflow must read the updater private key secret");
if (!signingWorkflow.includes("secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD")) errors.push("Signed workflow must read the updater key password secret");
if (requestedTag) {
  if (requestedTag !== `v${packageJson.version}`) errors.push(`Release tag ${requestedTag} must equal v${packageJson.version}`);
  const escaped = packageJson.version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  if (!new RegExp(`^## \\[${escaped}\\]`, "m").test(changelog)) errors.push(`CHANGELOG.md is missing a ${packageJson.version} release section`);
}

if (errors.length) {
  console.error("Release configuration is invalid:");
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(`Release configuration valid for WatchDog ${packageJson.version}.`);
console.log("Production and Tester bundles stage application resources and ONNX Runtime DLLs.");
console.log("The release workflow requires Tauri updater signatures and applies Authenticode when its three settings are configured.");
