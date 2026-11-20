/// maps a lowercase file extension to a programming, markup, or style language
pub fn language(extension: &str) -> Option<&'static str> {
    let language = match extension {
        "adb" | "ads" => "Ada",
        "asm" | "s" => "Assembly",
        "astro" => "Astro",
        "bat" | "cmd" => "Batchfile",
        "c" | "h" => "C",
        "cs" => "C#",
        "cc" | "cpp" | "cxx" | "hh" | "hpp" | "hxx" => "C++",
        "clj" | "cljc" | "cljs" => "Clojure",
        "cbl" | "cob" => "COBOL",
        "lisp" => "Common Lisp",
        "cr" => "Crystal",
        "css" => "CSS",
        "dart" => "Dart",
        "ex" | "exs" => "Elixir",
        "elm" => "Elm",
        "el" => "Emacs Lisp",
        "erl" | "hrl" => "Erlang",
        "fs" | "fsi" | "fsx" => "F#",
        "f" | "f03" | "f90" | "f95" => "Fortran",
        "gleam" => "Gleam",
        "frag" | "glsl" | "vert" => "GLSL",
        "go" => "Go",
        "gradle" | "groovy" => "Groovy",
        "hs" => "Haskell",
        "hx" => "Haxe",
        "hlsl" => "HLSL",
        "htm" | "html" => "HTML",
        "java" => "Java",
        "cjs" | "js" | "jsx" | "mjs" => "JavaScript",
        "jl" => "Julia",
        "ipynb" => "Jupyter Notebook",
        "kt" | "kts" => "Kotlin",
        "less" => "Less",
        "lua" => "Lua",
        "mojo" => "Mojo",
        "nim" => "Nim",
        "nix" => "Nix",
        "m" | "mm" => "Objective-C",
        "ml" | "mli" => "OCaml",
        "odin" => "Odin",
        "pas" => "Pascal",
        "pl" | "pm" => "Perl",
        "php" => "PHP",
        "ps1" | "psm1" => "PowerShell",
        "py" | "pyi" => "Python",
        "r" => "R",
        "rkt" => "Racket",
        "rb" => "Ruby",
        "rs" => "Rust",
        "sass" => "Sass",
        "scala" => "Scala",
        "scm" => "Scheme",
        "scss" => "SCSS",
        "bash" | "fish" | "sh" | "zsh" => "Shell",
        "sol" => "Solidity",
        "sql" => "SQL",
        "svelte" => "Svelte",
        "swift" => "Swift",
        "sv" => "SystemVerilog",
        "tcl" => "Tcl",
        "cts" | "mts" | "ts" | "tsx" => "TypeScript",
        "vb" => "Visual Basic",
        "vhd" | "vhdl" => "VHDL",
        "vim" => "Vim Script",
        "vue" => "Vue",
        "wgsl" => "WGSL",
        "zig" => "Zig",
        _ => return None,
    };
    Some(language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_code_extensions() {
        assert_eq!(language("rs"), Some("Rust"));
        assert_eq!(language("tsx"), Some("TypeScript"));
        assert_eq!(language("h"), Some("C"));
        assert_eq!(language("hpp"), Some("C++"));
        assert_eq!(language("zsh"), Some("Shell"));
    }

    #[test]
    fn skips_docs_and_data() {
        for extension in [
            "md", "json", "yaml", "yml", "toml", "lock", "txt", "csv", "xml", "",
        ] {
            assert_eq!(language(extension), None, "{extension}");
        }
    }
}
