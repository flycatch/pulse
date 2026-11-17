#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod config;
mod format;
mod icon;
mod popup;
mod stats;

use std::time::{Duration, Instant};

use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
#[cfg(target_os = "macos")]
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

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
    let mut cfg = config::Config::load();
    let bits_item = CheckMenuItem::new("Show bits (b/s)", true, cfg.bits, None);
    let sys_item = CheckMenuItem::new("Show CPU/mem/disk", true, cfg.show_sys, None);
    let sys_row = MenuItem::new("…", false, None);
    let interval_items: Vec<CheckMenuItem> = config::INTERVALS
        .iter()
        .map(|&n| CheckMenuItem::new(format!("{n}s"), true, n == cfg.interval_secs, None))
        .collect();
    let interval_refs: Vec<&dyn IsMenuItem> =
        interval_items.iter().map(|i| i as &dyn IsMenuItem).collect();
    let interval_menu = Submenu::with_items("Update interval", true, &interval_refs).expect("submenu");
    menu.append_items(&[
        &total_item,
        &sys_row,
        &graph_item,
        &PredefinedMenuItem::separator(),
        &bits_item,
        &sys_item,
        &interval_menu,
        &PredefinedMenuItem::separator(),
        &quit,
    ])
    .ok();
    let (quit_id, graph_id) = (quit.id().clone(), graph_item.id().clone());

    // One disabled row per interface, inserted after the total and system rows (positions 0, 1).
    let mut iface_rows: Vec<MenuItem> = Vec::new();
    let mut next_tick = Instant::now() + Duration::from_secs(cfg.interval_secs);
    let mut tray = None;
    let mut monitor = stats::Monitor::new();
    let mut last = stats::Sample::default();
    let mut popup: Option<popup::Popup> = None;
    let mut anchor = None;
    #[cfg(not(target_os = "macos"))]
    let mut icon_key = String::new();

    event_loop.run(move |event, target, flow| {
        *flow = ControlFlow::WaitUntil(next_tick);
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
                next_tick = Instant::now() + Duration::from_secs(cfg.interval_secs);
                last = monitor.sample();
                let net = format!(
                    "↓{} ↑{}",
                    format::rate(last.down, cfg.bits),
                    format::rate(last.up, cfg.bits)
                );
                total_item.set_text(&net);
                let sys = format::sys_line(last.cpu, last.mem, last.disk);
                sys_row.set_text(&sys);
                let title = if cfg.show_sys { format!("{net} · {sys}") } else { net };
                while iface_rows.len() > last.ifaces.len() {
                    if let Some(r) = iface_rows.pop() {
                        menu.remove(&r).ok();
                    }
                }
                while iface_rows.len() < last.ifaces.len() {
                    let r = MenuItem::new("", false, None);
                    menu.insert(&r, 2 + iface_rows.len()).ok();
                    iface_rows.push(r);
                }
                for (row, i) in iface_rows.iter().zip(&last.ifaces) {
                    row.set_text(format!(
                        "{}: ↓{} ↑{}",
                        i.name,
                        format::rate(i.down, cfg.bits),
                        format::rate(i.up, cfg.bits)
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
                } else if e.id == *bits_item.id() {
                    cfg.bits = bits_item.is_checked();
                    cfg.save();
                } else if e.id == *sys_item.id() {
                    cfg.show_sys = sys_item.is_checked();
                    cfg.save();
                } else if let Some(n) = interval_items
                    .iter()
                    .zip(config::INTERVALS)
                    .find(|(i, _)| *i.id() == e.id)
                    .map(|(_, n)| n)
                {
                    // Behave like radio buttons.
                    for (i, v) in interval_items.iter().zip(config::INTERVALS) {
                        i.set_checked(v == n);
                    }
                    cfg.interval_secs = n;
                    next_tick = Instant::now() + Duration::from_secs(n);
                    cfg.save();
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
                    p.draw(&monitor, &last, cfg.bits);
                }
            }
            _ => {}
        }
    });
}
