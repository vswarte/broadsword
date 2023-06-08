use std::fs;
use std::io::BufReader;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub entries: Vec<SignatureEntry>,
}

#[derive(Debug, Deserialize)]
pub struct SignatureEntry {
    pub key: String,
    pub signature: String,
    pub method: SignatureEntryMethod,
}

#[derive(Debug, Deserialize)]
pub enum SignatureEntryMethod {
    Offset,
    CaptureTarget,
}

pub fn parse_profile_file(input: fs::File) -> Result<Profile, serde_json::Error> {
    let reader = BufReader::new(input);
    serde_json::from_reader(reader)
}