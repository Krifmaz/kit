# baselineprofile

Generates the demo app's Baseline Profile and startup profile (`androidx.baselineprofile` 1.5.0): a cold start, then
scrolling the components (`BaselineProfileGenerator`). The profile lands in `demo/src/release/generated/` and ships in
the release APK; `profileinstaller` installs it.

```sh
gradle -p android :demo:generateBaselineProfile   # needs a connected device or emulator, API 28+
```

An app copies this module and `demo/build.gradle.kts`'s `baselineprofile` plugin and `baselineProfile(project(...))`
line, pointing `targetProjectPath` and `DEMO_PACKAGE` at itself. CI only builds it (no emulator).
