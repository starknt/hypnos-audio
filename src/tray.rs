use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::windows::EventLoopBuilderExtWindows;
use winit::window::WindowId;

use crate::{startup, updater};

fn load_icon() -> Result<tray_icon::Icon, tray_icon::BadIcon> {
    const ICON_SIZE: u32 = 64;
    let raw = include_bytes!("../assets/icon.rgba");
    tray_icon::Icon::from_rgba(raw.to_vec(), ICON_SIZE, ICON_SIZE)
}

pub fn run_tray(shutdown_tx: tokio::sync::mpsc::Sender<()>) -> crate::Result<()> {
    let menu = Menu::new();

    let update_item = MenuItem::new("Check for updates", true, None);

    let startup_item = CheckMenuItem::new("Launch on startup", true, false, None);
    match startup::is_enabled() {
        Ok(enabled) => startup_item.set_checked(enabled),
        Err(e) => tracing::warn!(error = %e, "failed to read startup registry state"),
    }

    let quit_item = MenuItem::new("Exit", true, None);

    menu.append(&update_item)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&startup_item)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&quit_item)?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Hypnos Audio - Bluetooth Auto-Mute")
        .with_icon(load_icon()?)
        .build()?;

    let event_loop = EventLoop::builder().with_any_thread(true).build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = TrayApp {
        shutdown_tx,
        update_item_id: update_item.id().clone(),
        quit_item_id: quit_item.id().clone(),
        startup_item,
        _tray_icon: tray_icon,
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}

struct TrayApp {
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
    update_item_id: tray_icon::menu::MenuId,
    quit_item_id: tray_icon::menu::MenuId,
    startup_item: CheckMenuItem,
    _tray_icon: TrayIcon,
}

impl ApplicationHandler for TrayApp {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.quit_item_id {
                if let Err(e) = self.shutdown_tx.try_send(()) {
                    tracing::warn!(error = %e, "failed to send shutdown signal");
                }
                event_loop.exit();
            } else if event.id == self.update_item_id {
                std::thread::spawn(|| {
                    updater::check_and_download_notify();
                });
            } else if event.id == *self.startup_item.id() {
                let new_state = self.startup_item.is_checked();
                if let Err(e) = startup::set_enabled(new_state) {
                    tracing::error!(error = %e, "failed to update startup registry");
                } else {
                    tracing::info!(startup_enabled = new_state, "startup registry updated");
                }
            }
        }
    }
}
