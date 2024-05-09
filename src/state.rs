use std::fs;

#[derive(Clone)]
pub struct AppState {
    pub http_client: reqwest::Client,
    pub forbidden_hosts: Vec<String>,
    pub banned_words: Vec<String>,
}

impl AppState {
    pub fn new() -> Self {
        let contents = fs::read_to_string("resources/forbidden-hosts.txt")
            .expect("Should have been able to read the file");
        let forbidden_hosts: Vec<String> = contents.lines().map(|l| l.to_string()).collect();

        let contents2 = fs::read_to_string("resources/banned-words.txt")
            .expect("Should have been able to read the file");
        let banned_words: Vec<String> = contents2.lines().map(|l| l.to_string()).collect();

        Self {
            http_client: reqwest::Client::new(),
            forbidden_hosts,
            banned_words,
        }
    }
}
