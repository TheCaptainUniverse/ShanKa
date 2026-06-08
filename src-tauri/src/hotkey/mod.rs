use crate::{config, pipeline, platform, selection};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DEBOUNCE_MS: u64 = 300;
const HOTKEY_RELEASE_SETTLE_DELAY: Duration = Duration::from_millis(140);

static CAPTURE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static HOTKEY_RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);
static LAST_TRIGGER_MS: AtomicU64 = AtomicU64::new(0);
static HOTKEY_ROUTES: Mutex<Option<[HotkeyRoute; 2]>> = Mutex::new(None);
static EVENT_HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static HOTKEY_MANAGER: RefCell<Option<GlobalHotKeyManager>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
struct HotkeyRoute {
    mode: selection::SelectionMode,
    hotkey: HotKey,
}

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    let routes = routes_from_hotkeys(load_hotkeys(app));

    HOTKEY_MANAGER.with(|manager_cell| {
        let mut manager_slot = manager_cell.borrow_mut();

        if manager_slot.is_some() {
            return Err(to_tauri_error("hotkey manager is already initialized"));
        }

        let manager = GlobalHotKeyManager::new().map_err(to_tauri_error)?;
        register_hotkey_set(&manager, &routes).map_err(to_tauri_error)?;
        *manager_slot = Some(manager);
        Ok::<(), tauri::Error>(())
    })?;

    install_event_handler(app.clone());
    replace_routes(routes).map_err(to_tauri_error)?;

    println!(
        "[hotkey] global hotkey router ready: {} = Safe Mode, {} = Magic Mode",
        routes[0].hotkey, routes[1].hotkey
    );
    Ok(())
}

pub fn reload(app: &tauri::AppHandle, hotkeys: config::ResolvedHotkeyConfig) -> Result<(), String> {
    let next_routes = routes_from_hotkeys(hotkeys);
    let (sender, receiver) = mpsc::channel();

    app.run_on_main_thread(move || {
        let result = reload_on_hotkey_thread(next_routes);
        let _ = sender.send(result);
    })
    .map_err(|error| format!("failed to schedule hotkey reload: {error}"))?;

    receiver
        .recv()
        .map_err(|error| format!("failed to receive hotkey reload result: {error}"))?
}

pub fn set_recording_active(active: bool) {
    HOTKEY_RECORDING_ACTIVE.store(active, Ordering::Release);

    if active {
        println!("[hotkey] hotkey triggers paused for shortcut recording");
    } else {
        println!("[hotkey] hotkey triggers resumed after shortcut recording");
    }
}

fn reload_on_hotkey_thread(next_routes: [HotkeyRoute; 2]) -> Result<(), String> {
    HOTKEY_MANAGER.with(|manager_cell| {
        let manager_slot = manager_cell.borrow();
        let manager = manager_slot
            .as_ref()
            .ok_or_else(|| "hotkey manager is not initialized".to_string())?;
        let previous_routes = current_routes()?;

        if same_hotkeys(&previous_routes, &next_routes) {
            println!("[hotkey] hotkey config unchanged; keeping current registrations");
            return Ok(());
        }

        unregister_hotkey_set(manager, &previous_routes)?;

        if let Err(register_error) = register_hotkey_set(manager, &next_routes) {
            let restore_result = register_hotkey_set(manager, &previous_routes);
            if let Err(restore_error) = restore_result {
                return Err(format!(
                    "failed to register new hotkeys: {register_error}; also failed to restore previous hotkeys: {restore_error}"
                ));
            }

            return Err(format!("failed to register new hotkeys: {register_error}"));
        }

        replace_routes(next_routes)?;
        println!(
            "[hotkey] hotkey router reloaded: {} = Safe Mode, {} = Magic Mode",
            next_routes[0].hotkey, next_routes[1].hotkey
        );
        Ok(())
    })
}

fn install_event_handler(app: tauri::AppHandle) {
    if EVENT_HANDLER_INSTALLED.swap(true, Ordering::AcqRel) {
        return;
    }

    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state != HotKeyState::Released {
            return;
        }

        if let Some(route) = route_for_event(event.id) {
            handle_hotkey(app.clone(), route);
        }
    }));
}

fn route_for_event(event_id: u32) -> Option<HotkeyRoute> {
    let routes = HOTKEY_ROUTES.lock().ok()?;
    routes
        .as_ref()?
        .iter()
        .copied()
        .find(|route| route.hotkey.id() == event_id)
}

