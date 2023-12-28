fn main() {
    cc::Build::new()
        .cpp(true)
        .warnings(false)
        .define("OOZ_BUILD_DLL", "true")
        .files(&["ooz/bitknit.cpp", "ooz/kraken.cpp", "ooz/lzna.cpp"])
        .include("ooz/simde/")
        .compile("libooz");
}
