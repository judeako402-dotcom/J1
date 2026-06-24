#![windows_subsystem = "windows"]

use webview2_com::*;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const CHROME_HEIGHT: i32 = 60;

const CHROME_HTML: &str = r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;font-size:13px;background:#1e1e1e;color:#ccc;overflow:hidden;height:100vh;display:flex;flex-direction:column;user-select:none}
#nav-bar{display:flex;align-items:center;height:36px;padding:0 8px;gap:6px;background:#1e1e1e;border-bottom:1px solid #333}
.nb{width:28px;height:28px;border:none;background:transparent;color:#999;cursor:pointer;border-radius:4px;font-size:12px;display:flex;align-items:center;justify-content:center}
.nb:hover{background:#333;color:#fff}.nb:disabled{opacity:.3;cursor:default}.nb:disabled:hover{background:transparent}
#ub{flex:1}
#url{width:100%;height:28px;padding:0 10px;border:1px solid #444;border-radius:14px;background:#2a2a2a;color:#ccc;font-size:13px;outline:none}
#url:focus{border-color:#4ea8ff;background:#1e1e1e}
#url.err{border-color:#ff5f57}
#url::placeholder{color:#666}
#lb{position:fixed;top:0;left:0;right:0;height:2px;background:#4ea8ff;z-index:20;transform:scaleX(0);transform-origin:left;transition:transform .2s;pointer-events:none}
#lb.on{transform:scaleX(1);transition:transform .3s ease-out}
</style></head><body>
<div id="nav-bar">
<button id="bk" class="nb" title="Back">&lt;</button>
<button id="fw" class="nb" title="Forward">&gt;</button>
<button id="rl" class="nb" title="Reload">&#8635;</button>
<button id="hm" class="nb" title="Home">Home</button>
<div id="ub"><input id="url" type="text" placeholder="Enter URL or search..."/></div>
</div>
<div id="lb"></div>
<script>
const H="https://duckduckgo.com/";
let B={url:"",title:"J1",loading:false},pend=null;
const $=id=>document.getElementById(id);
const url=$("url"),lb=$("lb");

function post(m){window.chrome.webview.postMessage(JSON.stringify(m))}
function nav(s){
  if(pend)return;
  let u;
  if(/^https?:\/\//i.test(s)||s.startsWith("about:"))u=s;
  else if(/^[\w.-]+\.[a-z]{2,}/i.test(s)||(s.includes(".")&&!s.includes(" ")))u="https://"+s;
  else u="https://duckduckgo.com/?q="+encodeURIComponent(s);
  pend=u;url.disabled=true;url.value=u;lb.classList.add("on");
  post({cmd:"navigate",url:u});B.url=u;B.loading=true;
  setTimeout(()=>{pend=null;url.disabled=false},500);
}
function goBack(){post({cmd:"go_back"})}
function goFwd(){post({cmd:"go_forward"})}
function rl(){post({cmd:"reload"})}
function st(){post({cmd:"stop"})}
function home(){nav(H)}

function upd(){
  if(!pend)url.value=B.url;
  url.disabled=!!pend;
  if(!pend&&B.url==="about:blank")url.value="";
  lb.classList.toggle("on",B.loading);
}

window.chrome.webview.addEventListener("message",e=>{
  const d=e.data;
  if(d.t==="u"){if(d.url!=null)B.url=d.url;if(d.title!=null)B.title=d.title;if(d.loading!=null)B.loading=d.loading;upd()}
});

url.addEventListener("keydown",e=>{if(e.key==="Enter"&&url.value.trim()){nav(url.value.trim());url.blur()}});
document.addEventListener("keydown",e=>{
  const c=e.ctrlKey||e.metaKey;
  if((c&&e.key.toLowerCase()==="l")||e.key==="F6"){e.preventDefault();url.focus();url.select()}
  else if((c&&e.key.toLowerCase()==="r")||e.key==="F5"){e.preventDefault();rl()}
  else if(e.key==="Escape"){if(document.activeElement===url)url.blur();else st()}
});
$("bk").onclick=goBack;$("fw").onclick=goFwd;$("rl").onclick=rl;$("hm").onclick=home;
upd();
</script></body></html>"#;

struct Browser {
    hwnd: HWND,
    env: Option<ICoreWebView2Environment>,
    chrome_ctl: Option<ICoreWebView2Controller>,
    chrome_wv: Option<ICoreWebView2>,
    content_ctl: Option<ICoreWebView2Controller>,
    content_wv: Option<ICoreWebView2>,
    content_url: String,
    content_title: String,
    content_loading: bool,
}

unsafe impl Send for Browser {}
unsafe impl Sync for Browser {}

fn get_browser(hwnd: HWND) -> Option<&'static mut Browser> {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        None
    } else {
        Some(unsafe { &mut *(ptr as *mut Browser) })
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

unsafe fn push_update(hwnd: HWND) {
    let br = match get_browser(hwnd) {
        Some(b) => b,
        None => return,
    };
    let wv = match &br.chrome_wv {
        Some(w) => w,
        None => return,
    };
    let url = br.content_url.clone();
    let title = br.content_title.clone();
    let loading = br.content_loading;
    let json = format!(
        "{{\"t\":\"u\",\"url\":\"{}\",\"title\":\"{}\",\"loading\":{}}}",
        escape_json(&url),
        escape_json(&title),
        loading
    );
    let json_wide = widen(&json);
    let _ = wv.PostWebMessageAsJson(PCWSTR(json_wide.as_ptr()));
}

fn widen(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn size_controllers(hwnd: HWND) {
    let mut rect = RECT::default();
    let _ = GetClientRect(hwnd, &mut rect);
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let ch = CHROME_HEIGHT;
    let content_h = height.saturating_sub(ch);
    if let Some(br) = get_browser(hwnd) {
        if let Some(ctl) = &br.chrome_ctl {
            let _ = ctl.SetBounds(RECT { left: 0, top: 0, right: width, bottom: ch });
        }
        if let Some(ctl) = &br.content_ctl {
            let _ = ctl.SetBounds(RECT { left: 0, top: ch, right: width, bottom: ch + content_h });
        }
    }
}

unsafe fn init_webview2(hwnd: HWND) {
    let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(move |_, env| {
        let env = match env {
            Some(e) => e,
            None => return Ok(()),
        };

        if let Some(br) = get_browser(hwnd) {
            br.env = Some(env.clone());
        }

        let chrome_handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |_, controller| {
                let controller = match controller {
                    Some(c) => c,
                    None => return Ok(()),
                };
                let wv = match controller.CoreWebView2() {
                    Ok(w) => w,
                    Err(_) => return Ok(()),
                };

                {
                    let br = get_browser(hwnd).unwrap();
                    br.chrome_ctl = Some(controller);
                    br.chrome_wv = Some(wv.clone());
                }

                size_controllers(hwnd);

                let html_wide = widen(CHROME_HTML);
                let _ = wv.NavigateToString(PCWSTR(html_wide.as_ptr()));

                let mut token: i64 = 0;
                let hwnd2 = hwnd;
                let _ = wv.add_WebMessageReceived(
                    &WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                        if let Some(args) = args {
                            let mut pwstr = PWSTR::null();
                            if args.WebMessageAsJson(&mut pwstr).is_ok() {
                                let msg = take_pwstr(pwstr);
                                handle_chrome_message(&msg, hwnd2);
                            }
                        }
                        Ok(())
                    })),
                    &mut token,
                );

                let env = get_browser(hwnd).unwrap().env.as_ref().unwrap().clone();
                let content_handler = CreateCoreWebView2ControllerCompletedHandler::create(
                    Box::new(move |_, controller| {
                        let controller = match controller {
                            Some(c) => c,
                            None => return Ok(()),
                        };
                        let wv = match controller.CoreWebView2() {
                            Ok(w) => w,
                            Err(_) => return Ok(()),
                        };

                        {
                            let br = get_browser(hwnd).unwrap();
                            br.content_ctl = Some(controller);
                            br.content_wv = Some(wv.clone());
                        }

                        size_controllers(hwnd);

                        let blank = widen("about:blank");
                        let _ = wv.Navigate(PCWSTR(blank.as_ptr()));

                        let mut token2: i64 = 0;
                        let hwnd3 = hwnd;
                        let _ = wv.add_NavigationStarting(
                            &NavigationStartingEventHandler::create(Box::new(
                                move |_, args| {
                                    if let Some(args) = args {
                                        let mut pwstr = PWSTR::null();
                                        if args.Uri(&mut pwstr).is_ok() {
                                            let url = take_pwstr(pwstr);
                                            if let Some(br) = get_browser(hwnd3) {
                                                br.content_url = url;
                                                br.content_loading = true;
                                                push_update(hwnd3);
                                            }
                                        }
                                    }
                                    Ok(())
                                },
                            )),
                            &mut token2,
                        );

                        let mut token3: i64 = 0;
                        let hwnd4 = hwnd;
                        let _ = wv.add_NavigationCompleted(
                            &NavigationCompletedEventHandler::create(Box::new(
                                move |_, _args| {
                                    if let Some(br) = get_browser(hwnd4) {
                                        br.content_loading = false;
                                        push_update(hwnd4);
                                    }
                                    Ok(())
                                },
                            )),
                            &mut token3,
                        );

                        let mut token4: i64 = 0;
                        let hwnd5 = hwnd;
                        let _ = wv.add_DocumentTitleChanged(
                            &DocumentTitleChangedEventHandler::create(Box::new(
                                move |_, _args| {
                                    if let Some(br) = get_browser(hwnd5) {
                                        let wv = br.content_wv.as_ref().unwrap();
                                        let mut pwstr = PWSTR::null();
                                        if wv.DocumentTitle(&mut pwstr).is_ok() {
                                            br.content_title = take_pwstr(pwstr);
                                            push_update(hwnd5);
                                        }
                                    }
                                    Ok(())
                                },
                            )),
                            &mut token4,
                        );

                        Ok(())
                    }),
                );

                let _ = env.CreateCoreWebView2Controller(hwnd, &content_handler);

                Ok(())
            },
        ));

        let _ = env.CreateCoreWebView2Controller(hwnd, &chrome_handler);

        Ok(())
    }));

    let _ = CreateCoreWebView2Environment(&handler);
}

