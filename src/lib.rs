mod builder;
use ::deferred_future::LocalDeferredFuture;
use ::futures::{future::Shared, executor};
use ::nwg::{self as nwg, ControlHandle, EventHandler, Frame, NwgError, RawEventHandler};
use ::std::{cell::RefCell, ops::Deref, rc::Rc};
use ::webview2::{Controller, Environment, WebView};
pub use builder::{WebviewContainerBuilder, WebviewContainerFlags};

pub type NwgResult<T> = Result<T, NwgError>;
/// [webview2::Controller](https://docs.rs/webview2/0.1.4/webview2/struct.Controller.html)的[NWG](https://docs.rs/native-windows-gui/1.0.1/native_windows_gui/index.html)控件包装器。借助于该`WebviewContainer`包装器，`webview2::Controller`控件实例就能参与`NWG`的【网格布局】【弹性布局】与【动态布局】。
/// # 原理：
/// 1. `WebviewContainer`将`webview2::Controller`嵌套于[nwg::Frame](https://docs.rs/native-windows-gui/1.0.1/native_windows_gui/struct.Frame.html)控件内，以参与控件布局管理。
/// 2. 监听主窗体的【最小化】事件。在主窗体被最小化之后，停止`webview2::Controller`控件对打开网页的帧刷新。即，将网页的`FPS`降到零。
/// 3. 监听主窗体的【窗体恢复】事件。在主窗体非最小化状态，恢复`webview2::Controller`控件对打开网页的帧刷新。
/// 4. 监听主窗体的【移动】事件。仅只透传窗体的最新屏幕坐标给底层的`webview2::Controller`控件。
/// 5. 监听`nwg::Frame`控件的`OnResize`事件。时刻拉伸或压缩`webview2::Controller`的大小。
/// # `webview2::Controller`的初始化
/// `webview2::Controller`初始化是异步的。所以在[`WebviewContainerBuilder::build()`]被同步执行结束之后，仅只`nwg::Frame`布局占位控件被初始化好了。而，`webview2::Controller`的初始化就绪需要等待由[`WebviewContainer.ready_fut()`]成员方法返回的`Future`
#[derive(Default)]
pub struct WebviewContainer {
    is_closing: Rc<RefCell<bool>>,
    frame: Rc<RefCell<Frame>>,
    webview_ctrl: Rc<RefCell<Option<Controller>>>,
    ready_fut: Option<Shared<LocalDeferredFuture<(Environment, Controller, WebView)>>>,
    event_handle: Option<EventHandler>,
    raw_event_handle: Option<RawEventHandler>
}
impl PartialEq for WebviewContainer {
    fn eq(&self, other: &Self) -> bool {
        self.frame.borrow().eq(other.frame.borrow().deref())
    }
}
impl Eq for WebviewContainer {}
impl From<WebviewContainer> for ControlHandle {
    fn from(value: WebviewContainer) -> Self {
        value.frame.borrow().handle
    }
}
impl From<&WebviewContainer> for ControlHandle {
    fn from(value: &WebviewContainer) -> Self {
        value.frame.borrow().handle
    }
}
impl Drop for WebviewContainer {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        println!("[WebviewContainer][drop]");
        *self.is_closing.borrow_mut() = true;
        if self.raw_event_handle.as_ref().map(nwg::unbind_raw_event_handler).map_or(Ok(None), |r| r.map(Some)).is_ok() {
            self.event_handle.as_ref().map(nwg::unbind_event_handler);
        }
        self.webview_ctrl.borrow().as_ref().map(|webview_ctrl| {
            webview_ctrl.close().map_err(|err| eprintln!("[WebviewContainer][drop]{err}")).ok()
        });
        self.frame.borrow_mut().handle.destroy();
    }
}
impl WebviewContainer {
    pub fn builder<'a>() -> WebviewContainerBuilder<'a> {
        WebviewContainerBuilder::default()
    }
    pub fn ready_fut(&self) -> NwgResult<Shared<LocalDeferredFuture<(Environment, Controller, WebView)>>> {
        self.ready_fut.clone().ok_or(NwgError::control_create("Webview 控件初始化失败或还未被初始化"))
    }
    pub fn ready_block(&self) -> NwgResult<(Environment, Controller, WebView)> {
        Ok(executor::block_on(self.ready_fut()?))
    }
}
