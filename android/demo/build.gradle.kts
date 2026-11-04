// The kit demo app: every kit-compose part on one scrolling screen and the pages it opens, against a server kept in
// memory (DemoServer), so it needs no network. It is also the target of the Baseline Profile generator
// (:baselineprofile) and the Macrobenchmarks (:benchmark). See README.md.
plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.baselineprofile)
}

android {
    namespace = "dev.krifmaz.kit.demo"
    compileSdk = 36

    defaultConfig {
        applicationId = "dev.krifmaz.kit.demo"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = providers.gradleProperty("kitVersion").get()
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
            // the demo is never published: the debug key lets the benchmarks install the release build
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }
}

dependencies {
    implementation(project(":kit-compose"))
    // installs the Baseline Profile on devices without Play's cloud profiles (and for the benchmarks)
    implementation(libs.androidx.profileinstaller)
    baselineProfile(project(":baselineprofile"))

    testImplementation(libs.junit)
}
