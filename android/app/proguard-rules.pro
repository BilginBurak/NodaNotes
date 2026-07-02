# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile

# Ignore missing errorprone and javax annotations used by Tink
-dontwarn com.google.errorprone.annotations.**
-dontwarn javax.annotation.**
-dontwarn com.google.crypto.tink.**

# Keep the Rust JNI Bridge
-keep class com.bubi.nodanotes.RustCore { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep all serialization DTO models so JSON mapping doesn't break
-keep class com.bubi.nodanotes.data.model.** { *; }
-keepattributes *Annotation*,Signature,InnerClasses,EnclosingMethod

# Keep kotlinx.serialization Companion objects and annotated classes
-keepclassmembers class * {
    @kotlinx.serialization.Serializable *;
}
-keepclassmembers class * {
    *** Companion;
}