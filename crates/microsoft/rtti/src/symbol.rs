use regex::Regex;
use msvc_demangler::demangle;

/// TODO: write own decorated symbol name parser
/// WARNING: hacky
/// This function attempts to unmangle RTTI symbol names and does it in a horribly hacky way.
/// This fn relies on `UnDecorateSymbol` indirectly so needs a microsoft machine to run on.
pub fn undecorate_symbol(input: impl AsRef<str>) -> Option<String> {
    let input = input.as_ref();

    let name = if input.starts_with(".?A") {
        format!("??1{}QAE@XZ", &input[4..]).to_string()
    } else {
        return None;
    };

    // Strip garbage off of the beginning
    let re = Regex::new("^(?:(public|private): (?:.+? )?__.+? )").unwrap();
    let flags = msvc_demangler::DemangleFlags::llvm();
    let demangled = demangle(name.as_str(), flags);
    if demangled.is_err() {
        return None;
    }

    let cleaned = re.replace(demangled.unwrap().as_str(), "").to_string();

    // Find the destructor def and remove it
    let re = Regex::new("::~(.*)$").unwrap();
    let mat = re.find(cleaned.as_str());
    mat?;

    let mat = mat.unwrap();
    Some(cleaned[..mat.start()].to_string())
}

/// Makes an educated guess as to whether or not some string is a mangled symbol name.
/// Not a 100% accurate but should suffice for the time being.
pub fn is_decorated_symbol(input: impl AsRef<str>) -> bool {
    let input = input.as_ref();

    input.starts_with(".?") && input.ends_with('@')
}