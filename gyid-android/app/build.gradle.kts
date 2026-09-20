plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.geoyuan.gyid"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.geoyuan.gyid"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    sourceSets {
        getByName("main") {
            // UniFFI 生成的 Kotlin 绑定与 cargo ndk 产出的 .so 直接引用，
            // 避免每次重新生成后手动拷贝（替代计划中的 copy 步骤）。
            // rootProject = gyid-android/，Rust crate 在其上级目录。
            // 绑定在 java srcDir 下同样会被 KGP 当 Kotlin 源编译。
            java.srcDir(rootProject.file("../gyid-android-rs/kotlin"))
            jniLibs.srcDirs(rootProject.file("../gyid-android-rs/jniLibs"))
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }
    packaging {
        resources.excludes += "/META-INF/{AL2.0,LGPL2.1}"
    }
}

dependencies {
    // UniFFI 0.28 Kotlin 绑定的 JNI/JNA 运行时（生成代码依赖 com.sun.jna）
    implementation("net.java.dev.jna:jna:5.14.0@aar")

    // Compose
    val composeBom = platform("androidx.compose:compose-bom:2024.09.03")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    debugImplementation("androidx.compose.ui:ui-tooling")
    implementation("androidx.activity:activity-compose:1.9.2")
    implementation("androidx.navigation:navigation-compose:2.8.2")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.6")

    // 定位（CollectService）
    implementation("com.google.android.gms:play-services-location:21.3.0")

    // Verifier HTTP / WebSocket
    implementation("com.squareup.okhttp3:okhttp:4.12.0")

    // seed 加密存储
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
}
