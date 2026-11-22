// Copyright 2022 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

use accesskit::{
    Action, ActionHandler, ActionRequest, ActivationHandler, Node, NodeId, Role, TreeId, TreeInfo,
    TreeUpdate,
};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{LazyLock, mpsc},
};
use windows as Windows;
use windows::{
    Win32::{
        Foundation::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Accessibility::*, Input::KeyboardAndMouse::SetFocus, WindowsAndMessaging::*},
    },
    core::*,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    platform::windows::EventLoopBuilderExtWindows,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowId},
};

use super::{DEFAULT_TIMEOUT, MUTEX, scope_with_window};
use crate::{
    SubclassingAdapter,
    adapter::menu_tests::{MENU_CLOSES, popup},
    window_handle::WindowHandle,
};

const WINDOW_TITLE: &str = "Simple test";

const WINDOW_ID: NodeId = NodeId(0);
const BUTTON_1_ID: NodeId = NodeId(1);
const BUTTON_2_ID: NodeId = NodeId(2);

fn make_button(label: &str) -> Node {
    let mut node = Node::new(Role::Button);
    node.set_label(label);
    node.add_action(Action::Focus);
    node
}

fn get_initial_state() -> TreeUpdate {
    let mut root = Node::new(Role::Window);
    root.set_children(vec![BUTTON_1_ID, BUTTON_2_ID]);
    let button_1 = make_button("Button 1");
    let button_2 = make_button("Button 2");
    TreeUpdate {
        nodes: vec![
            (WINDOW_ID, root),
            (BUTTON_1_ID, button_1),
            (BUTTON_2_ID, button_2),
        ],
        tree: Some(TreeInfo::new(WINDOW_ID)),
        tree_id: TreeId::ROOT,
        focus: BUTTON_1_ID,
    }
}

pub struct NullActionHandler;

impl ActionHandler for NullActionHandler {
    fn do_action(&mut self, _request: ActionRequest) {}
}

struct SimpleActivationHandler;

impl ActivationHandler for SimpleActivationHandler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(get_initial_state())
    }
}

// This module uses winit for the purpose of testing with a real third-party
// window implementation that we don't control. However, only one test
// can use winit, because winit only allows an event loop to be created
// once per process. So we end up creating our own window anyway for the
// double-instantiation test.
//
// Also, while these tests don't use the main test harness or show the window,
// they still need to run with the main harness's mutex, to avoid disturbing
// other tests, particularly the focus test.

struct TestApplication;

impl ApplicationHandler<()> for TestApplication {
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_visible(false);

        let window = event_loop.create_window(window_attributes).unwrap();
        let hwnd = match window.window_handle().unwrap().as_raw() {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut core::ffi::c_void),
            RawWindowHandle::WinRt(_) => unimplemented!(),
            _ => unreachable!(),
        };
        let adapter =
            SubclassingAdapter::new(hwnd, SimpleActivationHandler {}, NullActionHandler {});
        assert!(unsafe { UiaHasServerSideProvider(hwnd) }.as_bool());
        drop(window);
        drop(adapter);
        event_loop.exit();
    }
}

#[test]
fn has_native_uia() {
    let _lock_guard = MUTEX.lock();
    let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
    let mut state = TestApplication {};
    event_loop.run_app(&mut state).unwrap();
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message == WM_DESTROY {
        unsafe { PostQuitMessage(0) };
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

static WINDOW_CLASS_ATOM: LazyLock<u16> = LazyLock::new(|| {
    let class_name = w!("AccessKitSubclassTest");

    let wc = WNDCLASSW {
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap(),
        hInstance: unsafe { GetModuleHandleW(None) }.unwrap().into(),
        lpszClassName: class_name,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        ..Default::default()
    };

    let atom = unsafe { RegisterClassW(&wc) };
    if atom == 0 {
        panic!("{}", Error::from_thread());
    }
    atom
});

fn create_window(title: &str, style: WINDOW_STYLE, extended_style: WINDOW_EX_STYLE) -> HWND {
    let module = HINSTANCE::from(unsafe { GetModuleHandleW(None).unwrap() });

    let window = unsafe {
        CreateWindowExW(
            extended_style,
            PCWSTR(*WINDOW_CLASS_ATOM as usize as _),
            &HSTRING::from(title),
            style,
            0,
            0,
            160,
            100,
            None,
            None,
            Some(module),
            None,
        )
    }
    .unwrap();
    if window.is_invalid() {
        panic!("{}", Error::from_thread());
    }

    window
}

#[test]
#[should_panic(expected = "already instantiated")]
fn double_instantiate() {
    let _lock_guard = MUTEX.lock();
    let window = create_window(WINDOW_TITLE, WS_OVERLAPPEDWINDOW, Default::default());
    let _adapter1 =
        SubclassingAdapter::new(window, SimpleActivationHandler {}, NullActionHandler {});
    let _adapter2 =
        SubclassingAdapter::new(window, SimpleActivationHandler {}, NullActionHandler {});
}

const OPEN_POPUP: u32 = WM_APP;
const FOCUS_EXIT: u32 = WM_APP + 1;
const INVOKE_EXIT: u32 = WM_APP + 2;

struct PopupActivation(Rc<Cell<bool>>);
impl ActivationHandler for PopupActivation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(popup(self.0.get(), NodeId(0)))
    }
}

