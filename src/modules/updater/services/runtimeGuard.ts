let updateInstallationActive = false;

export function activateUpdateInstallationGuard(): boolean {
  if (updateInstallationActive) return false;
  updateInstallationActive = true;
  return true;
}

export function releaseUpdateInstallationGuard(): void {
  updateInstallationActive = false;
}

export function assertRuntimeStartAllowed(): void {
  if (updateInstallationActive) {
    throw new Error("An application update is being installed. New work cannot start.");
  }
}

export function isUpdateInstallationActive(): boolean {
  return updateInstallationActive;
}
