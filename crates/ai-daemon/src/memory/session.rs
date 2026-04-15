/// Memoria di sessione — volatile, solo per la sessione corrente
pub struct Session {
    pub recent_actions: Vec<String>,
}

impl Session {
    pub fn new() -> Self {
        Self { recent_actions: vec![] }
    }

    pub fn add(&mut self, action: &str) {
        if self.recent_actions.len() >= 10 {
            self.recent_actions.remove(0);
        }
        self.recent_actions.push(action.chars().take(80).collect());
    }
}
