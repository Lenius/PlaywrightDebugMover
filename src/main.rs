//! Windows Rust app der:
//! - Starter skjult (uden konsolvindue)
//! - Sikrer at kun én instans kører via named mutex
//! - Lytter efter "Playwright"-vinduer og flytter dem til højre hjørne
//! - Har en indbygget Axum-webserver med /start, /stop og /kill endpoints

#![windows_subsystem = "windows"]

use std::{collections::HashSet, ffi::OsString, net::SocketAddr, os::windows::ffi::OsStringExt, sync::{Arc, Mutex}, thread, time::Duration};
use realtime::notify_session;
use widestring::U16CString;
use windows::core::PCWSTR;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT, GetLastError},
    System::Threading::CreateMutexW,
    UI::WindowsAndMessaging::{
        EnumWindows, GetSystemMetrics, GetWindowRect, GetWindowTextW, IsWindowVisible,
        MoveWindow, ShowWindow, SPI_GETWORKAREA, SM_CXSCREEN, SW_SHOWMINNOACTIVE,
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    },
};
use crossbeam_channel::{Receiver, Sender, select};
use axum::{response::{Html}, routing::get, Router};

struct AppState {
    watcher: Option<thread::JoinHandle<()>>,
    stop_tx: Option<Sender<()>>,
}

mod realtime;
mod tool;

type SharedState = Arc<Mutex<AppState>>;

fn get_window_text(hwnd: HWND) -> Option<String> {
    let mut buf = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) } as usize;
    if len > 0 {
        Some(OsString::from_wide(&buf[..len]).to_string_lossy().into_owned())
    } else {
        None
    }
}

fn get_work_area() -> RECT {
    let mut rect = RECT::default();
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    rect
}

fn move_to_top_right(hwnd: HWND, rect: RECT) {
    let width = rect.right - rect.left;
    let work_area = get_work_area();
    let height = work_area.bottom - work_area.top;
    let x = unsafe { GetSystemMetrics(SM_CXSCREEN) } - width;
    let y = work_area.top;
    unsafe {
        let _ = MoveWindow(hwnd, x, y, width, height, true);
    }
}

extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }
        if let Some(title) = get_window_text(hwnd) {
            if title.starts_with("Playwright") {
                let moved = &mut *(lparam.0 as *mut HashSet<isize>);
                if !moved.contains(&hwnd.0) {
                    notify_session("service", "fundet");
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(hwnd, &mut rect);
                    move_to_top_right(hwnd, rect);
                    moved.insert(hwnd.0);
                }
                // Stop enumeration for this cycle
                return false.into();
            }
        }
        true.into()
    }
}

fn watcher_loop(stop_rx: Receiver<()>) {
    let mut moved_windows = HashSet::<isize>::new();
    loop {
        select! {
            recv(stop_rx) -> _ => break,
            default() => {
                unsafe {
                    let _ = EnumWindows(Some(enum_windows_proc), LPARAM(&mut moved_windows as *mut _ as isize));
                }
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

async fn handler_root() -> Html<&'static str> {
    Html(r#"
        <html>
        <body>
        <h1>PW Mover</h1>
        <div>
          <button onclick="wsCmd('start')">Start</button>
          <button onclick="wsCmd('stop')">Stop</button>
          <button onclick="wsCmd('kill')">Kill</button>
          <button onclick="wsCmd('template')">Template</button>
          <div id="ws-status" style="margin-top:12px;color:#296;">
            Status: <span id="ws-connected">forbinder...</span>
          </div>
          <div id="ws-response" style="margin-top:12px;color:#962;"></div>
        </div>
        <hr>
        <script>
        let ws;
        function wsConnect() {
          ws = new WebSocket("ws://"+location.host+"/ws");
          ws.onopen = () => {
            document.getElementById('ws-connected').innerText = "tilsluttet";
          };
          ws.onclose = () => {
            document.getElementById('ws-connected').innerText = "afbrudt – prøver igen om lidt...";
            setTimeout(wsConnect, 1500);
          };
          ws.onmessage = (ev) => {
            document.getElementById('ws-response').innerText = ev.data;
          };
        }
        function wsCmd(cmd) {
          if (ws && ws.readyState === 1) {
            ws.send(cmd);
          }
        }
        wsConnect();
        </script>
        </body>
        </html>
    "#)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    realtime::init_broadcaster();

    // Sikr. kun én instans via mutex
    let name = U16CString::from_str("Global\\PlaywrightMoverMutex").unwrap();
    let name_ptr = PCWSTR::from_raw(name.as_ptr());
    let _mx = unsafe { CreateMutexW(None, false, name_ptr) };
    if unsafe { GetLastError().0 } == 183 {
        return;
    }

    // Skjul konsolvinduet
    unsafe { let _ = ShowWindow(HWND(0), SW_SHOWMINNOACTIVE); }

    // Init delt state
    let state = Arc::new(Mutex::new(AppState { watcher: None, stop_tx: None }));

    // Byg Axum-router
    let app = Router::new()
        .route("/", get(handler_root))
        .route("/ws", get(realtime::ws_handler)) //handle_ws
        .with_state(state);

    // Kør server

    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app.into_make_service())
        .await
        .unwrap();
}
