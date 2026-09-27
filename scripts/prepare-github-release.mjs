import { readFile, writeFile, mkdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const args = new Map();
for (let index = 2; index < process.argv.length; index += 2) args.set(process.argv[index], process.argv[index + 1]);
const tag = args.get("--tag");
const installerArg = args.get("--installer");
const signatureArg = args.get("--signature");
const outputArg = args.get("--output") ?? "artifacts/latest.json";
if (!tag || !installerArg || !signatureArg) throw new Error("Usage: --tag vX.Y.Z --installer PATH --signature PATH [--output PATH]");

const packageJson = JSON.parse(await readFile(resolve(root, "package.json"), "utf8"));
if (tag !== `v${packageJson.version}`) throw new Error(`Tag ${tag} does not match package version v${packageJson.version}`);
const changelog = await readFile(resolve(root, "CHANGELOG.md"), "utf8");
const escaped = packageJson.version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const header = new RegExp(`^## \\[${escaped}\\][^\\n]*$`, "m");
const headerMatch = header.exec(changelog);
if (!headerMatch) throw new Error(`CHANGELOG.md has no notes for ${packageJson.version}`);
const sectionStart = headerMatch.index + headerMatch[0].length;
const remaining = changelog.slice(sectionStart).replace(/^\r?\n/, "");
const nextSection = remaining.search(/^## \[/m);
const notes = (nextSection < 0 ? remaining : remaining.slice(0, nextSection)).trim();
if (!notes) throw new Error(`CHANGELOG.md has no notes for ${packageJson.version}`);
const signature = (await readFile(resolve(root, signatureArg), "utf8")).trim();
if (!signature) throw new Error("Updater signature is empty");
const installerName = basename(installerArg);
const manifest = {
  version: packageJson.version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/luberserker0v0/watchdog-automation/releases/download/${tag}/${installerName}`,
    },
  },
};
const output = resolve(root, outputArg);
await mkdir(dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
await writeFile(resolve(dirname(output), "release-notes.md"), `${notes}\n`, "utf8");
const portable = resolve(root, `artifacts/watch-dog-${packageJson.version}-windows-x64.zip`);
const releaseFiles = [resolve(root, installerArg), resolve(root, signatureArg), portable];
const checksumLines = [];
for (const file of releaseFiles) {
  const hash = createHash("sha256").update(await readFile(file)).digest("hex");
  checksumLines.push(`${hash}  ${basename(file)}`);
}
await writeFile(resolve(dirname(output), "checksums.sha256"), `${checksumLines.join("\n")}\n`, "ascii");
console.log(`Prepared ${output} for ${tag}.`);
