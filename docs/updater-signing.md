# Updater signing and key recovery

The updater key is independent from the Windows Authenticode certificate. Only
the public key belongs in `src-tauri/tauri.conf.json`.

## Generate the first key

Run this helper in an interactive terminal and enter a strong password at the
signer prompt:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\configure-updater-key.ps1
```

The helper copies the generated public key into `plugins.updater.pubkey`. Put
the complete private-key content and its password into protected GitHub secrets named
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## Back up and recover

Before publishing, place the encrypted private-key file and password in
separate protected recovery systems. Test restoration in a private release
candidate job. Never commit the key, password, temporary signing overlay,
certificate, or thumbprint.

Losing this key breaks the trust chain for every installed updater-enabled
version. Recovery then requires users to install a new build manually. Do not
rotate the key through `latest.json`; the existing application verifies that
manifest's artifact with its already embedded public key.

## Publish

Create a changelog section and make all version files agree. Push a matching
`vX.Y.Z` tag. The signed Windows workflow verifies Authenticode and the Tauri
signature before it publishes the installer, `.sig`, `latest.json`, portable
ZIP, and checksums. Manual workflow dispatch builds a signed candidate but does
not update GitHub Releases.
