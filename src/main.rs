#[cfg(windows)] extern crate winapi;
use std::fmt::UpperHex;
use std::io::Error;
use std::mem::MaybeUninit;
use std::{thread, time};
use eframe::egui::{self, Response};
use log::info;
use winapi::shared::windef::{HMENU, HMENU__, HWND, LPRECT, POINT, RECT};
use winapi::shared::minwindef::{LPARAM};
use winapi::um::winuser::{HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WINDOWINFO, WS_EX_TOPMOST};
use interpol::format as s;

mod validating_value;
use validating_value::ValidatingValue;

#[cfg(windows)]
fn print_message(msg: &str) -> Result<i32, Error> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::winuser::{MB_OK, MessageBoxW};
    let wide: Vec<u16> = OsStr::new(msg).encode_wide().chain(once(0)).collect();
    let ret = unsafe {
        MessageBoxW(null_mut(), wide.as_ptr(), wide.as_ptr(), MB_OK)
    };
    if ret == 0 { Err(Error::last_os_error()) }
    else { Ok(ret) }
}

#[cfg(windows)]
fn getMousePos() -> Result<POINT, Error> {
    let mut lpPoint: POINT = POINT { x: 0, y: 0 };
    let ret = unsafe {
        winapi::um::winuser::GetCursorPos(&mut lpPoint)
    };
    if ret == 0 { Err(Error::last_os_error()) }
    else { Ok(lpPoint) }
}

fn getHwnd(pt: &POINT) -> Result<HWND, Error> {
    let ret = unsafe {
        let mut lpPoint: POINT = pt.clone();
        winapi::um::winuser::WindowFromPoint(lpPoint)
    };
    if ret.is_null() { Err(Error::last_os_error()) }
    else { Ok(ret) }
}

fn getMenu(hwnd: &HWND) -> Result<HMENU, Error> {
    let ret = unsafe {
        let hwndc = hwnd.clone();
        winapi::um::winuser::GetMenu(hwndc)
    };
    if ret.is_null() { Err(Error::last_os_error()) }
    else { Ok(ret) }
}

fn isMenu(hmenu: &HMENU) -> bool {
    return unsafe {
        let hmenuc = hmenu.clone();
        winapi::um::winuser::IsMenu(hmenuc) != 0
    };
}

fn doStuff() {
    let pt = getMousePos().unwrap();
    println!("pt: {}, {}", pt.x, pt.y);
    let hwnd = getHwnd(&pt).unwrap();
    println!("hwnd: {:?}", hwnd);
    let hmenu = getMenu(&hwnd);
    if (hmenu.is_ok()) {
        println!("hmenu: {:?}", hmenu.unwrap());
    } else {
        println!("hmenu err: {:?}", hmenu.err());
    }
    unsafe {
        type Callback = unsafe extern "system" fn(hwnd: HWND, s: isize) -> i32;
        unsafe extern "system" fn callback(hw: HWND, is: isize) -> i32 {
            println!("child - {:?}", hw);
            1
        }

        winapi::um::winuser::EnumChildWindows(hwnd.clone(), Some(callback as Callback), 0 as LPARAM);
    }
}

fn logln(app: &mut MyApp, s: &str) {
    println!("{}",s);
    app.logText = app.logText.clone()+s+"\n";
}

// fn convert<T, U>(a: *mut T) -> *mut U {
//     let p = a.addr() as *const U;
//     return unsafe { ptr::read(p) };
// }

#[cfg(not(windows))]
fn print_message(msg: &str) -> Result<(), Error> {
    println!("{}", msg);
    Ok(())
}
fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "WINAPI Hooks",
        options,
        Box::new(|cc| {
            // This gives us image support:
            // egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<MyApp>::default())
        }),
    )
}

use derive_more::Display;

