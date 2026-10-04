buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:9.3.1")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.2.10")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
        exclusiveContent {
            forRepository { maven { url = uri("https://jitpack.io") } }
            filter { includeGroup("com.github.topjohnwu.libsu") }
        }
    }
}

tasks.register("clean").configure {
    delete("build")
}

