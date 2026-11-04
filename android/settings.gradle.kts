// Krifmaz kit – the Android build: android/kit-compose (DESIGN.md §11). A Gradle build of its own (the rest of the
// kit is Rust). Apps use it as dev.krifmaz.kit:kit-compose (GitHub Packages), or as an included build
// (includeBuild("<kit>/android")), which Gradle substitutes for that dependency.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.PREFER_SETTINGS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "krifmaz-kit-android"
include(":kit-compose")
// the demo app (every component on one screen) and its Baseline Profile generator and Macrobenchmarks; only when
// this build runs on its own: an app that includes it (includeBuild) needs just the library
if (gradle.parent == null) {
    include(":demo")
    include(":baselineprofile")
    include(":benchmark")
}