unsafe fn handle_chrome_message(json: &str, hwnd: HWND) {
    let br = match get_browser(hwnd) {
        Some(b) => b,
        None => return,
    };
    let wv = match &br.content_wv {
        Some(w) => w,
        None => return,
    };

    if json.contains("\"navigate\"") {
        if let Some(start) = json.find("\"url\":\"") {
            let s = start + 7;
            if let Some(end) = json[s..].find('"') {
                let url = &json[s..s + end];
                let url_wide = widen(url);
                let _ = wv.Navigate(PCWSTR(url_wide.as_ptr()));
                br.content_url = url.to_string();
                br.content_loading = true;
                push_update(hwnd);
            }
        } else if let Some(start) = json.find("\"url\": \"") {
            let s = start + 8;
            if let Some(end) = json[s..].find('"') {
                let url = &json[s..s + end];
                let url_wide = widen(url);
                let _ = wv.Navigate(PCWSTR(url_wide.as_ptr()));
                br.content_url = url.to_string();
                br.content_loading = true;
                push_update(hwnd);
            }
        }
    } else if json.contains("\"go_back\"") {
        let _ = wv.GoBack();
    } else if json.contains("\"go_forward\"") {
        let _ = wv.GoForward();
    } else if json.contains("\"reload\"") {
        let _ = wv.Reload();
    } else if json.contains("\"stop\"") {
        let _ = wv.Stop();
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_SIZE => {
            let width = (lparam.0 & 0xFFFF) as i32;
            let height = ((lparam.0 >> 16) & 0xFFFF) as i32;
            if let Some(br) = get_browser(hwnd) {
                let ch = CHROME_HEIGHT;
                let content_h = height.saturating_sub(ch);
                if let Some(ctl) = &br.chrome_ctl {
                    let _ = ctl.SetBounds(RECT {
                        left: 0,
                        top: 0,
                        right: width,
                        bottom: ch,
                    });
                }
                if let Some(ctl) = &br.content_ctl {
                    let _ = ctl.SetBounds(RECT {
                        left: 0,
                        top: ch,
                        right: width,
                        bottom: ch + content_h,
                    });
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                drop(Box::from_raw(ptr as *mut Browser));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn main() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        let _ = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);

        let instance = GetModuleHandleW(None).unwrap();

        let class_name = widen("J1Window");
        let window_title = widen("J1");

        let brush = GetStockObject(BLACK_BRUSH);

        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH(brush.0),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        RegisterClassExW(&class);

        let hwnd = CreateWindowExW(
            WS_EX_OVERLAPPEDWINDOW,
            PCWSTR(class_name.as_ptr()),
            PCWSTR(window_title.as_ptr()),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1024,
            768,
            None,
            None,
            Some(HINSTANCE(instance.0 as _)),
            None,
        )
        .unwrap();

        let browser = Box::new(Browser {
            hwnd,
            env: None,
            chrome_ctl: None,
            chrome_wv: None,
            content_ctl: None,
            content_wv: None,
            content_url: String::new(),
            content_title: "J1".to_string(),
            content_loading: false,
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(browser) as isize);

        init_webview2(hwnd);

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        CoUninitialize();
    }
}
