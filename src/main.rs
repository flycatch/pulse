#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod format;
mod icon;
mod popup;
mod stats;

use std::time::{Duration, Instant};

use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
#[cfg(target_os = "macos")]
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

const TICK: Duration = Duration::from_secs(1);

enum UserEvent {
    Tray(TrayIconEvent),
    Menu(MenuEvent),
}

fn blank_icon() -> Icon {
    // 1x1 transparent; macOS shows title text, others get tooltip until icon renderer lands.
    Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).expect("icon")
}

fn main() {
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    #[cfg(target_os = "macos")]
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    // Forward tray/menu events into the loop so clicks wake it immediately.
    let p = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |e| {
        let _ = p.send_event(UserEvent::Tray(e));
    }));
    let p = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = p.send_event(UserEvent::Menu(e));
    }));

    let menu = Menu::new();
    let total_item = MenuItem::new("…", false, None);
    let graph_item = MenuItem::new("Show graph", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append_items(&[&total_item, &graph_item, &PredefinedMenuItem::separator(), &quit])
        .ok();
    let (quit_id, graph_id) = (quit.id().clone(), graph_item.id().clone());

    // One disabled row per interface, inserted after the total row (position 0).
    let mut iface_rows: Vec<MenuItem> = Vec::new();
    let mut tray = None;
    let mut monitor = stats::Monitor::new();
    let mut last = stats::Sample::default();
    let mut popup: Option<popup::Popup> = None;
    let mut anchor = None;
    let bits = false;
    #[cfg(not(target_os = "macos"))]
    let mut icon_key = String::new();

    event_loop.run(move |event, target, flow| {
        *flow = ControlFlow::WaitUntil(Instant::now() + TICK);
        match event {
            Event::NewEvents(StartCause::Init) => {
                tray = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(menu.clone()))
                        .with_menu_on_left_click(false)
                        .with_icon(blank_icon())
                        .build()
                        .expect("tray"),
                );
                if std::env::args().any(|a| a == "--popup") {
                    popup = popup::Popup::open(target, None);
                }
            }
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                last = monitor.sample();
                let title = format!(
                    "↓{} ↑{}",
                    format::rate(last.down, bits),
                    format::rate(last.up, bits)
                );
                total_item.set_text(&title);
                while iface_rows.len() > last.ifaces.len() {
                    if let Some(r) = iface_rows.pop() {
                        menu.remove(&r).ok();
                    }
                }
                while iface_rows.len() < last.ifaces.len() {
                    let r = MenuItem::new("", false, None);
                    menu.insert(&r, 1 + iface_rows.len()).ok();
                    iface_rows.push(r);
                }
                for (row, i) in iface_rows.iter().zip(&last.ifaces) {
                    row.set_text(format!(
                        "{}: ↓{} ↑{}",
                        i.name,
                        format::rate(i.down, bits),
                        format::rate(i.up, bits)
                    ));
                }
                if let Some(t) = &tray {
                    #[cfg(target_os = "macos")]
                    t.set_title(Some(&title));
                    t.set_tooltip(Some(&title)).ok();
                    // Windows/Linux trays have no text: redraw the icon only when it changes.
                    #[cfg(not(target_os = "macos"))]
                    {
                        let (d, u) = (icon::compact(last.down), icon::compact(last.up));
                        let key = format!("{d}|{u}");
                        if key != icon_key {
                            if let Some(i) = icon::render(&d, &u) {
                                t.set_icon(Some(i)).ok();
                            }
                            icon_key = key;
                        }
                    }
                }
                if let Some(pp) = &popup {
                    pp.request_redraw();
                }
            }
            Event::UserEvent(UserEvent::Tray(TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            })) => {
                anchor = Some((rect.position, f64::from(rect.size.width), f64::from(rect.size.height)));
                if popup.take().is_none() {
                    popup = popup::Popup::open(target, anchor);
                }
            }
            Event::UserEvent(UserEvent::Menu(e)) => {
                if e.id == quit_id {
                    *flow = ControlFlow::Exit;
                } else if e.id == graph_id && popup.is_none() {
                    popup = popup::Popup::open(target, anchor);
                }
            }
            Event::WindowEvent { window_id, event, .. } => {
                if popup.as_ref().is_some_and(|p| p.is(window_id)) {
                    match event {
                        // Drop on close or blur so the surface/window memory is freed.
                        WindowEvent::CloseRequested | WindowEvent::Focused(false) => popup = None,
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) => {
                if let Some(p) = popup.as_mut().filter(|p| p.is(window_id)) {
                    p.draw(&monitor, &last, bits);
                }
            }
            _ => {}
        }
    });
}
