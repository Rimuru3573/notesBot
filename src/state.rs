#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    Title,
    Text {
        title: String,
    },
    Priority {
        title: String,
        text: String,
    },
}
