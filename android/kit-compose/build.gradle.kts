// kit-compose: the kit's Jetpack Compose library for Android apps (DESIGN.md §11). See README.md.
plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
    `maven-publish`
}

group = "dev.krifmaz.kit"
version = providers.gradleProperty("kitVersion").get()

android {
    namespace = "dev.krifmaz.kit.compose"
    compileSdk = 36

    defaultConfig {
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }

    // The theme is generated from design/tokens.json (design/out/KitTokens.kt); compile it from there, so the
    // library can't drift from the tokens.
    sourceSets {
        getByName("main") {
            kotlin.srcDir("../../design/out")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
    }

    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }
}

dependencies {
    api(platform(libs.androidx.compose.bom))
    api(libs.androidx.compose.ui)
    api(libs.androidx.compose.foundation)
    api(libs.androidx.compose.material3)
    api(libs.androidx.activity.compose)
    api(libs.okhttp)
    api(libs.kotlinx.serialization.json)
    api(libs.kotlinx.coroutines.android)
    api(libs.coil.compose)
    api(libs.coil.network.okhttp)
    api(libs.coil.network.cache.control)
    api(libs.androidx.metrics.performance)

    testImplementation(libs.junit)
}

publishing {
    publications {
        register<MavenPublication>("release") {
            groupId = "dev.krifmaz.kit"
            artifactId = "kit-compose"
            afterEvaluate { from(components["release"]) }
            pom {
                name.set("kit-compose")
                description.set("The kit's look and ease-of-use features for Jetpack Compose apps.")
                url.set("https://github.com/Krifmaz/kit")
                licenses {
                    license {
                        name.set("MIT")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }
            }
        }
    }
    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/Krifmaz/kit")
            credentials {
                username = System.getenv("GITHUB_ACTOR")
                password = System.getenv("GITHUB_TOKEN")
            }
        }
    }
}
