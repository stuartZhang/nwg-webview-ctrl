#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(debug_assertions, feature(trace_macros, log_syntax))]

use ::clap::Parser;
use ::futures::{FutureExt, executor::LocalPool, task::LocalSpawnExt};
use ::nwg::{self as nwg, GridLayout, Icon, Monitor, NativeUi, Window};
use ::nwd::NwgUi;
use ::nwg_webview_ctrl::{WebviewContainer, WebviewContainerFlags};
use ::std::error::Error;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliParams {
    #[arg(short, long, value_name = "URL")]
    url: String
}
#[derive(Default, NwgUi)]
pub struct DemoUi {
    #[nwg_resource(
        source_bin: Some(include_bytes!("../asserts/images/favicon.ico")),
        size: Some((16, 16)),
        strict: true
    )]
    app_icon: Icon,
    #[nwg_control(
        size: DemoUi::SIZE,
        position: DemoUi::position(),
        icon: Some(&data.app_icon),
        title: "内嵌 WebView 例程",
        flags: "MAIN_WINDOW|VISIBLE"
    )]
    #[nwg_events(OnWindowClose: [nwg::stop_thread_dispatch()])]
    window: Window,
    #[nwg_layout(
        margin: [0; 4],
        parent: window,
        max_column: Some(1),
        max_row: Some(1),
        spacing: 0
    )]
    grid: GridLayout,
    #[nwg_control(
        flags: "VISIBLE",
        parent: window,
        window: &data.window,
        language: "en_us"
    )]
    #[nwg_layout_item(layout: grid, row: 0, col: 0)]
    webview_container: WebviewContainer,
}
impl DemoUi {
    /// 主窗体大小
    const SIZE:(i32, i32) = (1024, 168);
    /// 主窗体初始显示位置
    fn position() -> (i32, i32) {
        ((Monitor::width() - Self::SIZE.0) / 2, (Monitor::height() - Self::SIZE.1) / 2)
    }
    /// 业务处理逻辑封装成员方法
    fn executor(&self, cli_params: CliParams) -> Result<LocalPool, Box<dyn Error>> {
        let executor = LocalPool::new();
        let webview_ready_fut = self.webview_container.ready_fut()?;
        executor.spawner().spawn_local(async move {
            let (_, _, webview) = webview_ready_fut.await;
            webview.navigate(&cli_params.url)?;
            Ok::<_, Box<dyn Error>>(())
        }.map(|result| {
            if let Err(err) = result {
                eprintln!("[app_main]{err}");
            }
        }))?;
        Ok(executor)
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let cli_params = CliParams::parse();
    nwg::init()?;
    // 主窗体
    let demo_ui_app = DemoUi::build_ui(Default::default())?;
    // 业务处理逻辑
    let mut executor = demo_ui_app.executor(cli_params)?;
    // 阻塞主线程，等待用户手动关闭主窗体
    nwg::dispatch_thread_events_with_callback(move ||
        // 以 win32 UI 的事件循环为【反应器】，对接 futures crate 的【执行器】
        executor.run_until_stalled());
    Ok(())
}
