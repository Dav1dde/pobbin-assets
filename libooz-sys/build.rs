fn main() {
    cc::Build::new()
        .cpp(true)
        .warnings(false)
        .define("OOZ_BUILD_DLL", "true")
        .files(&[
            "ooz/bitknit.cpp",
            "ooz/compr_entropy.cpp",
            "ooz/compr_kraken.cpp",
            "ooz/compr_leviathan.cpp",
            "ooz/compr_match_finder.cpp",
            "ooz/compr_mermaid.cpp",
            "ooz/compr_multiarray.cpp",
            "ooz/compr_tans.cpp",
            "ooz/compress.cpp",
            "ooz/kraken.cpp",
            "ooz/lzna.cpp",
        ])
        .include("ooz/simde/")
        .compile("libooz");
}
