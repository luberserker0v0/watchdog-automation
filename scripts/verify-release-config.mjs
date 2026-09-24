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

for (const [name, config] of [["production", production], ["Tester", tester]]) {
  const resources = config.bundle?.resources ?? [];
  if (!resources.includes("resources/**/*")) errors.push(`${name} bundle is missing application resources`);
  if (!resources.includes("native/windows-x64/*.dll")) errors.push(`${name} bundle is missing native runtime DLL resources`);
  if (!config.build?.beforeBuildCommand?.includes("install-onnxruntime.ps1")) {
    errors.push(`${name} build does not stage ONNX Runtime before bundling`);
  }
}

for (const script of ["build:all-targets", "build:all-targets:debug", "package:windows", "package:windows:debug"]) {
  if (!packageJson.scripts?.[script]) errors.push(`package.json script '${script}' is missing`);
}

if (errors.length) {
  console.error("Release configuration is invalid:");
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(`Release configuration valid for WatchDog ${packageJson.version}.`);
console.log("Production and Tester bundles stage application resources and ONNX Runtime DLLs.");