#[derive(Display, Debug)]
// #[display("{self:?}")] // Forgot logText would recurse
#[display("{:?},{},{},{},{}", self.selectedHwnd, self.x, self.y, self.sx, self.sy)]
struct MyApp {
    selectedHwnd: Option<HWND>,
    logText: String,
    x: i32,
    y: i32,
    sx: i32,
    sy: i32,
    tempString: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            selectedHwnd: None,
            logText: "".to_string(),
            x: 0, //SHAME It would be nicer to use Option, but then it doesn't work well with the DragValues
            y: 0,
            sx: 0,
            sy: 0,
            tempString: "".to_string(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("Selected hwnd {:?}", self.selectedHwnd));
            ui.horizontal(|ui| {
                if ui.button("Set hwnd").focus_clicked() {
                    match (|| -> Result<HWND, <usize as std::str::FromStr>::Err> {
                        let res = self.tempString.parse::<usize>()?;
                        return Ok(res as HWND);
                    })() {
                        Ok(hwnd) => {
                            self.selectedHwnd = Some(hwnd);
                            self.logText = "".to_string();
                        },
                        Err(e) => {
                            self.selectedHwnd = None;
                            self.logText = e.to_string();
                        },
                    }
                }
                ui.add(ValidatingValue::new(
                    &mut self.x,
                    |f| {s!("{f}")},
                    |str| {str.parse::<i32>().ok()}
                )); //DUMMY
            });
            if ui.button("Get window under mouse").focus_clicked() {
                match (|| -> Result<HWND, Error> {
                    let pt = getMousePos()?;
                    println!("pt: {}, {}", pt.x, pt.y);
                    let hwnd = getHwnd(&pt)?;
                    println!("hwnd: {:?}", hwnd);
                    return Ok(hwnd);
                })() {
                    Ok(hwnd) => {
                        self.selectedHwnd = Some(hwnd);
                        self.logText = "".to_string();
                    },
                    Err(e) => {
                        self.selectedHwnd = None;
                        self.logText = e.to_string();
                    },
                }
            }
            ui.add_enabled_ui(self.selectedHwnd != None, |ui| {
                if ui.button("Parent").focus_clicked() {
                    let hwnd = self.selectedHwnd.unwrap();
                    unsafe {
                        self.selectedHwnd = Some(winapi::um::winuser::GetParent(hwnd));
                    }
                }
                if ui.button("Toggle stay-on-top").focus_clicked() {
                    let hwnd = self.selectedHwnd.unwrap();
                    unsafe {
                        let info: MaybeUninit<WINDOWINFO> = MaybeUninit::zeroed();
                        let mut info = info.assume_init();
                        winapi::um::winuser::GetWindowInfo(hwnd, &mut info);
                        if (info.dwExStyle & WS_EX_TOPMOST) != 0 {
                            winapi::um::winuser::SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
                        } else {
                            winapi::um::winuser::SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
                        }
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("Get pos").focus_clicked() {
                        let hwnd = self.selectedHwnd.unwrap();
                        unsafe {
                            let mut lpRect = RECT {
                                left: 0,
                                right: 0,
                                top: 0,
                                bottom: 0,
                            };
                            winapi::um::winuser::GetWindowRect(hwnd, &mut lpRect as LPRECT);
                            self.x = lpRect.left;
                            self.y = lpRect.top;
                            self.sx = lpRect.right-lpRect.left;
                            self.sy = lpRect.bottom-lpRect.top;
                            logln(self, &s!("self: {self}"));
                        }
                    }
                    if ui.button("Set pos").focus_clicked() {
                        let hwnd = self.selectedHwnd.unwrap();
                        unsafe {
                            winapi::um::winuser::SetWindowPos(hwnd, HWND_TOP, self.x, self.y, self.sx, self.sy, SWP_NOZORDER | SWP_NOACTIVATE);
                        }
                    }
                    ui.add(egui::DragValue::new(&mut self.x));
                    ui.add(egui::DragValue::new(&mut self.y));
                    ui.add(egui::DragValue::new(&mut self.sx));
                    ui.add(egui::DragValue::new(&mut self.sy));
                });
                //ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            });
            if ui.button("Clear log").focus_clicked() {
                self.logText = "".to_string();
            }
            ui.label("Log:");
            ui.label(&self.logText);
        });
    }
}

trait FocusClicked {
    fn focus_clicked(&self) -> bool;
}

impl FocusClicked for Response {
    fn focus_clicked(&self) -> bool {
        let res = self.clicked();
        if res {
            self.request_focus();
        }
        return res;
    }
}