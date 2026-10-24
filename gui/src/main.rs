//! rxui — the desktop companion for `rx`.
//!
//! Owns only the presentation: the NixOS workflow itself lives in the `rx`
//! CLI, which this binary supervises step by step.

mod pipeline;
mod theme;
mod ui;

use std::borrow::Cow;
use std::time::Duration;

use gpui::{
    App, AppContext as _, Application, Bounds, Timer, WindowBounds, WindowOptions, px, size,
};
use ui::RxApp;

const FONTS: [&[u8]; 5] = [
    include_bytes!("../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../assets/fonts/Inter-Medium.ttf"),
    include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf"),
];

fn main() {
    Application::new().run(|cx: &mut App| {
        // Bundled faces keep the composition identical on every machine.
        let text_system = cx.text_system();
        for font in FONTS {
            text_system.add_fonts(vec![Cow::Borrowed(font)]).ok();
        }

        let bounds = Bounds::new(gpui::point(px(40.), px(40.)), size(px(1080.), px(760.)));
        let app = cx.new(RxApp::new);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(880.), px(620.))),
                // Reverse-DNS identity, shared with the desktop entry and the
                // AppStream metainfo so launchers can group the window.
                app_id: Some(String::from("io.github.badwatt.rxui")),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("rx — System Update".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, _cx| app.clone(),
        )
        .expect("open the rx window");

        // Foreground poller: drain supervisor messages, keep the clock live.
        cx.spawn(async move |cx| {
            loop {
                Timer::after(Duration::from_millis(70)).await;
                if app.update(cx, ui::RxApp::pump).is_err() {
                    break;
                }
            }
        })
        .detach();

        cx.activate(true);
    });
}
