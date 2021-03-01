use std::convert::TryInto;

use x11rb::connection::Connection;
use x11rb::protocol::xproto;
use x11rb::protocol::xproto::ConnectionExt;

use crate::util::Dimension;

type Conn = impl x11rb::connection::Connection + Send + Sync;

// TODO: remove pub
pub struct Atoms {
    window_type: xproto::Atom,
    window_type_dock: xproto::Atom,
    desktop: xproto::Atom,
    strut_partial: xproto::Atom,
    strut: xproto::Atom,
    state: xproto::Atom,
    state_sticky: xproto::Atom,
    state_above: xproto::Atom,
}

impl Atoms {
    pub fn new(get_atom: &dyn Fn(&'static str) -> xproto::Atom) -> Self {
        Self {
            window_type: get_atom("_NET_WM_WINDOW_TYPE"),
            window_type_dock: get_atom("_NET_WM_WINDOW_TYPE_DOCK"),
            desktop: get_atom("_NET_WM_DESKTOP"),
            strut_partial: get_atom("_NET_WM_STRUT_PARTIAL"),
            strut: get_atom("_NET_WM_STRUT"),
            state: get_atom("_NET_WM_STATE"),
            state_sticky: get_atom("_NET_WM_STATE_STICKY"),
            state_above: get_atom("_NET_WM_STATE_ABOVE"),
        }
    }
}

// TODO: remove pub and have a general window abstraction to work with any display manager
pub struct XWindow<'a> {
    x11: &'a X11,
    id: u32,
    dim: Dimension,
}

impl<'a> XWindow<'a> {
    // TODO: abstract height, width, x and y into container struct
    pub fn new(x11: &'a X11, dim: Dimension) -> Self {
        let id = x11.conn.generate_id().unwrap();
        xproto::create_window(
            &x11.conn,
            x11rb::COPY_DEPTH_FROM_PARENT,
            id,
            x11.screen.root,
            dim.x,
            dim.y,
            dim.width,
            dim.height,
            0,
            xproto::WindowClass::INPUT_OUTPUT,
            0,
            &xproto::CreateWindowAux::new()
                .background_pixel(x11.screen.black_pixel)
                .border_pixel(x11.screen.black_pixel)
                .event_mask(xproto::EventMask::EXPOSURE
                            | xproto::EventMask::BUTTON_PRESS
                            | xproto::EventMask::FOCUS_CHANGE)
                // colormap
        ).unwrap();

        Self { x11, id, dim }
    }

    pub fn make_dock(self) -> Self {
        xproto::change_property(
            &self.x11.conn,
            xproto::PropMode::REPLACE,
            self.id,
            self.x11.atoms.window_type,
            xproto::AtomEnum::ATOM,
            32,
            1,
            unsafe { &std::mem::transmute::<xproto::Atom, [u8; 4]>(self.x11.atoms.window_type_dock) }
        ).unwrap();

        self
    }

    pub fn make_sticky(self) -> Self {
        xproto::change_property(
            &self.x11.conn,
            xproto::PropMode::APPEND,
            self.id,
            self.x11.atoms.state,
            xproto::AtomEnum::ATOM,
            32,
            1,
            unsafe { &std::mem::transmute::<xproto::Atom, [u8; 4]>(self.x11.atoms.state_sticky) }
        ).unwrap();

        self
    }

    pub fn make_above(self) -> Self {
        xproto::change_property(
            &self.x11.conn,
            xproto::PropMode::APPEND,
            self.id,
            self.x11.atoms.state,
            xproto::AtomEnum::ATOM,
            32,
            1,
            unsafe { &std::mem::transmute::<xproto::Atom, [u8; 4]>(self.x11.atoms.state_above) }
        ).unwrap();

        self
    }

    pub fn appear_on_all_desktops(self) -> Self {
        xproto::change_property(
            &self.x11.conn,
            xproto::PropMode::REPLACE,
            self.id,
            self.x11.atoms.desktop,
            xproto::AtomEnum::CARDINAL,
            32,
            1,
            unsafe { &std::mem::transmute::<u32, [u8; 4]>(u32::MAX) }
        ).unwrap();

        self
    }

