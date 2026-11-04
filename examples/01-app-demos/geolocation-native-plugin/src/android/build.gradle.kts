import org.gradle.api.tasks.bundling.AbstractArchiveTask

plugins {
    id("com.android.library") version "9.4.0"
}

// dx bundles this module into its generated Android project; compile against the same
// SDK as the app (`compile_sdk` plus the newest installed platform minor and build-tools).
evaluationDependsOn(":app")
val app = project(":app").extensions.getByType<com.android.build.api.dsl.ApplicationExtension>()

android {
    namespace = "com.dioxus.geolocation"
    compileSdk = app.compileSdk
    compileSdkMinor = app.compileSdkMinor
    buildToolsVersion = app.buildToolsVersion

    defaultConfig {
        minSdk = 24
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
        getByName("debug") {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("com.google.android.gms:play-services-location:21.3.0")
}

tasks.withType<AbstractArchiveTask>().configureEach {
    archiveBaseName.set("geolocation-plugin")
}
