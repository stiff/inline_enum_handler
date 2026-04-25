use enum_handlers::{with_enum_handlers, handle};

enum Message {
    Get,
    Inc(isize),
    Dec(isize),
}

struct App {
    state: isize,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Output(isize);

#[with_enum_handlers(Message)]
impl App {
    #[handle(Message::Get)]
    fn handle_get(&self) -> Output {
        Output(self.state)
    }

    #[handle(Message::Inc)]
    fn handle_inc(&mut self, value: isize) -> Output {
        self.state += value;
        Output(self.state)
    }

    #[handle(Message::Dec)]
    fn handle_dec(&mut self, value: isize) -> Output {
        self.state -= value;
        Output(self.state)
    }
}

fn main() {
    let mut app = App { state: 0 };
    println!("{:?}", app.dispatch(Message::Inc(10)));
    println!("{:?}", app.dispatch(Message::Dec(5)));
    println!("{:?}", app.dispatch(Message::Get));
}
