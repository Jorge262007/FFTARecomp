plugins {
    alias(libs.plugins.android.application)
    id("org.jetbrains.kotlin.android")
}

// Wear OS front end for the same recompiled core as :app. The engine sources and the native
// libraries (libgba_android.so + libtcc.so from scripts/build_android.py) are shared with
// :app, never copied into this module: the watch runs the exact code the phone runs.
val shared = listOf(
    "NativeBridge.kt", "GameSurface.kt", "FrameMailbox.kt", "VideoPass.kt", "VideoOptions.kt",
    "NormalAudioOutput.kt", "GameBenchmark.kt", "TouchSelection.kt",
)
val sharedEngine = layout.buildDirectory.dir("generated/shared-engine")
val syncSharedEngine = tasks.register<Sync>("syncSharedEngine") {
    from(file("../app/src/main/java/com/fftarecomp/port")) { include(shared) }
    into(sharedEngine.map { it.dir("com/fftarecomp/port") })
}

android {
    // Same namespace as :app so the JNI symbols (Java_com_fftarecomp_port_NativeBridge_*)
    // and BuildConfig resolve; the installed package is a separate application id.
    namespace = "com.fftarecomp.port"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.fftarecomp.port.wear"
        minSdk = 30
        targetSdk = 35
        versionCode = 37
        versionName = "3.1-wear"
        // TinyCC emits ARM64 at runtime, so the watch must run a 64-bit userspace.
        ndk { abiFilters += "arm64-v8a" }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("debug")
        }
    }
    sourceSets {
        getByName("main") {
            java.srcDir(sharedEngine)
            jniLibs.srcDir("../app/src/main/jniLibs")
            assets.srcDir("../app/src/main/assets")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    buildFeatures { buildConfig = true }
    packaging {
        jniLibs { useLegacyPackaging = true }
    }
}

tasks.named("preBuild") { dependsOn(syncSharedEngine) }

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>().configureEach {
    dependsOn(syncSharedEngine)
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_11)
    }
}

// Debug convenience: a ROM placed in wear/dev-rom/ (git-ignored) is bundled into debug APKs
// and imported on first launch. Build with -PwearNoRom to get the Wi-Fi/QR transfer instead.
val devRom = layout.buildDirectory.dir("generated/dev-rom")
val syncDevRom = tasks.register<Sync>("syncDevRom") {
    from(file("dev-rom")) { include("*.gba", "*.GBA"); into("devrom") }
    into(devRom)
}
if (!project.hasProperty("wearNoRom")) {
    android.sourceSets.getByName("debug").assets.srcDir(devRom)
    tasks.named("preBuild") { dependsOn(syncDevRom) }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("com.google.zxing:core:3.5.3")
}
