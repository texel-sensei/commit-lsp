pub struct State {
    lines: Vec<String>,
}

impl State {
    pub fn new(text: &str) -> Self {
        Self {
            lines: text.lines().map(ToOwned::to_owned).collect(),
        }
    }

    pub fn update_text(&mut self, new_text: &str) {
        self.lines = new_text.lines().map(ToOwned::to_owned).collect();
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new("")
    }
}
