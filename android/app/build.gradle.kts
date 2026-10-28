import java.io.File

plugins {
    id("com.android.application")
}

android {
    namespace = "com.roland.earth"
    compileSdk = 35
    ndkVersion = "27.2.12479018"

    defaultConfig {
        applicationId = "com.roland.earth"
        minSdk = 28
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }

    androidResources {
        noCompress += listOf("dds")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

val buildRustAndroid = tasks.register("buildRustAndroid") {
    description = "Compiles the Rust earth library (libearth.so) for Android arm64-v8a"
    doLast {
        val projectRoot = rootProject.projectDir.parentFile
        val sdkDir = android.sdkDirectory
        val ndkDir = File(sdkDir, "ndk/${android.ndkVersion}")
        if (!ndkDir.exists()) {
            throw GradleException("Android NDK not found at ${ndkDir.absolutePath}")
        }
        val prebuiltDir = File(ndkDir, "toolchains/llvm/prebuilt")
        val hostDir = prebuiltDir.listFiles()?.firstOrNull { it.isDirectory }
            ?: throw GradleException("No prebuilt LLVM toolchain found in $prebuiltDir")
        val toolchainBin = File(hostDir, "bin")
        val clang = File(toolchainBin, "aarch64-linux-android28-clang")
        val ar = File(toolchainBin, "llvm-ar")

        val cargoBin = File(System.getProperty("user.home"), ".cargo/bin/cargo").let {
            if (it.exists()) it.absolutePath else "cargo"
        }

        val mobileElev = File(projectDir, "src/main/assets/earth/elevation.dds")
        val mobileDiff = File(projectDir, "src/main/assets/earth/diffuse.dds")
        if (!mobileElev.exists() || !mobileDiff.exists()) {
            val convertPb = ProcessBuilder(
                cargoBin,
                "run",
                "--release",
                "--bin",
                "convert_textures"
            )
            convertPb.directory(projectRoot)
            convertPb.inheritIO()
            val convertExit = convertPb.start().waitFor()
            if (convertExit != 0 || !mobileElev.exists() || !mobileDiff.exists()) {
                throw GradleException(
                    "Missing Android texture assets (${mobileElev.name}, ${mobileDiff.name}) and convert_textures failed (exit code $convertExit)"
                )
            }
        }

        val pb = ProcessBuilder(
            cargoBin,
            "build",
            "--release",
            "--lib",
            "--target",
            "aarch64-linux-android"
        )
        pb.directory(projectRoot)
        pb.inheritIO()
        val env = pb.environment()
        env["CC_aarch64_linux_android"] = clang.absolutePath
        env["AR_aarch64_linux_android"] = ar.absolutePath
        env["CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"] = clang.absolutePath
        env["CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS"] = "-C link-arg=-Wl,-z,max-page-size=16384"

        val exitCode = pb.start().waitFor()
        if (exitCode != 0) {
            throw GradleException("cargo build --target aarch64-linux-android failed with code $exitCode")
        }

        val soSrc = File(projectRoot, "target/aarch64-linux-android/release/libearth.so")
        val jniDstDir = File(projectDir, "src/main/jniLibs/arm64-v8a")
        jniDstDir.mkdirs()
        soSrc.copyTo(File(jniDstDir, "libearth.so"), overwrite = true)
    }
}

tasks.named("preBuild") {
    dependsOn(buildRustAndroid)
}
