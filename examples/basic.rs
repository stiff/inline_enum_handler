use enum_handlers::{with_enum_handlers, handle};

struct Inc(isize, isize);

enum Message {
    Inc(Inc),
    Dec(isize),
}

struct App {
    state: isize,
}

#[with_enum_handlers(Message)]
impl App {
    #[handle(Message::Inc)]
    fn handle_inc(&self, Inc(value, times): Inc) {
        // self.state += value;
        println!("{} + {}", self.state, value * times);
    }

    #[handle(Message::Dec)]
    fn handle_dec(&mut self, value: isize) {
        self.state -= value;
    }
}

fn main() {
    let mut app = App { state: 0 };

    app.dispatch(Message::Inc(Inc(10, 3)));
    println!("{}", app.state);

    app.dispatch(Message::Dec(5));
    println!("{}", app.state);
}