    pub fn reserve_space(self) -> Self {
        let width_i32: i32 = self.dim.width.into();
        let width_x: i32 = width_i32 + Into::<i32>::into(self.dim.x) - 1;
        let strut: [i32; 12] = if self.dim.y == 0 {
            [0, 0, self.dim.height.into(), 0, 0, 0, 0, 0, self.dim.x.into(), width_x, 0, 0]
        } else {
            [0, 0, 0, self.dim.height.into(), 0, 0, 0, 0, 0, 0, self.dim.x.into(), width_x]
        };

        // reserve space
        xproto::change_property(
            &self.x11.conn,
            xproto::PropMode::REPLACE,
            self.id,
            self.x11.atoms.strut_partial,
            xproto::AtomEnum::CARDINAL,
            32,
            12,
            unsafe { std::mem::transmute::<&[i32; 12], &[u8; 48]>(&strut) }
        ).unwrap();

        // reserve space (backwards compatibility)
        // TODO: inline whatever is going on with ayy
        let ayy: [i32; 4] = [0, 0, self.dim.height as i32, 0];
        xproto::change_property(
            &self.x11.conn,
            xproto::PropMode::REPLACE,
            self.id,
            self.x11.atoms.strut,
            xproto::AtomEnum::CARDINAL,
            32,
            4,
            unsafe { std::mem::transmute::<&[i32; 4], &[u8; 16]>(&ayy) }
        ).unwrap();

        self
    }

    pub fn set_window_name(self, name: &'static str) -> Self {
        xproto::change_property(
            &self.x11.conn,
            xproto::PropMode::REPLACE,
            self.id,
            xproto::AtomEnum::WM_NAME,
            xproto::AtomEnum::STRING,
            8,
            name.len().try_into().unwrap(),
            name.as_bytes()
        ).unwrap();
        
        self
    }

    pub fn set_window_class(self, class: &'static str) -> Self {
        xproto::change_property(
            &self.x11.conn,
            xproto::PropMode::REPLACE,
            self.id,
            xproto::AtomEnum::WM_CLASS,
            xproto::AtomEnum::STRING,
            8,
            class.len().try_into().unwrap(),
            class.as_bytes()
        ).unwrap();
        
        self
    }

    pub fn force_position(self) -> Self {
        xproto::configure_window(
            &self.x11.conn,
            self.id,
            &xproto::ConfigureWindowAux::new()
                .x(self.dim.x as i32)
                .y(self.dim.y as i32)
        ).unwrap();

        self
    }

    pub fn make_visible(self) -> Self {
        self.x11.conn.map_window(self.id).unwrap();
        self
    }
}

pub struct X11 {
    pub(super) conn: Conn,
    pub(super) screen_num: usize,
    pub(super) screen: xproto::Screen,
    pub(super) atoms: Atoms,
}

impl X11 {
    pub fn new() -> Self {
        let wrapper = || -> (Conn, usize) { x11rb::connect(None).unwrap() };
        let (conn, screen_num): (Conn, usize) = wrapper();

        // TODO: move this logic into Atoms::new()
        let get_atom = |name: &'static str| xproto::intern_atom(&conn, false, name.as_bytes())
            .unwrap()  // TODO: clean
            .reply()
            .unwrap()  // TODO: clean
            .atom;

        let atoms = Atoms::new(&get_atom);
        let screen = conn.setup().roots[screen_num].clone();
        Self {
            conn,
            screen_num,
            screen,
            atoms,
        }
    }

    pub fn create_window(&self, dim: Dimension) -> XWindow {
        let win = XWindow::new(&self, dim)
            .make_dock()
            .make_sticky()
            .make_above()
            .appear_on_all_desktops()
            .reserve_space()
            .set_window_name("bar")
            .set_window_class("onyxbar")
            .make_visible()
            .force_position();

        self.conn.flush().unwrap();

        win
    }

    pub fn loop_events(&self) {
        loop {
            println!("Event: {:?}", self.conn.wait_for_event().unwrap());
        }
    }
}
