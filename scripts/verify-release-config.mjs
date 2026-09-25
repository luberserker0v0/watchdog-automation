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
if (Array.isArray(productionResources) || productionResources?.["target/release/watch-dog-target.exe"] !== "watch-dog-target.exe") {
  errors.push("Production bundle must install the Sample Target companion beside watch-dog.exe");
}

if (!runtimeInstaller.includes('Microsoft.ML.OnnxRuntime.DirectML')) {
  errors.push("Runtime installer does not declare the ONNX Runtime DirectML package");
}
if (!runtimeInstaller.includes('Microsoft.AI.DirectML')) {
  errors.push("Runtime installer does not declare the DirectML dependency package");
}

for (const script of ["build:all-targets", "build:all-targets:debug", "build:installer", "package:windows", "package:windows:debug"]) {
  if (!packageJson.scripts?.[script]) errors.push(`package.json script '${script}' is missing`);
}

if (!packageJson.scripts?.["build:installer"]?.includes("build:target")) {
  errors.push("build:installer must build the Sample Target before creating the production installer");
}

if (errors.length) {
  console.error("Release configuration is invalid:");
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(`Release configuration valid for WatchDog ${packageJson.version}.`);
console.log("Production and Tester bundles stage application resources and ONNX Runtime DLLs.");
