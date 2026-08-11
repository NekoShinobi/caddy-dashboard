use std::{collections::HashMap, sync::Mutex};

pub struct LoginThrottle {
    failures: Mutex<HashMap<String, u32>>,
}

impl LoginThrottle {
    pub fn new() -> Self {
        Self {
            failures: Mutex::new(HashMap::new()),
        }
    }

    pub fn record_failure(&self, username: &str) -> u32 {
        let mut map = self.failures.lock().unwrap();
        let count = map.entry(username.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    pub fn record_success(&self, username: &str) {
        self.failures.lock().unwrap().remove(username);
    }

    pub fn fail_count(&self, username: &str) -> u32 {
        self.failures
            .lock()
            .unwrap()
            .get(username)
            .copied()
            .unwrap_or(0)
    }
}
