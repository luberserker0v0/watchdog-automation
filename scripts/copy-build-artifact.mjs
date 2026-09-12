import { copyFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const [
  ,
  ,
  sourceName,
  destinationName,
  profile = "release",
  destinationDirectory = "bin",
] = process.argv;

if (!sourceName || !destinationName) {
  console.error(
    "Usage: node scripts/copy-build-artifact.mjs <source-name> <destination-name> [profile] [destination-directory]",
  );
  process.exitCode = 1;
} else {
  const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const outputDirectory = resolve(projectRoot, "src-tauri", "target", profile);
  const source = resolve(outputDirectory, sourceName);
  const destination = resolve(outputDirectory, destinationDirectory, destinationName);

  await mkdir(dirname(destination), { recursive: true });

  const retryableWindowsErrors = new Set(["EBUSY", "EPERM", "EACCES"]);
  const retryDelaysMs = [100, 250, 500, 1000];
  let copied = false;

  for (let attempt = 0; attempt <= retryDelaysMs.length; attempt++) {
    try {
      await copyFile(source, destination);
      copied = true;
      break;
    } catch (error) {
      const retryable = error && typeof error === "object"
        && "code" in error
        && retryableWindowsErrors.has(error.code);
      if (!retryable || attempt === retryDelaysMs.length) {
        if (retryable) {
          console.error(`Cannot replace ${destination}`);
          console.error(`${destinationName} is locked, usually because it is still running.`);
          console.error(`Close the ${destinationName} window/process, then run the build command again.`);
          console.error(`The newly compiled intermediate executable is available at ${source}`);
          process.exitCode = 1;
          break;
        }
        throw error;
      }
      await new Promise((resolveDelay) => setTimeout(resolveDelay, retryDelaysMs[attempt]));
    }
  }

  if (copied) {
    console.log(`Copied ${source} to ${destination}`);
  }
}
