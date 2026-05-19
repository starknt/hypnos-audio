use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowId;

pub fn run_tray(shutdown_tx: tokio::sync::mpsc::Sender<()>) -> Result<()> {
    let menu = Menu::new();
    let quit_item = MenuItem::new("Exit", true, None);
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&quit_item)?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Hypnos Audio - Bluetooth Auto-Mute")
        .build()?;

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = TrayApp {
        shutdown_tx,
        quit_item_id: quit_item.id().clone(),
        _tray_icon: tray_icon,
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}

struct TrayApp {
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
    quit_item_id: tray_icon::menu::MenuId,
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
            }
        }
    }
}
