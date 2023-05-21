use regex::Regex;
use msvc_demangler::demangle;

#[derive(Debug, Default)]
pub struct TypeInfoSymbol {
    pub name: String,
    pub namespaces: Vec<String>,
}

// TODO: write an actual mangled RTTI symbol name parser and get rid of the junk here
impl TypeInfoSymbol {
    // WARNING: hacky as hell
    pub fn from(input: &str) -> Self {
        let name = if input.starts_with(".?A") {
            format!("??1{}QAE@XZ", &input[4..]).to_string()
        } else {
            return Self::default();
        };

        // Strip garbage off of the beginning
        let re = Regex::new("^(?:(public|private): (?:.+? )?__.+? )").unwrap();
        let flags = msvc_demangler::DemangleFlags::llvm();
        let demangled = demangle(name.as_str(), flags);
        if demangled.is_err() {
            return Self::default();
        }

        let cleaned = re.replace(demangled.unwrap().as_str(), "").to_string();

        // Find the destructor def and remove it
        let re = Regex::new("::~(.*)$").unwrap();
        let mat = re.find(cleaned.as_str());
        if mat.is_none() {
            return Self::default();
        }

        let mat = mat.unwrap();
        let clean = cleaned[..mat.start()].to_string();
        let mut parts = into_parts(clean);
        if parts.len() == 0 {
            return Self::default();
        }

        let name = parts.remove(parts.len() - 1);
        let namespaces = parts;

        Self { name, namespaces }
    }
}

/// Makes an educated guess as to whether or not some string is a mangled symbol name.
/// Not a 100% accurate but should suffice for the time being.
pub fn is_decorated_symbol(input: &str) -> bool {
    input.starts_with(".?") && input.ends_with('@')
}

/// This function attempts to split namespace parts.
/// It does this by assuming that <> and () always match up in count.
pub(crate) fn into_parts(input: String) -> Vec<String> {
    let split = input.split("::").map(|c| c.to_string());

    let mut result = vec![];
    let mut capture = String::new();
    for item in split {
        // Check if there's a chunk already in the capture. If there is we need to add back the
        // namespace separator.
        capture = if capture.is_empty() {
            item
        } else {
            format!("{}::{}", capture, item).to_string()
        };

        // Check if all <> and () pairs line up
        let generic_closers = capture.matches(">").count() as isize;
        let generic_openers = capture.matches("<").count() as isize;
        let function_closers = capture.matches(")").count() as isize;
        let function_openers = capture.matches("(").count() as isize;

        // All <> and () align for the capture
        if generic_openers - generic_closers == 0 && function_openers - function_closers == 0 {
            result.push(capture);
            capture = String::default();
        }
    }

    result
}