fn handle_hotkey(app: tauri::AppHandle, route: HotkeyRoute) {
    let mode = route.mode;

    if HOTKEY_RECORDING_ACTIVE.load(Ordering::Acquire) {
        println!(
            "[hotkey] {} ignored because shortcut recording is active",
            mode.label()
        );
        return;
    }

    if is_debounced() {
        println!("[hotkey] {} ignored by debounce", mode.label());
        return;
    }

    if CAPTURE_IN_PROGRESS.swap(true, Ordering::AcqRel) {
        println!(
            "[hotkey] {} ignored because capture is already running",
            mode.label()
        );
        return;
    }

    thread::spawn(move || {
        println!("[hotkey] {} triggered", mode.label());
        if !platform::wait_for_hotkey_keys_released(route.hotkey) {
            println!("[hotkey] timed out waiting for physical hotkey keys to release");
        }
        thread::sleep(HOTKEY_RELEASE_SETTLE_DELAY);

        if HOTKEY_RECORDING_ACTIVE.load(Ordering::Acquire) {
            println!(
                "[hotkey] {} cancelled because shortcut recording became active",
                mode.label()
            );
            CAPTURE_IN_PROGRESS.store(false, Ordering::Release);
            return;
        }

        platform::prepare_for_selection_capture();

        match pipeline::run(&app, mode) {
            Ok(outcome) => {
                println!(
                    "[hotkey] {} completed: {} -> {} characters, selection={}, rewrite={}, rewrite={}ms, total={}ms",
                    mode.label(),
                    outcome.selected_characters,
                    outcome.replacement_characters,
                    outcome.selection_provider,
                    outcome.rewrite_provider,
                    outcome.rewrite_duration.as_millis(),
                    outcome.total_duration.as_millis()
                );
            }
            Err(error) => {
                println!("[hotkey] {} failed: {error}", mode.label());
            }
        }

        CAPTURE_IN_PROGRESS.store(false, Ordering::Release);
    });
}

fn routes_from_hotkeys(hotkeys: config::ResolvedHotkeyConfig) -> [HotkeyRoute; 2] {
    [
        HotkeyRoute {
            mode: selection::SelectionMode::Safe,
            hotkey: hotkeys.safe_mode,
        },
        HotkeyRoute {
            mode: selection::SelectionMode::Magic,
            hotkey: hotkeys.magic_mode,
        },
    ]
}

fn hotkeys_from_routes(routes: &[HotkeyRoute; 2]) -> [HotKey; 2] {
    [routes[0].hotkey, routes[1].hotkey]
}

fn current_routes() -> Result<[HotkeyRoute; 2], String> {
    HOTKEY_ROUTES
        .lock()
        .map_err(|error| format!("failed to lock hotkey routes: {error}"))?
        .ok_or_else(|| "hotkey routes are not initialized".to_string())
}

fn replace_routes(routes: [HotkeyRoute; 2]) -> Result<(), String> {
    let mut registered_routes = HOTKEY_ROUTES
        .lock()
        .map_err(|error| format!("failed to lock hotkey routes: {error}"))?;
    *registered_routes = Some(routes);
    Ok(())
}

fn same_hotkeys(left: &[HotkeyRoute; 2], right: &[HotkeyRoute; 2]) -> bool {
    left.iter()
        .map(|route| route.hotkey.id())
        .eq(right.iter().map(|route| route.hotkey.id()))
}

fn register_hotkey_set(
    manager: &GlobalHotKeyManager,
    routes: &[HotkeyRoute; 2],
) -> Result<(), String> {
    let hotkeys = hotkeys_from_routes(routes);
    let mut registered = Vec::with_capacity(hotkeys.len());

    for hotkey in hotkeys {
        if let Err(error) = manager.register(hotkey) {
            for registered_hotkey in registered.iter().rev() {
                let _ = manager.unregister(*registered_hotkey);
            }

            return Err(format!("failed to register {hotkey}: {error}"));
        }

        registered.push(hotkey);
    }

    Ok(())
}

fn unregister_hotkey_set(
    manager: &GlobalHotKeyManager,
    routes: &[HotkeyRoute; 2],
) -> Result<(), String> {
    for hotkey in hotkeys_from_routes(routes) {
        manager
            .unregister(hotkey)
            .map_err(|error| format!("failed to unregister {hotkey}: {error}"))?;
    }

    Ok(())
}

fn load_hotkeys(app: &tauri::AppHandle) -> config::ResolvedHotkeyConfig {
    let config = config::load_or_create(app).unwrap_or_else(|error| {
        println!("[hotkey] failed to load app config; using default hotkeys: {error}");
        config::AppConfig::default()
    });

    config.hotkeys.resolve().unwrap_or_else(|error| {
        println!("[hotkey] invalid configured hotkeys; using defaults: {error}");
        config::default_hotkeys()
            .resolve()
            .expect("default hotkeys must be valid")
    })
}

fn is_debounced() -> bool {
    let now = now_ms();
    let previous = LAST_TRIGGER_MS.swap(now, Ordering::AcqRel);
    previous != 0 && now.saturating_sub(previous) < DEBOUNCE_MS
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn to_tauri_error(error: impl std::fmt::Display) -> tauri::Error {
    tauri::Error::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        error.to_string(),
    ))
}
