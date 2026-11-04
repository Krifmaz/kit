# android/

The kit's Android build (Gradle, Kotlin, Jetpack Compose): **kit-compose**, Encore's look and ease-of-use features as
a Compose library (DESIGN.md §11), plus a demo app and the Baseline Profile / Macrobenchmark setup every app copies.

| Module | What it is |
| --- | --- |
| [`kit-compose`](kit-compose/README.md) | The library, `dev.krifmaz.kit:kit-compose`. |
| [`demo`](demo/README.md) | A small app that shows every component (and the target of the profile and benchmarks). |
| [`baselineprofile`](baselineprofile/README.md) | Generates the demo's Baseline Profile and startup profile. |
| [`benchmark`](benchmark/README.md) | Macrobenchmark journeys (startup, scrolling). |

## Building

The versions are pinned in `gradle/libs.versions.toml` and equal Encore's, so an app that includes this build
gets one AGP and one Kotlin. No wrapper jar is committed (as in Encore): run the Gradle that
`gradle/wrapper/gradle-wrapper.properties` names.

```sh
gradle -p android :kit-compose:testDebugUnitTest :kit-compose:assembleRelease :demo:assembleDebug
ktlint "android/**/*.kt" "android/**/*.kts"       # style: android/.editorconfig
```

CI: `.github/workflows/android.yml` on every PR; `.github/workflows/android-publish.yml` publishes the library to
GitHub Packages for every `v*` release.

## Using it in an app

From GitHub Packages (reading needs a GitHub token with `read:packages`, even for a public package):

```kotlin
// settings.gradle.kts
dependencyResolutionManagement {
    repositories {
        maven {
            url = uri("https://maven.pkg.github.com/Krifmaz/kit")
            credentials { username = providers.gradleProperty("gpr.user").orNull; password = providers.gradleProperty("gpr.key").orNull }
        }
    }
}
// app/build.gradle.kts
dependencies { implementation("dev.krifmaz.kit:kit-compose:0.1.0") }
```

Or from source, with no token: check out the kit at a tag and `includeBuild("<kit>/android")` in the app's
`settings.gradle.kts`; Gradle then builds `dev.krifmaz.kit:kit-compose` from it (Encore does this, see
docs/DECISIONS.md).
