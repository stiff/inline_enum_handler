use enum_handlers::{handle, with_enum_handlers};

struct Inc(isize, isize);

enum Message {
    Inc(Inc),
    Dec(isize),
}

struct App {
    state: isize,
}

struct IcedTask;

#[with_enum_handlers(Message, dispatch=update)]
impl App {
    #[handle(Message::Inc)]
    fn handle_inc(&mut self, Inc(value, _times): Inc) -> IcedTask {
        self.state += value;
        IcedTask
    }

    #[handle(Message::Dec)]
    fn handle_dec(&mut self, value: isize) -> IcedTask {
        self.state -= value;
        IcedTask
    }

    fn get(&self) -> isize {
        self.state
    }
}

fn main() {
    let mut app = App { state: 0 };

    app.update(Message::Inc(Inc(10, 3)));
    println!("{}", app.get());

    app.update(Message::Dec(5));
    println!("{}", app.get());
}
