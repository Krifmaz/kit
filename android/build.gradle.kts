// AGP 9 compiles Kotlin itself ("built-in Kotlin"), so org.jetbrains.kotlin.android is not applied anywhere.
// The classpath entry raises the Kotlin Gradle plugin AGP brings to the Kotlin pinned in gradle/libs.versions.toml
// (the Compose compiler plugin must match it exactly). An app that includes this build gets one AGP and one
// Kotlin: these.
buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.4.21")
    }
}

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.android.test) apply false
    alias(libs.plugins.baselineprofile) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.kotlin.serialization) apply false
}
