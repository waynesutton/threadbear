// Pictures use an attached sheet on macOS, so choosing one does not leave
// a separate picker window behind the app. Other platforms use GPUI's picker.
use gpui_kit::*;
use std::{future::Future, path::PathBuf, pin::Pin};

pub(crate) fn choose(
    window: &mut Window,
    prompt: &str,
    cx: &mut App,
) -> Pin<Box<dyn Future<Output = Option<PathBuf>>>> {
    #[cfg(target_os = "macos")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use std::ffi::{c_char, c_void, CStr, CString};
        type Sender = futures::channel::oneshot::Sender<Option<PathBuf>>;
        unsafe extern "C" {
            fn tb_picture_sheet(
                view: *mut c_void,
                prompt: *const c_char,
                context: *mut c_void,
                done: extern "C" fn(*mut c_void, *const c_char),
            );
        }
        extern "C" fn done(context: *mut c_void, path: *const c_char) {
            // The native completion runs once, including cancellation. Copy
            // its borrowed UTF-8 path before returning to the native block.
            let sender = unsafe { Box::from_raw(context.cast::<Sender>()) };
            let path = (!path.is_null()).then(|| {
                PathBuf::from(
                    unsafe { CStr::from_ptr(path) }
                        .to_string_lossy()
                        .into_owned(),
                )
            });
            let _ = sender.send(path);
        }
        let _ = cx;
        let view = HasWindowHandle::window_handle(window)
            .ok()
            .and_then(|h| match h.as_raw() {
                RawWindowHandle::AppKit(h) => Some(h.ns_view.as_ptr()),
                _ => None,
            });
        let (sender, receiver) = futures::channel::oneshot::channel::<Option<PathBuf>>();
        if let (Some(view), Ok(prompt)) = (view, CString::new(prompt)) {
            unsafe {
                tb_picture_sheet(
                    view,
                    prompt.as_ptr(),
                    Box::into_raw(Box::new(sender)).cast(),
                    done,
                );
            }
        }
        Box::pin(async move { receiver.await.ok().flatten() })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(prompt.to_owned().into()),
        });
        Box::pin(async move { paths.await.ok()?.ok()??.into_iter().next() })
    }
}
