use ::bitflags::bitflags;
use ::deferred_future::LocalDeferredFuture;
use ::futures::FutureExt;
use ::nwg::{self as nwg, ControlHandle, Event as NwgEvent, Frame, FrameBuilder, FrameFlags, NwgError};
use ::webview2::{Controller, Environment, EnvironmentBuilder, Result as WvResult};
use ::std::{cell::RefCell, path::Path, mem, rc::Rc, sync::atomic::{AtomicUsize, Ordering}};
use ::winapi::{shared::windef::{HWND, RECT}, um::winuser::{GetClientRect, SC_RESTORE, WM_SYSCOMMAND, WS_BORDER, WS_DISABLED, WS_VISIBLE}};
use super::{NwgResult, WebviewContainer};

static HANDLE_ID: AtomicUsize = AtomicUsize::new(0xffff + 1);

bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct WebviewContainerFlags: u32 {
        const NONE = 0;
        const VISIBLE = WS_VISIBLE;
        const DISABLED = WS_DISABLED;
        const BORDER = WS_BORDER;
    }
}
pub struct WebviewContainerBuilder<'a> {
    window: Option<ControlHandle>,
    webview_env: Option<Environment>,
    webview_env_builder: EnvironmentBuilder<'a>,
    frame_builder: FrameBuilder
}
impl<'a> Default for WebviewContainerBuilder<'a> {
    fn default() -> Self {
        Self {
            window: None,
            webview_env: None,
            webview_env_builder: Environment::builder(),
            frame_builder: Frame::builder()
        }
    }
}
impl<'a> WebviewContainerBuilder<'a> {
    /// nwg::FrameBuilder 的配置项
    pub fn flags(mut self, flags: WebviewContainerFlags) -> WebviewContainerBuilder<'a> {
        let mut frame_flags = FrameFlags::NONE;
        if flags.contains(WebviewContainerFlags::BORDER) {
            frame_flags = frame_flags | FrameFlags::BORDER;
        }
        if flags.contains(WebviewContainerFlags::DISABLED) {
            frame_flags = frame_flags | FrameFlags::DISABLED;
        }
        if flags.contains(WebviewContainerFlags::VISIBLE) {
            frame_flags = frame_flags | FrameFlags::VISIBLE;
        }
        self.frame_builder = self.frame_builder.flags(frame_flags);
        self
    }
    /// nwg::FrameBuilder 的配置项
    pub fn size(mut self, size: (i32, i32)) -> WebviewContainerBuilder<'a> {
        self.frame_builder = self.frame_builder.size(size);
        self
    }
    /// nwg::FrameBuilder 的配置项
    pub fn position(mut self, pos: (i32, i32)) -> WebviewContainerBuilder<'a> {
        self.frame_builder = self.frame_builder.position(pos);
        self
    }
    /// nwg::FrameBuilder 的配置项
    pub fn enabled(mut self, e: bool) -> WebviewContainerBuilder<'a> {
        self.frame_builder = self.frame_builder.enabled(e);
        self
    }
    /// nwg::FrameBuilder 的配置项
    pub fn parent<C: Into<ControlHandle>>(mut self, p: C) -> WebviewContainerBuilder<'a> {
        self.frame_builder = self.frame_builder.parent(p);
        self
    }
    /// webview2::EnvironmentBuilder 的配置项
    pub fn browser_executable_folder(mut self, browser_executable_folder: &'a Path) -> Self {
        self.webview_env_builder = self.webview_env_builder.with_browser_executable_folder(browser_executable_folder);
        self
    }
    /// webview2::EnvironmentBuilder 的配置项
    pub fn user_data_folder(mut self, user_data_folder: &'a Path) -> Self {
        self.webview_env_builder = self.webview_env_builder.with_user_data_folder(user_data_folder);
        self
    }
    /// webview2::EnvironmentBuilder 的配置项
    pub fn additional_browser_arguments(mut self, additional_browser_arguments: &'a str) -> Self {
        self.webview_env_builder = self.webview_env_builder.with_additional_browser_arguments(additional_browser_arguments);
        self
    }
    /// webview2::EnvironmentBuilder 的配置项
    pub fn language(mut self, language: &'a str) -> Self {
        self.webview_env_builder = self.webview_env_builder.with_language(language);
        self
    }
    /// webview2::EnvironmentBuilder 的配置项
    pub fn target_compatible_browser_version(mut self, target_compatible_browser_version: &'a str) -> Self {
        self.webview_env_builder = self.webview_env_builder.with_target_compatible_browser_version(target_compatible_browser_version);
        self
    }
    /// webview2::EnvironmentBuilder 的配置项
    pub fn allow_single_sign_on_using_osprimary_account(mut self, allow_single_sign_on_using_osprimary_account: bool) -> Self {
        self.webview_env_builder = self.webview_env_builder.with_allow_single_sign_on_using_osprimary_account(allow_single_sign_on_using_osprimary_account);
        self
    }
    /// webview2::EnvironmentBuilder 的配置项。
    /// 获取当前浏览器实例版本字符串
    pub fn get_available_browser_version_string(&self) -> WvResult<String> {
        self.webview_env_builder.get_available_browser_version_string()
    }
    // 其它
    pub fn window<C: Into<ControlHandle>>(mut self, window: C) -> WebviewContainerBuilder<'a> {
        self.window = Some(window.into());
        self
    }
    pub fn webview_env<E: Into<Environment>>(mut self, webview_env: E) -> WebviewContainerBuilder<'a> {
        self.webview_env = Some(webview_env.into());
        self
    }
    /// 1. 在多 TAB 应用程序场景下，重用`webview2::Environment(i.e. CoreWebView2Environment)`实例。
    ///    于是，由相同`CoreWebView2Environment`实例构造的多`webview`将共用相同的
    ///     1. 浏览器进程
    ///     2. 渲染进程
    ///     3. 缓存目录
    /// 2. 需要深度定制 CoreWebView2Environment 实例。比如，
    ///     1. 默认语言
    ///     2. 缓存目录
    ///     3. 浏览器启动参数
    ///     4. 浏览器版本号
    ///     5. 浏览器安装目录
    ///     6. 是否允许单点登录
    pub fn build(self, webview_container: &mut WebviewContainer) -> NwgResult<()> {
        // 主窗体
        let window_handle = self.window.ok_or(NwgError::initialization("window 配置项代表了主窗体。它是必填项"))?;
        let window_hwnd = window_handle.hwnd().ok_or(NwgError::control_create("主窗体不是有效的 Win32 COM 控件"))?;
        // webview 容器
        self.frame_builder.build(&mut webview_container.frame.borrow_mut())?;
        let frame_hwnd = webview_container.frame.borrow().handle.hwnd().ok_or(NwgError::control_create("Webview 容器控件 Frame 初始化失败"))?;
        macro_rules! unpack {
            ($variable: ident, $return: expr) => {
                match $variable.upgrade() {
                    Some(variable) => variable,
                    None => return $return
                }
            };
        }
        // webview 组件构造异步锁
        webview_container.ready_fut.replace({
            let webview_ctrl = Rc::clone(&webview_container.webview_ctrl);
            let webview_ready_future = LocalDeferredFuture::default();
            let defer = webview_ready_future.defer();
            let frame = Rc::clone(&webview_container.frame);
            let build = move |env: Environment| env.clone().create_controller(frame_hwnd, move |webview_ctrl_core| {
                let webview_ctrl_core = webview_ctrl_core?;
                let webview = webview_ctrl_core.get_webview()?;
                align_webview_2_container(&webview_ctrl_core, frame, frame_hwnd)?;
                #[cfg(debug_assertions)]
                println!("[WebviewContainerBuilder][build]Webview 实例化成功");
                webview_ctrl.borrow_mut().replace(webview_ctrl_core.clone());
                defer.borrow_mut().complete((env, webview_ctrl_core, webview));
                Ok(())
            });
            if let Some(webview_env) = self.webview_env {
                build(webview_env)
            } else {
                self.webview_env_builder.build(move |env| build(env?))
            }.map(|_| webview_ready_future.shared()).map_err(|err|NwgError::control_create(err.to_string()))
        }?);
        webview_container.event_handle.replace({ // 因为【主窗体】直接就是 webview 的父组件，所以传递主窗体的事件给 webview 组件。
            let webview_ctrl = Rc::downgrade(&webview_container.webview_ctrl);
            let is_closing = Rc::downgrade(&webview_container.is_closing);
            let frame = Rc::downgrade(&webview_container.frame);
            nwg::full_bind_event_handler(&window_handle, move |event, _data, handle| {
                let is_closing = unpack!(is_closing, ());
                if *is_closing.borrow() {
                    return;
                }
                if let ControlHandle::Hwnd(hwnd) = handle {
                    if window_hwnd == hwnd { // 事件源是主窗体
                        let webview_ctrl = unpack!(webview_ctrl, ());
                        match event {
                            // 当主窗体被最小化时，关闭 webview 组件，以减小空耗。
                            NwgEvent::OnWindowMinimize => webview_ctrl.borrow().as_ref().and_then(|controller| {
                                #[cfg(debug_assertions)]
                                println!("[WebviewContainer][OnWindowMinimize]Webview 被挂起了");
                                controller.put_is_visible(false).map_err(|err| eprintln!("[OnWindowMinimize]{err}")).ok()
                            }),
                            // 当主窗体被移动时，徒手传递位移事件给 webview 组件。
                            NwgEvent::OnMove => webview_ctrl.borrow().as_ref().and_then(|controller|
                                controller.notify_parent_window_position_changed().map_err(|err| eprintln!("[OnMove]{err}")).ok()
                            ),
                            _ => Some(())
                        };
                    } else if frame_hwnd == hwnd { // 事件源是 webview 容器 Frame
                        let webview_ctrl = unpack!(webview_ctrl, ());
                        match event {
                            NwgEvent::OnResize => { // 当主窗体被调整大小时，徒手传递尺寸调整事件给 webview 组件。
                                let frame = unpack!(frame, ());
                                webview_ctrl.borrow().as_ref().and_then(move |controller| {
                                    align_webview_2_container(controller, frame, frame_hwnd).map_err(|err| eprintln!("[OnResize|OnWindowMaximize]{err}")).ok()
                                })
                            },
                            NwgEvent::OnMove => webview_ctrl.borrow().as_ref().and_then(|controller|
                                controller.notify_parent_window_position_changed().map_err(|err| eprintln!("[OnMove]{err}")).ok()
                            ),
                            _ => Some(())
                        };
                    }
                }
            })
        });
        webview_container.raw_event_handle.replace({ // nwg 封闭里漏掉了【主窗体】的 restore 事件，所以这里直接经由 winapi crate 的原始接口挂事件处理函数了。
            let handle_id = loop {
                let handle_id = HANDLE_ID.fetch_add(1, Ordering::Relaxed);
                if !nwg::has_raw_handler(&window_handle, handle_id) {
                    break handle_id;
                }
            };
            let webview_ctrl = Rc::downgrade(&webview_container.webview_ctrl);
            let is_closing = Rc::downgrade(&webview_container.is_closing);
            nwg::bind_raw_event_handler(&window_handle, handle_id, move |_, msg, w, _| {
                let webview_ctrl = unpack!(webview_ctrl, None);
                let is_closing = unpack!(is_closing, None);
                if !*is_closing.borrow() && (WM_SYSCOMMAND, SC_RESTORE) == (msg, w as usize) {
                    #[cfg(debug_assertions)]
                    println!("[WebviewContainer][OnWindowMinimize]Webview 被恢复了");
                    webview_ctrl.borrow().as_ref().and_then(|controller| // 当主窗体被还原时，打开 webview 组件。
                        controller.put_is_visible(true).map_err(|err| eprintln!("[OnWindowRestore]{err}")).ok()
                    );
                }
                None
            })?
        });
        #[cfg(debug_assertions)]
        println!("[WebviewContainerBuilder][build]同步执行结束");
        Ok(())
    }
}
/// 调整 webview 控件的大小·至·包含该 webview 控件的容器元素的最新大小
fn align_webview_2_container(webview_ctrl: &Controller, frame: Rc<RefCell<Frame>>, frame_hwnd: HWND) -> WvResult<()> {
    let (successful, mut rect) = unsafe {
        let mut rect = mem::zeroed();
        let successful = GetClientRect(frame_hwnd, &mut rect);
        (successful, rect)
    };
    if successful == 0 {
        let position = frame.borrow().position();
        let size = frame.borrow().size();
        rect = RECT {
            top: position.1,
            left: position.0,
            right: size.0 as i32,
            bottom: size.1 as i32
        }
    }
    println!("rect={{top: {}, left: {}, width: {}, height: {} }}", rect.top, rect.left, rect.right, rect.bottom);
    return webview_ctrl.put_bounds(rect);
}