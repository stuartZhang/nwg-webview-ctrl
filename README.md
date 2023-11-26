# nwg-webview-ctrl

封装[Microsoft Edge WebView2](https://learn.microsoft.com/en-us/microsoft-edge/webview2/)浏览器内核为[Native Windows GUI (i.e. NWG crate)](https://gabdube.github.io/native-windows-gui/native-windows-docs/index.html)开发框架的`WebView`图形控件 — 具体包括

1. `WebviewContainer`自定义控件和
2. `WebviewContainerBuilder`控件构建器。

进而，给`Rust Win32 Bindings`增添`WebView`图形控件新成员。

相较人气爆棚的[Tauri crate](https://github.com/tauri-apps/tauri)，`nwg-webview-ctrl`允许`WebView`参与原生图形控件的[布局管理](https://gabdube.github.io/native-windows-gui/native-windows-docs/layouts.html)，包括但不限于：

   1. 网格布局`GridLayout`
   2. 弹性布局`FlexboxLayout`
   3. 动态布局`DynLayout`

`WebviewContainer`图形控件的功能定位等同于[OSX Cacao crate](https://docs.rs/cacao/0.3.2/cacao/index.html)图形界面开发框架中的[cacao::webview::WebView](https://docs.rs/cacao/0.3.2/cacao/webview/struct.WebView.html)控件。它们都力图凭借构建**功能丰富**的**原生图形界面**，重塑**原生图形交互**在应用程序中的【主体地位】，而不只是陪衬作为`H5`网页程序的套壳浏览器“附属品”（— 至多也就是位“收租公”）。要说有差别，那也仅是

* `cacao::webview::WebView`封装的是`Apple Webkit Webview`
* `WebviewContainer`套壳的是`Microsoft Edge Webview2`

## 运行环境要求

1. 预安装`Microsoft Edge 86+`浏览器的`Windows`操作系统。或
2. 已安装[Evergreen WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/consumer)的其它版本`Windows`操作系统。

一般来讲，`Windows 11`与打过升级补丁的`Windows 10`都可以直接运行包含了此图形控件的应用程序。

## 编译环境要求

`Cargo Package`配置已经锁定了`nightly-x86_64-pc-windows-msvc`工具链。虽然`stable channel`工具链也可成功编译，但工具链的`GNU (ABI) build`却会导致编译时链接`WebView2 Runtime`失败。

此外，编译环境也需要给`Windows`操作系统预安装`Microsoft Edge 86+`或`Evergreen WebView2 Runtime`。

## 我的贡献

`nwg-webview-ctrl crate`并没有直接调用浏览器内核的[Win32 COM ABI](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/environment-controller-core?tabs=dotnetcsharp)。而是，站在巨人的肩膀上，将[webview2 crate](https://github.com/blckngm/webview2/tree/master)套壳封装于`NWG`图形开发框架的[nwg::Frame](https://docs.rs/native-windows-gui/1.0.1/native_windows_gui/struct.Frame.html)控件内。最后，以`nwg::Frame`控件为“代理”参与原生控件布局。

> 题外话，`nwg::Frame`控件本就是`NWG`图形开发框架对第三方扩展提供的“接入插槽”。`nwg-webview-ctrl crate`也算是第三方扩展了。

以图描述更直观，一图抵千词。

![封装嵌套层](https://github.com/stuartZhang/my_rs_ideas_playground/assets/13935927/c2bb12e0-c09a-4232-ab78-2bb307fd668b)

## `Webview`初始化是异步的

这不是我定的，而是从`Win32 COM`那一层开始就已经是**异步回调**了。然后，再经由`webview2 crate`透传至`nwg-webview-ctrl`封装代码。但，`WebviewContainer`还是做了些使生活更美好的工作 — 将【异步回调】变形为

* 要么，[futures::future::FusedFuture](https://docs.rs/futures/latest/futures/future/trait.FusedFuture.html)的异步阻塞

   ```rust
   // 这是【伪码】呀！真实的【返回值】类型会更复杂，但本质如下。
   WebviewContainer::ready_fut(&self) -> FusedFuture<Output = (Environment, Controller, WebView)>
   ```

   将该成员方法返回值直接注入【异步块`Task`】。再将`NWG`事件循环作为【反应器`Reactor`】对接`futures crate`的【执行器`Executor`】，以持续轮询推进【异步块`Task`】的程序执行。

   ![异步工作原理](https://github.com/stuartZhang/my_rs_ideas_playground/assets/13935927/cd0e8314-e590-4c12-99c9-4b18d02a4342)

   这是我非常推荐的用法，也是`examples`采用的代码套路。

* 要么，[futures::executor::block_on(Future)](https://docs.rs/futures/latest/futures/executor/fn.block_on.html)的同步阻塞

   ```rust
   // 这是【伪码】呀！真实的【返回值】类型会更复杂，但本质如下。
   WebviewContainer::ready_block(&self) -> (Environment, Controller, WebView)
   ```

   该成员方法内部会调用`futures::executor::block_on()`阻塞当前线程。特别注意：该成员方法仅能在**同步**上下文中被调用。否则，会导致应用程序运行崩溃！

虽然`Webview`初始化是异步的，但`WebviewContainerBuilder`控件构造器自身却**未**执行任何（同/异步）阻塞操作，而仅只

1. 构造`nwg::Frame`控件**占位**原生布局流
2. 开启`webview2::Controller(i.e.` [Microsoft.Web.WebView2.Core.CoreWebView2Controller](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2controller?view=webview2-dotnet-1.0.2151.40)`)`异步初始化流程，却**不等待**初始化处理结束

然后，由`WebviewContainer`控件“亲自”阻塞程序执行，和等待`Webview`完全就绪

### 同步阻塞

除了简单，啥也不是。

```rust
let mut webview_container = WebviewContainer::default();
// builder 自身是不阻塞的
WebviewContainer::builder().parent(&window).window(&window).build(&mut webview_container)?;
// 由控件对象的成员方法阻塞主线程，和等待 Webview 完全就绪
let (_, _, webview) = webview_container.ready_block().unwrap();
webview.navigate("https://www.minxing365.com").unwrap();
```

### 异步阻塞

仅四步便点亮`Native GUI`【异步编程】科技树 — 绝对值得拥有：

1. 构造一个异步任务`Task`
2. 构造一个单线程异步执行器`Executor`
3. 将异步执行器对接`NWG`事件循环，和将`NWG`事件循环作为`Reactor`
4. 将`Webview`初始化`FusedFuture`对象捕获入异步任务`Task`

```rust
let mut webview_container = WebviewContainer::default();
// builder 自身是不阻塞的
WebviewContainer::builder()
   .parent(&window) // nwg::Frame 控件的父控制是主窗体 window
   .window(&window) // webview2::Controller 的关联主窗体也是相同的 window
   .build(&mut webview_container).unwrap();
// 1. 构造一个异步任务
let webview_ready_fut = webview_container.ready_fut().unwrap();
// 2. 构造一个单线程异步执行器
let mut executor = {
   let executor = LocalPool::new();
   executor.spawner().spawn_local(async move {
      // 4. 将异步任务注入异步执行器
      let (_, _, webview) = webview_ready_fut.await;
      webview.navigate("https://www.minxing365.com").unwrap();
      Ok::<_, Box<dyn Error>>(())
   }).unwrap();
   executor
};
// 3. 将异步执行器对接`NWG`事件循环
nwg::dispatch_thread_events_with_callback(move || executor.run_until_stalled());
```

## `Webview`初始化成功的返回值

返回值是三元素元组。其三个子元素依次是

1. `webview2::Environment(i.e.` [Microsoft.Web.WebView2.Core.CoreWebView2Environment](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environment?view=webview2-dotnet-1.0.2151.40)`)`

   在多`TAB`签场景下，此返回值允许多个`webview2::Controller`实例共享同一个`webview2::Environment`构造源。于是，多个同源`webview2::Controller`实例就能共用一套
     * 浏览器进程
     * 渲染进程
     * 缓存目录

2. `webview2::Controller(i.e.` [Microsoft.Web.WebView2.Core.CoreWebView2Controller](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2controller?view=webview2-dotnet-1.0.2151.40)`)`

   面向整个应用程序中的原生部分，实现
     * 焦点传递
     * `DPI`级的整体缩放
     * 改变整体背景色
     * 挂起/恢复渲染进程。`WebviewContainer`控件内部正在调用该接口，并
       * 在主窗体被隐藏`NwgEvent::OnWindowMinimize`时，挂起`Webview`渲染进程
       * 在主窗体被弹出`(WM_SYSCOMMAND, SC_RESTORE)`时，恢复`Webview`渲染进程
     * 同步发送主窗体的`UI`状态信息给`CoreWebView2Controller`。`WebviewContainer`控件内部就正在监听
       * 主窗体的位移事件`NwgEvent::OnMove`
       * `nwg::Frame`父控件的
         * 尺寸变化事件`NwgEvent::OnResize`
         * 位移事件`NwgEvent::OnMove`

       和传递最新的位置与尺寸信息给`CoreWebView2Controller`。

     * 析构掉整个`Webview`控件（包括`CoreWebView2Controller`和`CoreWebView2`）。`WebviewContainer`控件已被实现为底层`Webview`控件的`RAII`守卫。即，只要`WebviewContainer`控件被析构，那么`Webview`控件也将同步地被释放。

3. `webview2::WebView(i.e.` [Microsoft.Web.WebView2.Core.CoreWebView2](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2?view=webview2-dotnet-1.0.2151.40)`)`

   面向应用程序中网页部分，实现
      * `native <-> javascript`桥
      * 直接操控网页内容
      * 定制与替换浏览器弹出对话框。
      * 拦截与篡改网络请求
      * 拦截与篡改网页路由

目前，被用于布局占位的`nwg::Frame`控件实例还尚未对外可见。

## `WebviewContainer`的构造与配置

`WebviewContainer`控件支持`API`与【派生宏】两种实例化方式

### `API`实例化模式

```rust
// 构造主窗体
let mut window = Window::default();
// 配置主窗体
Window::builder().title("内嵌 WebView 例程").size((1024, 168)).build(&mut window)?;
nwg::full_bind_event_handler(&window.handle, move |event, _data, _handle| {
    if let NwgEvent::OnWindowClose = event { // 关闭主窗体。
        nwg::stop_thread_dispatch();
    }
});
// 构造 Webview 控件
let mut webview_container = WebviewContainer::default();
// 配置 Webview 控件
WebviewContainer::builder()
  .parent(&window) // 指定紧上一级控件
  .window(&window) // 指定主窗体。在本例中，【主窗体】即是【紧上一级控件】
  .flags(WebviewContainerFlags::VISIBLE) // 指定不显示控件边框
  .build(&mut webview_container)?;
// 构造 网格布局
let mut grid = GridLayout::default();
// 配置 网格布局
GridLayout::builder()
  .margin([0; 4]) // 白边
  .max_column(Some(1)) // 网络总列数
  .max_row(Some(1)) // 网络总行数
  .parent(&window)  // 指定给谁布局
  .child(0, 0, &webview_container) // 给布局加入子控件。在本例中，唯一的子控件就是 Webview
  .build(&mut grid)?;
// 构造【异步·执行器】与【异步·任务】
let mut executor = {
    let executor = LocalPool::new();
    let webview_ready_fut = webview_container.ready_fut()?;
    executor.spawner().spawn_local(async move {
        // 在这可以发起能够与 webview 初始化并行工作的异步任务。比如，
        // 1. 请求后端接口。
        // 2. 读取配置文件
        // 然后，再将这些 Future 实例与 webview 初始化 FusedFuture 实例 futures::join! 在一起。
        // ....
        // ....
        let (_, _, webview) = webview_ready_fut.await;
        // 执行直接依赖于 webview 实例的业务处理功能。
        // 比如，跳转至【欢迎页】
        webview.navigate(&cli_params.url)?;
        Ok::<_, Box<dyn Error>>(())
    }.map(|result| {
        if let Err(err) = result {
            eprintln!("[app_main]{err}");
        }
    }))?;
    executor
};
// 阻塞主线程，等待用户手动关闭主窗体
nwg::dispatch_thread_events_with_callback(move ||
    // 以 win32 UI 的事件循环为【反应器】，对接 futures crate 的【执行器】
    executor.run_until_stalled());
Ok(())
```

执行命令`cargo run --example nwg-remote-page`可直接运行该例程。

### 【派生宏】模式

将层叠嵌套的数据结构映射为分区划块的图形界面。这似乎能少写相当一部分重复代码。

```rust
// 以数据结构定义与映射图形界面布局。
#[derive(Default, NwgUi)]
pub struct DemoUi {
  // 主窗体
  #[nwg_control(size: (1024, 168), title: "内嵌 WebView 例程", flags: "MAIN_WINDOW|VISIBLE")]
  #[nwg_events(OnWindowClose: [nwg::stop_thread_dispatch()])]
  window: Window,
  // 布局对象
  #[nwg_layout(margin: [0; 4], parent: window, max_column: Some(1), max_row: Some(1), spacing: 0)]
  grid: GridLayout, // 网格布局主窗体
  // webview 控件
  #[nwg_control(flags: "VISIBLE", parent: window, window: &data.window)]
  #[nwg_layout_item(layout: grid, row: 0, col: 0)]
  webview_container: WebviewContainer, // 向网格布局填入唯一的子控件
}
impl DemoUi {
  // 构造【异步·执行器】与【异步·任务】
  fn executor(&self, url: &str) -> Result<LocalPool, Box<dyn Error>> {
    let executor = LocalPool::new();
    let webview_ready_fut = self.webview_container.ready_fut()?;
    executor.spawner().spawn_local(async move {
      // 在这可以发起能够与 webview 初始化并行工作的异步任务。比如，
      // 1. 请求后端接口。
      // 2. 读取配置文件
      // 然后，再将这些 Future 实例与 webview 初始化 FusedFuture 实例 futures::join! 在一起。
      // ....
      // ....
      let (_, _, webview) = webview_ready_fut.await;
      // 执行直接依赖于 webview 实例的业务处理功能。
      // 比如，跳转至【欢迎页】
      webview.navigate(url)?;
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
  let demo_ui_app = DemoUi::build_ui(Default::default())?;
  // 构造【异步·执行器】
  let mut executor = demo_ui_app.executor("https://www.minxing365.com")?;
  // 阻塞主线程，等待用户手动关闭主窗体
  nwg::dispatch_thread_events_with_callback(move ||
    // 以 win32 UI 的事件循环为【反应器】，对接 futures crate 的【执行器】
    executor.run_until_stalled());
  Ok(())
}
```

执行命令`cargo run --example nwd-remote-page`可直接运行该例程。

## `WebviewContainerBuilder`配置参数是`nwg::Frame`与`webview2::Environment(i.e.` [Microsoft.Web.WebView2.Core.CoreWebView2Environment](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environment?view=webview2-dotnet-1.0.2151.40)`)`的合集

> 后续出现的文字链都直接关联至`Microsoft MSDN`的`Win32`线上文档，因为
>
> * `Rust docs`实在太稀缺
> * `Rust Binding`对`Win32 COM ABI`几乎是`1:1`映射的，所以直接阅读`Microsoft MSDN`就足够理解接口功能了。另外，`Rust Binding`仍尚未全面覆盖每个`Win32 COM ABI`，所以别看到什么高级功能就兴奋得不要不要的，还得确认它是否已经被`webview2-sys crate`绑定？

### 占位原生布局流

依赖于来自`nwg::FrameBuilder`的配置参数[`flags`, `size`, `position`, `enabled`, 和`parent`](https://docs.rs/native-windows-gui/1.0.1/native_windows_gui/struct.Frame.html)。这些配置

* 参数名与底层`nwg::FrameBuilder`签名保持一致，而
* 参数值仅被透传给`nwg::FrameBuilder`实例

### `webview2::Environment(i.e.` [CoreWebView2Environment](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environment?view=webview2-dotnet-1.0.2151.40)`)`初始化

依赖于来自`webview2::EnvironmentBuilder`的配置参数 [browser_executable_folder, user_data_folder](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environment.createasync?view=webview2-dotnet-1.0.2151.40#parameters), [language](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environmentoptions.language?view=webview2-dotnet-1.0.2151.40#microsoft-web-webview2-core-corewebview2environmentoptions-language), [target_compatible_browser_version](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environmentoptions.targetcompatiblebrowserversion?view=webview2-dotnet-1.0.2151.40#microsoft-web-webview2-core-corewebview2environmentoptions-targetcompatiblebrowserversion),  [additional_browser_arguments](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environmentoptions.additionalbrowserarguments?view=webview2-dotnet-1.0.2151.40#microsoft-web-webview2-core-corewebview2environmentoptions-additionalbrowserarguments),
[allow_single_sign_on_using_osprimary_account](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environmentoptions.allowsinglesignonusingosprimaryaccount?view=webview2-dotnet-1.0.2151.40#microsoft-web-webview2-core-corewebview2environmentoptions-allowsinglesignonusingosprimaryaccount)。这些参数的含义与用法，请点开链接自己读吧。`Microsoft MSDN`文档写得极精细。

### `WebviewContainerBuilder`独有的参数

* `window: nwg::Window`
  * 【必填】图形应用程序的主窗体句柄。即便`WebviewContainer`控件的父控件就是应用程序的主窗体，该参数也得显式地传递 — 像例程里那样。
* `webview_env: webview2::Environment`
  * 【可选】在多`TAB`场景下，共享相同的`webview2::Environment`构造源

## [`Webview`操控接口](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp)

> 后续出现的文字链都直接关联至`Microsoft MSDN`的`Win32`线上文档，因为
>
> * `Rust docs`实在太稀缺
> * `Rust Binding`对`Win32 COM ABI`几乎是`1:1`映射的，所以直接阅读`Microsoft MSDN`就足够理解接口功能了。另外，`Rust Binding`仍尚未全面覆盖每个`Win32 COM ABI`，所以别看到什么高级功能就兴奋得不要不要的，还得确认它是否已经被`webview2-sys crate`绑定？

按照“（对外）面向原生图形界面上下文`hosting-related`”与“（对内）面向网页内容`web-specific`”的分类标准，`Webview API`被分别挂到

* [webview2::Controller](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2controller?view=webview2-dotnet-1.0.2151.40)
* [webview2::WebView](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2?view=webview2-dotnet-1.0.2151.40)

两个类实例上 — 对`Webview API`的分类也是从`Win32 COM`那一层就开始了，而不是我搞的。

在`webview2::Controller`上的`Webview API`包括：

* [关联图形应用程序的**主**窗体](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#parent-window)
* [析构整个`Webview`控件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#close-window)
* [读写`Webview`默认背景色](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#default-background-color)
* [读写`Webview`显示尺寸](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#sizing-positioning-and-visibility)
* [读写`Webview`显示位置](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#sizing-positioning-and-visibility)
* [否挂起或恢复`Webview`控件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#sizing-positioning-and-visibility)
  * 主窗口最小化时，推荐挂起`Webview`控件，以降低耗电
  * 主窗口恢复正常大小时，再恢复`Webview`控件。
* [缩放`Webview`原生控件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#rasterization-scale)。涵盖了：
  * 网页内容,
  * 弹出对话框
  * 上下文菜单
  * 滚动条
* [仅缩放`Webview`内的网页内容](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#zooming)
* [监听`Webview`的聚焦/失焦事件，以及焦点在不同**原生控件**之间转移的事件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#focus-and-tabbing)
* [监听来自键盘的`Ctrl / Alt +`任意键的组合键敲击事件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#keyboard-accelerators)
* [监听来自键盘的**不可打印**字符输入事件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#keyboard-accelerators)

在`webview2::WebView`上的`Webview API`包括：

* 原生<->`js`桥
  * [向网页上下文注入**原生**对象或**原生**函数](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#hostweb-object-sharing)
  * [向网页注入待执行的`js`代码](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#script-execution)
  * [与网页做跨源通信](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#web-messaging)
  * [监听浏览器内置对话框的弹出事件，以抑制或替换弹出对话框](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#script-dialogs)
  * [与网页程序共享内存](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#shared-buffer)
* 浏览器功能
  * [打印网页](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#printing)
  * [读写`Cookie`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#cookies)
  * [网页截屏与保存](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#image-capture)
  * [阻塞下载操作、定制下载保存目录和下载对话框`UI`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#downloads)
  * [抑制或定制`Permission`对话框外观](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#permissions)
  * [抑制或定制上下文菜单](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#context-menus)
  * [抑制、读写左下角状态栏内容，或监听状态栏内容变化](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#status-bar)
  * [读写`User Agent`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#user-agent)
  * [抑制表单自动填充](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#autofill)
  * [播放音频](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#audio)
  * [全屏模式](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#fullscreen)
  * [监听与拦截`js window.open()`调用](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#new-window)
  * [监听与拦截`js window.close()`调用](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#close-window)
  * [修改网页标题和监听网页标题内容变化](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#document-title)
  * [修改网页`Favicon`和监听网页`Favicon`变化](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#favicon)
* 进程管理
  * [读取`Webview TAB`的进程信息，甚至杀掉`TAB`进程](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#process-management)
* 网页内容
  * [加载、停止加载、重新加载网页内容](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#manage-content-loaded-into-webview2)
  * [网页路由](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#navigation-history)
  * [阻塞网页路由](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#block-unwanted-navigating)
  * [监听网页路由事件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#navigation-events)
  * [拦截与篡改网络请求](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#manage-network-requests-in-webview2)
  * [定制证书选择器对话框，甚至编程查询证书](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#client-certificates)
  * [信任服务端证书](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#server-certificates)
  * 从网页启动本地原生应用
    * [注册自定义的`scheme`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#custom-scheme-registration)
    * [监听自定义`scheme`被触发](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#launch-an-external-uri-scheme)
  * [监听`iframe`的创建与路由，甚至允许`iframe`跨域](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#iframes)
  * [定制`Basic Auth`登录表单对话框](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#authentication)
  * [切换`Profile`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#multiple-profiles)
  * [调试接口](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#performance-and-debugging)
  * [启用节能模式](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#memory-usage-target)
  * [开启`devtools`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#chrome-devtools-protocol-cdp)

其它`Webview API`包括：

* [抑制触屏设备上的左滑倒退，右滑前进，下拉刷新的手势识别](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#swipe-gesture-navigation)
  * 这个功能默认就已经是被关闭了，除非给`Webview`的额外启动参数`AdditionalBrowserArguments`添加`--pull-to-refresh`
* [抑制`PDF`视图的工具条](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#pdf-toolbar)
* [变换`Webview`主题色](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#theming)
* [获取当前网页是如何路由打开的](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=dotnetcsharp#navigation-kind)

这个汇总列表直接参考自`Microsoft MSDN`文档。其中有些`Win32 COM ABI`接口还没有被`webview2-sys crate`封装，所以需要亲自动手编写`FFI`代码才能正常调用许多高级功能接口。
