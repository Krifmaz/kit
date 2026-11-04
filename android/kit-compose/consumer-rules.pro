# R8 rules an app gets with kit-compose. The kit's own @Serializable models (bug reports, schema, changelog…) are
# read with kotlinx.serialization: keep their generated serializers (the library's README rules, for the kit's package).
-keepattributes RuntimeVisibleAnnotations,AnnotationDefault,InnerClasses,Signature
-keep,includedescriptorclasses class dev.krifmaz.kit.compose.**$$serializer { *; }
-keepclassmembers class dev.krifmaz.kit.compose.** {
    *** Companion;
}
-keepclasseswithmembers class dev.krifmaz.kit.compose.** {
    kotlinx.serialization.KSerializer serializer(...);
}
