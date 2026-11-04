//! The kit-iced demo app: `cargo run -p kit-iced-demo`.

use iced::futures::channel::mpsc;
use iced::futures::Stream;
use iced::Subscription;
use kit_iced::{drop_zone, theme};
use kit_iced_demo::{Demo, Message};
use std::time::Duration;

fn main() -> iced::Result {
    iced::application(Demo::new, Demo::update, Demo::view)
        .title("Kit demo")
        .theme(|_: &Demo| theme::iced_theme(&theme::look()))
        .subscription(subscription)
        .window_size((1280.0, 820.0))
        .run()
}

/// The window's drag and drop events, and the half-second tick that expires toasts and brings the fake
/// server's answers.
fn subscription(_: &Demo) -> Subscription<Message> {
    Subscription::batch([drop_zone::events().map(Message::Drop), Subscription::run(ticks)])
}

/// A [`Message::Tick`] every half second, from a thread of its own (the app runs without an async runtime's timer).
fn ticks() -> impl Stream<Item = Message> {
    let (mut tx, rx) = mpsc::channel(1);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(500));
        if tx.is_closed() {
            break;
        }
        // a tick still waiting is enough: a full channel drops this one
        let _ = tx.try_send(Message::Tick);
    });
    rx
}
