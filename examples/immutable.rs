use std::cell::Cell;

use enum_handlers::{handle, with_enum_handlers};

struct Inc(isize, isize);

enum Message {
    Inc(Inc),
    Dec(isize),
}

struct App {
    state: Cell<isize>,
}

#[with_enum_handlers(Message)]
impl App {
    #[handle(Message::Inc)]
    fn handle_inc(&self, Inc(value, times): Inc) -> isize {
        self.state.update(|x| x + value * times);
        self.state.get()
    }

    #[handle(Message::Dec)]
    fn handle_dec(&self, value: isize) -> isize {
        self.state.update(|x| x - value);
        self.state.get()
    }
}

fn main() {
    let app = App {
        state: Default::default(),
    };

    println!("{}", app.dispatch(Message::Inc(Inc(10, 3))));

    println!("{}", app.dispatch(Message::Dec(5)));
}
