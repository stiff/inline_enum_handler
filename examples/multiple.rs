use enum_handlers::{handle, with_enum_handlers};

enum ApiMessage {
    Inc,
    #[allow(dead_code)]
    Dec,
}

enum BackgroundMessage {
    Done(isize),
}

struct App {
    state: isize,
}

struct IcedTask;

#[with_enum_handlers(ApiMessage, dispatch=on_api)]
#[with_enum_handlers(BackgroundMessage, dispatch=on_background)]
impl App {
    #[handle(ApiMessage::Inc)]
    async fn handle_api_inc(&mut self) -> IcedTask {
        println!("handling api message Inc");
        self.state += 1;
        IcedTask
    }

    #[handle(ApiMessage::Dec)]
    async fn handle_confusing_name_dec(&mut self) -> IcedTask {
        println!("handling api message Dec");
        self.state -= 1;
        IcedTask
    }

    // Handler for BackgroundMessage
    // different enum than ApiMessage handlers, async/sync, return type
    #[handle(BackgroundMessage::Done)]
    fn handle_done(&mut self, value: isize) {
        println!("Saving background job result");
        self.state = value;
    }

    fn get(&self) -> isize {
        self.state
    }
}

#[tokio::main]
async fn main() {
    let mut app = App { state: 0 };

    app.on_api(ApiMessage::Inc).await;
    println!("{}", app.get());

    app.on_background(BackgroundMessage::Done(5));
    println!("{}", app.get());

    app.on_api(ApiMessage::Dec).await;
    println!("{}", app.get());
}
