use std::ffi::CString;
use std::ptr;
use std::rc::Rc;

use hbb_common::libc;

use super::ffi::*;
use super::{Display, Rect, Server};

//TODO: Do I have to free the displays?

pub struct DisplayIter {
    outer: xcb_screen_iterator_t,
    inner: Option<(
        xcb_screen_t,
        xcb_randr_monitor_info_iterator_t,
        xcb_window_t,
    )>,
    server: Rc<Server>,
}

impl DisplayIter {
    pub unsafe fn new(server: Rc<Server>) -> DisplayIter {
        let mut outer = xcb_setup_roots_iterator(server.setup());
        let inner = Self::next_screen(&mut outer, &server);
        DisplayIter {
            outer,
            inner,
            server,
        }
    }

    fn next_screen(
        outer: &mut xcb_screen_iterator_t,
        server: &Server,
    ) -> Option<(
        xcb_screen_t,
        xcb_randr_monitor_info_iterator_t,
        xcb_window_t,
    )> {
        if outer.rem == 0 {
            return None;
        }

        unsafe {
            let root = (*outer.data).root;

            let cookie = xcb_randr_get_monitors_unchecked(
                server.raw(),
                root,
                1, //TODO: I don't know if this should be true or false.
            );

            let response = xcb_randr_get_monitors_reply(server.raw(), cookie, ptr::null_mut());

            let inner = xcb_randr_get_monitors_monitors_iterator(response);

            libc::free(response as *mut _);
            xcb_screen_next(outer);

            Some((*(outer.data).clone(), inner, root))
        }
    }
}

impl Iterator for DisplayIter {
    type Item = Display;

    fn next(&mut self) -> Option<Display> {
        loop {
            if let Some((ref screen, ref mut inner, root)) = self.inner {
                // If there is something in the current screen, return that.
                if inner.rem != 0 {
                    unsafe {
                        let data = &*inner.data;
                        let name = get_atom_name(self.server.raw(), data.name);

                        let geo_cookie = xcb_get_geometry_unchecked(self.server.raw(), root);
                        let geo =
                            xcb_get_geometry_reply(self.server.raw(), geo_cookie, ptr::null_mut());
                        println!(
                            "x: {}, y: {}, width: {}, height: {}",
                            (*geo).x,
                            (*geo).y,
                            (*geo).width,
                            (*geo).height,
                        );
                        let translate_cookie = xcb_translate_coordinates_unchecked(
                            self.server.raw(),
                            root,
                            root,
                            0,
                            0,
                        );
                        let translate = xcb_translate_coordinates_reply(
                            self.server.raw(),
                            translate_cookie,
                            ptr::null_mut(),
                        );
                        println!(
                            "translate x: {}, y: {}",
                            (*translate).dst_x,
                            (*translate).dst_y,
                        );
                        let display = Display::new(
                            self.server.clone(),
                            data.primary != 0,
                            Rect {
                                x: (*geo).x,
                                y: (*geo).y,
                                w: (*geo).width,
                                h: (*geo).height,
                            },
                            root,
                            name,
                        );
                        libc::free(geo as _);
                        libc::free(translate as _);

                        xcb_randr_monitor_info_next(inner);
                        return Some(display);
                    }
                }
            } else {
                // If there is no current screen, the screen iterator is empty.
                return None;
            }

            // The current screen was empty, so try the next screen.
            self.inner = Self::next_screen(&mut self.outer, &self.server);
        }
    }
}

fn get_atom_name(conn: *mut xcb_connection_t, atom: xcb_atom_t) -> String {
    let empty = "".to_owned();
    if atom == 0 {
        return empty;
    }
    unsafe {
        let mut e: *mut xcb_generic_error_t = std::ptr::null_mut();
        let reply = xcb_get_atom_name_reply(conn, xcb_get_atom_name(conn, atom), &mut e as _);
        if reply == std::ptr::null() {
            return empty;
        }
        let length = xcb_get_atom_name_name_length(reply);
        let name = xcb_get_atom_name_name(reply);
        let mut v = vec![0u8; length as _];
        std::ptr::copy_nonoverlapping(name as _, v.as_mut_ptr(), length as _);
        libc::free(reply as *mut _);
        if let Ok(s) = CString::new(v) {
            return s.to_string_lossy().to_string();
        }
        empty
    }
}
