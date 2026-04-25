use enum_handlers::{with_enum_handlers, handle};

enum Message {
    Get,
    Inc(isize),
}

struct App {
    state: isize,
}

#[with_enum_handlers(Message)]
impl App {
    // Mix sync and async
    #[handle(Message::Get)]
    fn handle_get(&self) -> isize {
        self.state
    }

    #[handle(Message::Inc)]
    async fn handle_inc(&mut self, value: isize) -> isize {
        self.state += value;
        self.state
    }

    fn some_extra_method(&mut self) -> String {
        self.state -= 1;
        self.state.to_string()
    }
}

#[tokio::main]
async fn main() {
    let mut app = App { state: 0 };
    let output = app.dispatch(Message::Inc(10)).await;
    println!("{:?}", output);
    println!("{:?}", app.some_extra_method());
    let output = app.dispatch(Message::Get).await;
    println!("{:?}", output);
}
