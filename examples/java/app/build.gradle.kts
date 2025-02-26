plugins {
    application
}

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(libs.junit)
    implementation(libs.guava)

    // how to add statsig java core as dependency
    implementation("com.statsig:javacore:0.0.3") // Platform-independent Java core, ALWAYS ADD THIS
    implementation("com.statsig:javacore:0.0.3:macos-arm64") // add this if you are on macOS (Apple Silicon/ARM64)

    // see all supported OS and Arch combinations of installation here:
    // https://docs.statsig.com/server-core/java-core#faq
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(11)
    }
}

application {
    mainClass = "org.example.App"
}
