//! Windows Rust app der:
//! - Starter skjult (uden konsolvindue)
//! - Sikrer at kun én instans kører via named mutex
//! - Lytter efter "Playwright"-vinduer og flytter dem til højre hjørne
//! - Har en indbygget webserver hvor man kan lukke programmet via browser

#![windows_subsystem = "windows"]

use std::{collections::HashSet, ffi::OsString, os::windows::ffi::OsStringExt, thread, time::Duration};
use crossbeam_channel::{bounded, select};
use widestring::U16CString;
use tiny_http::{Server, Response, Header};

use windows::core::PCWSTR;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT, GetLastError},
    System::Threading::CreateMutexW,
    UI::WindowsAndMessaging::{
        EnumWindows, GetSystemMetrics, GetWindowRect, GetWindowTextW, IsWindowVisible,
        MoveWindow, ShowWindow, SystemParametersInfoW, SPI_GETWORKAREA, SM_CXSCREEN,
        SW_SHOWMINNOACTIVE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    },
};

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
        let _ = SystemParametersInfoW(
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

fn start_watcher_thread(stop_rx: crossbeam_channel::Receiver<()>) {
    thread::spawn(move || {
        let mut moved_windows = HashSet::<isize>::new();

        loop {
            select! {
                recv(stop_rx) -> _ => {
                    println!("Watcher stopper...");
                    break;
                },
                default() => {
                    unsafe {
                        let _ = EnumWindows(Some(enum_windows_proc), LPARAM(&mut moved_windows as *mut _ as isize));
                    }
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }
    });
}

extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }

        if let Some(title) = get_window_text(hwnd) {
            if title.starts_with("Playwright") {
                let moved_windows = &mut *(lparam.0 as *mut HashSet<isize>);
                if !moved_windows.contains(&hwnd.0) {
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(hwnd, &mut rect);
                    move_to_top_right(hwnd, rect);
                    moved_windows.insert(hwnd.0);
                }
                return false.into();
            }
        }

        true.into()
    }
}

fn main() {
    let name = U16CString::from_str("Global\\PlaywrightMoverMutex").unwrap();
    let name_ptr = PCWSTR::from_raw(name.as_ptr());
    let _mutex = unsafe {
        CreateMutexW(None, false, name_ptr)
    };
    if unsafe { GetLastError().0 } == 183 {
        println!("Programmet kører allerede.");
        return;
    }

    unsafe {
        let _ = ShowWindow(HWND(0), SW_SHOWMINNOACTIVE);
    }

    let (stop_tx, stop_rx) = bounded::<()>(1);
    start_watcher_thread(stop_rx.clone());

    {
        let stop_tx = stop_tx.clone();
        thread::spawn(move || {
            let server = Server::http("127.0.0.1:8080").expect("Kan ikke starte webserver");
            for req in server.incoming_requests() {
                if req.url().starts_with("/stop") {
                    println!("Stop-request modtaget fra browser");
                    let _ = req.respond(Response::from_string("Programmet stoppes..."));
                    let _ = stop_tx.send(());
                    thread::sleep(Duration::from_millis(200));
                    std::process::exit(0);
                }

                let html = r#"
                    <html><body>
                    <h1>PW Mover running</h1>
                    <form method='GET' action='/stop'>
                        <button type='submit'>Stop programmet</button>
                    </form>
                    </body></html>
                "#;

                let response = Response::from_string(html)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
                let _ = req.respond(response);
            }
        });
    }

    ctrlc::set_handler(move || {
        println!("Ctrl+C modtaget – lukker...");
        let _ = stop_tx.send(());
    }).expect("Kunne ikke opsætte Ctrl+C handler");

    println!("PW Mover kører. Besøg http://localhost:8080 for at stoppe det.");

    loop {
        if let Ok(_) = stop_rx.try_recv() {
            println!("Stop-signal modtaget – program lukker.");
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }

    println!("Program stoppet.");
}