struct InvokeExit(WindowHandle);
impl ActionHandler for InvokeExit {
    fn do_action(&mut self, request: ActionRequest) {
        if request.action == Action::Click && request.target_node == NodeId(2) {
            unsafe { PostMessageW(Some(self.0.0), INVOKE_EXIT, WPARAM(0), LPARAM(0)) }.unwrap();
        }
    }
}

#[implement(Windows::Win32::UI::Accessibility::IUIAutomationEventHandler)]
struct MenuClosed {
    window: WindowHandle,
    received: mpsc::Sender<bool>,
}

#[allow(non_snake_case)]
impl IUIAutomationEventHandler_Impl for MenuClosed_Impl {
    fn HandleAutomationEvent(
        &self,
        sender: Ref<IUIAutomationElement>,
        _: UIA_EVENT_ID,
    ) -> Result<()> {
        if let Some(sender) = sender.as_ref() {
            if unsafe { sender.CachedNativeWindowHandle() }.ok() == Some(self.window.0) {
                let _ = self
                    .received
                    .send(unsafe { IsWindowVisible(self.window.0) }.as_bool());
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
struct PopupFocus(UIA_CONTROLTYPE_ID, String);

#[implement(Windows::Win32::UI::Accessibility::IUIAutomationFocusChangedEventHandler)]
struct PopupFocusListener {
    window: WindowHandle,
    received: mpsc::Sender<Result<PopupFocus>>,
}

#[allow(non_snake_case)]
impl IUIAutomationFocusChangedEventHandler_Impl for PopupFocusListener_Impl {
    fn HandleFocusChangedEvent(&self, sender: Ref<IUIAutomationElement>) -> Result<()> {
        let Some(sender) = sender.as_ref() else {
            return Ok(());
        };
        let handle = unsafe { sender.CachedNativeWindowHandle() }?;
        if handle == self.window.0
            || (handle.0.is_null()
                && unsafe { sender.CachedProcessId() }? as u32 == std::process::id())
        {
            let focus = || unsafe {
                Ok(PopupFocus(
                    sender.CachedControlType()?,
                    sender.CachedName()?.to_string(),
                ))
            };
            let _ = self.received.send(focus());
        }
        Ok(())
    }
}

#[test]
fn popup_focus_and_exit_are_reported_through_window_destruction() -> Result<()> {
    let (finished, teardown) = mpsc::channel();
    let (closed, closes) = mpsc::channel();
    let (focused, focus) = mpsc::channel();
    let menu_focus = PopupFocus(UIA_MenuControlTypeId, "Ember".into());
    let item_focus = PopupFocus(UIA_MenuItemControlTypeId, "Exit".into());
    scope_with_window(
        move || {
            let window = create_window("", WS_POPUP, WS_EX_TOOLWINDOW | WS_EX_TOPMOST);
            let open = Rc::new(Cell::new(false));
            let mut adapter = Some(SubclassingAdapter::new(
                window,
                PopupActivation(open.clone()),
                InvokeExit(window.into()),
            ));
            Ok((window, move |message: &MSG| {
                if message.hwnd != window {
                    return false;
                }
                let Some(access) = adapter.as_mut() else {
                    return false;
                };
                match message.message {
                    OPEN_POPUP => {
                        access.activate_for_listeners();
                        open.set(true);
                        let events = access.update_if_active(|| popup(true, NodeId(0)));
                        unsafe {
                            let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
                            events.unwrap().raise();
                            let _ = SetForegroundWindow(window);
                            let _ = SetFocus(Some(window));
                        }
                    }
                    FOCUS_EXIT => access
                        .update_if_active(|| popup(true, NodeId(2)))
                        .unwrap()
                        .raise(),
                    INVOKE_EXIT => {
                        let _ = unsafe { ShowWindow(window, SW_HIDE) };
                        open.set(false);
                        access
                            .update_if_active(|| popup(false, NodeId(0)))
                            .unwrap()
                            .raise();
                        unsafe { DestroyWindow(window) }.unwrap();
                        drop(adapter.take());
                        finished.send(MENU_CLOSES.get()).unwrap();
                    }
                    _ => return false,
                }
                true
            }))
        },
        |s| unsafe {
            let handler: IUIAutomationEventHandler = MenuClosed {
                window: s.window,
                received: closed,
            }
            .into();
            let desktop = s.uia.GetRootElement()?;
            let cache = s.uia.CreateCacheRequest()?;
            cache.AddProperty(UIA_NativeWindowHandlePropertyId)?;
            cache.AddProperty(UIA_ControlTypePropertyId)?;
            cache.AddProperty(UIA_NamePropertyId)?;
            cache.AddProperty(UIA_ProcessIdPropertyId)?;
            s.uia.AddAutomationEventHandler(
                UIA_MenuClosedEventId,
                &desktop,
                TreeScope_Subtree,
                &cache,
                &handler,
            )?;
            let _listener = scopeguard::guard((), |_| {
                s.uia
                    .RemoveAutomationEventHandler(UIA_MenuClosedEventId, &desktop, &handler)
                    .unwrap();
            });

            let listener: IUIAutomationFocusChangedEventHandler = PopupFocusListener {
                window: s.window,
                received: focused,
            }
            .into();
            s.uia.AddFocusChangedEventHandler(&cache, &listener)?;
            let _focus_listener = scopeguard::guard((), |_| {
                s.uia.RemoveFocusChangedEventHandler(&listener).unwrap();
            });
            let post = |message| PostMessageW(Some(s.window.0), message, WPARAM(0), LPARAM(0));
            post(OPEN_POPUP)?;
            assert_eq!(
                focus
                    .recv_timeout(DEFAULT_TIMEOUT)
                    .expect("menu did not take focus")?,
                menu_focus
            );
            let menu = s.uia.ElementFromHandle(s.window.0)?;
            assert_eq!(menu.CurrentControlType()?, UIA_MenuControlTypeId);
            assert_eq!(menu.CurrentName()?.to_string(), "Ember");
            assert_eq!(menu.CurrentNativeWindowHandle()?, s.window.0);
            let exit = s.uia.ControlViewWalker()?.GetFirstChildElement(&menu)?;
            assert_eq!(exit.CurrentName()?.to_string(), "Exit");
            let invoke =
                exit.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)?;
            post(FOCUS_EXIT)?;
            loop {
                let source = focus
                    .recv_timeout(DEFAULT_TIMEOUT)
                    .expect("Exit did not take focus")?;
                if source == item_focus {
                    break;
                }
                assert_eq!(source, menu_focus);
            }
            assert!(IsWindowVisible(s.window.0).as_bool());
            invoke.Invoke()?;
            assert_eq!(
                teardown
                    .recv_timeout(DEFAULT_TIMEOUT)
                    .expect("popup was not destroyed"),
                1
            );
            assert!(
                !closes
                    .recv_timeout(DEFAULT_TIMEOUT)
                    .expect("MenuClosed was not delivered")
            );
            assert!(!IsWindow(Some(s.window.0)).as_bool());
            Ok(())
        },
    )?;
    // UIA can deliver callbacks after unsubscription. The handler owns the
    // channel's only sender until UIA releases its final COM reference.
    assert_eq!(
        closes.recv_timeout(DEFAULT_TIMEOUT),
        Err(mpsc::RecvTimeoutError::Disconnected),
        "expected exactly one MenuClosed callback through window destruction"
    );
    loop {
        match focus.recv_timeout(DEFAULT_TIMEOUT) {
            Ok(source) => {
                let source = source?;
                assert!(source == menu_focus || source == item_focus, "{source:?}");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => panic!("UIA retained the focus listener"),
        }
    }
    Ok(())
}
