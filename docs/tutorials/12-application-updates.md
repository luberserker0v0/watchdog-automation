# 12. Application updates

Production WatchDog can check stable GitHub Releases after startup. Tester,
Splash, Preview, and Sample Target cannot invoke the updater.

## 12.1 Check for an update

Open **Settings → Application updates** and select **Check for updates**.
Automatic checks run once after startup by default and can be disabled without
removing the manual action. Connection or feed errors stay in this Settings
card and the application log.

When an update is available, a banner remains above the workspace. Settings
shows the installed and offered versions, publication date, plain-text release
notes, and download progress.

## 12.2 Install safely

Finish or stop every pipeline and stop Action Recorder. Select **Install and
restart**, then confirm. WatchDog blocks new pipeline and recorder starts,
stops a manual capture stream, verifies the update, and starts the passive
Windows installer. A download or install failure releases the guard so the
update can be retried.

Portable builds use the same NSIS update and therefore become normal installed
applications after acceptance.

## 12.3 Trust and recovery

Tauri updater signatures verify that the artifact belongs to WatchDog's update
key. Authenticode separately verifies its Windows publisher. A release needs
both. If the updater private key is lost, installed copies cannot accept a new
key through the trusted update channel. Release operators must keep an
encrypted backup before publishing the first updater-enabled version. See
[Updater signing and recovery](../updater-signing.md).
