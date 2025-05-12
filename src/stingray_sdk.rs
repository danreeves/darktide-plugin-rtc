#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::type_complexity)]
#![allow(unused)]

mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_char;
use std::os::raw::c_void;

pub use bindings::GetApiFunction;
pub use bindings::PluginApi;
pub use bindings::PluginApiID;
use bindings::lua_CFunction;
pub use bindings::lua_State;

impl std::default::Default for PluginApi {
    fn default() -> Self {
        Self {
            version: 69,
            flags: 3,
            setup_game: None,
            update_game: None,
            shutdown_game: None,
            get_name: None,
            loaded: None,
            start_reload: None,
            unloaded: None,
            finish_reload: None,
            setup_resources: None,
            shutdown_resources: None,
            unregister_world: None,
            register_world: None,
            get_hash: None,
            unkfunc13: None,
            unkfunc14: None,
            unkfunc15: None,
            debug_draw: None,
        }
    }
}

#[cfg(debug_assertions)]
fn get_engine_api(f: GetApiFunction, id: PluginApiID) -> *mut c_void {
    let f = f.expect("'GetApiFunction' is always passed by the engine");
    unsafe { f(id as u32) }
}

#[cfg(not(debug_assertions))]
fn get_engine_api(f: GetApiFunction, id: PluginApiID) -> *mut c_void {
    // `Option::unwrap` still generates several instructions in
    // optimized code.
    let f = unsafe { f.unwrap_unchecked() };

    unsafe { f(id as u32) }
}

pub struct LoggingApi {
    info: unsafe extern "C" fn(*const c_char, *const c_char),
    warning: unsafe extern "C" fn(*const c_char, *const c_char),
    error: unsafe extern "C" fn(*const c_char, *const c_char),
}

impl LoggingApi {
    pub fn get(f: GetApiFunction) -> Self {
        let api = unsafe {
            let api = get_engine_api(f, PluginApiID::LOGGING_API_ID);
            api as *mut bindings::LoggingApi
        };

        unsafe {
            Self {
                info: (*api).info.unwrap_unchecked(),
                warning: (*api).warning.unwrap_unchecked(),
                error: (*api).error.unwrap_unchecked(),
            }
        }
    }

    pub fn info(&self, system: impl Into<Vec<u8>>, message: impl Into<Vec<u8>>) {
        let system = CString::new(system).expect("Invalid CString");
        let message = CString::new(message).expect("Invalid CString");
        unsafe {
            (self.info)(system.as_ptr(), message.as_ptr());
        }
    }

    pub fn warning(&self, system: impl Into<Vec<u8>>, message: impl Into<Vec<u8>>) {
        let system = CString::new(system).expect("Invalid CString");
        let message = CString::new(message).expect("Invalid CString");
        unsafe {
            (self.warning)(system.as_ptr(), message.as_ptr());
        }
    }

    pub fn error(&self, system: impl Into<Vec<u8>>, message: impl Into<Vec<u8>>) {
        let system = CString::new(system).expect("Invalid CString");
        let message = CString::new(message).expect("Invalid CString");
        unsafe {
            (self.error)(system.as_ptr(), message.as_ptr());
        }
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LuaType {
    None = -1,
    Nil = 0,
    Boolean = 1,
    LightUserdata = 2,
    Number = 3,
    String = 4,
    Table = 5,
    Function = 6,
    Userdata = 7,
    Thread = 8,
    Unknown(i32),
}

impl From<i32> for LuaType {
    fn from(value: i32) -> Self {
        match value {
            -1 => Self::None,
            0 => Self::Nil,
            1 => Self::Boolean,
            2 => Self::LightUserdata,
            3 => Self::Number,
            4 => Self::String,
            5 => Self::Table,
            6 => Self::Function,
            7 => Self::Userdata,
            8 => Self::Thread,
            _ => Self::Unknown(value),
        }
    }
}

impl std::fmt::Display for LuaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Nil => write!(f, "Nil"),
            Self::Boolean => write!(f, "Boolean"),
            Self::LightUserdata => write!(f, "LightUserdata"),
            Self::Number => write!(f, "Number"),
            Self::String => write!(f, "String"),
            Self::Table => write!(f, "Table"),
            Self::Function => write!(f, "Function"),
            Self::Userdata => write!(f, "Userdata"),
            Self::Thread => write!(f, "Thread"),
            Self::Unknown(val) => write!(f, "Unkown({})", val),
        }
    }
}

pub struct LuaApi {
    add_module_function: unsafe extern "C" fn(*const c_char, *const c_char, lua_CFunction),
    set_module_number: unsafe extern "C" fn(*const c_char, *const c_char, f64),
    set_module_string: unsafe extern "C" fn(*const c_char, *const c_char, *const c_char),
    tolstring: unsafe extern "C" fn(*mut lua_State, i32, *mut usize) -> *const c_char,
    pushstring: unsafe extern "C" fn(*mut lua_State, *const c_char),
    pushboolean: unsafe extern "C" fn(*mut lua_State, i32),
    pushvalue: unsafe extern "C" fn(*mut lua_State, i32),
    lib_ref: unsafe extern "C" fn(*mut lua_State, i32) -> i32,
    rawgeti: unsafe extern "C" fn(*mut lua_State, i32, i32),
    call: unsafe extern "C" fn(*mut lua_State, i32, i32) -> (),
    getscriptenvironmentstate: unsafe extern "C" fn() -> *mut lua_State,
    lua_type: unsafe extern "C" fn(*mut lua_State, i32) -> i32,
    lua_typename: unsafe extern "C" fn(*mut lua_State, i32) -> *const c_char,
}

impl LuaApi {
    pub fn get(f: GetApiFunction) -> Self {
        let api = unsafe {
            let api = get_engine_api(f, PluginApiID::LUA_API_ID);
            api as *mut bindings::LuaApi
        };

        unsafe {
            Self {
                add_module_function: (*api).add_module_function.unwrap_unchecked(),
                set_module_number: (*api).set_module_number.unwrap_unchecked(),
                set_module_string: (*api).set_module_string.unwrap_unchecked(),
                tolstring: (*api).tolstring.unwrap_unchecked(),
                pushstring: (*api).pushstring.unwrap_unchecked(),
                pushboolean: (*api).pushboolean.unwrap_unchecked(),
                pushvalue: (*api).pushvalue.unwrap_unchecked(),
                lib_ref: (*api).lib_ref.unwrap_unchecked(),
                rawgeti: (*api).rawgeti.unwrap_unchecked(),
                call: (*api).call.unwrap_unchecked(),
                getscriptenvironmentstate: (*api).getscriptenvironmentstate.unwrap_unchecked(),
                lua_type: (*api).type_.unwrap_unchecked(),
                lua_typename: (*api).lua_typename.unwrap_unchecked(),
            }
        }
    }

    pub fn add_module_function(
        &self,
        module: impl Into<Vec<u8>>,
        name: impl Into<Vec<u8>>,
        cb: extern "C" fn(*mut lua_State) -> i32,
    ) {
        let module = CString::new(module).expect("Invalid CString");
        let name = CString::new(name).expect("Invalid CString");

        unsafe { (self.add_module_function)(module.as_ptr(), name.as_ptr(), Some(cb)) }
    }

    pub fn set_module_number(
        &self,
        module: impl Into<Vec<u8>>,
        name: impl Into<Vec<u8>>,
        value: f64,
    ) {
        let module = CString::new(module).expect("Invalid CString");
        let name = CString::new(name).expect("Invalid CString");

        unsafe { (self.set_module_number)(module.as_ptr(), name.as_ptr(), value) }
    }

    pub fn set_module_string(
        &self,
        module: impl Into<Vec<u8>>,
        name: impl Into<Vec<u8>>,
        value: impl Into<Vec<u8>>,
    ) {
        let module = CString::new(module).expect("Invalid CString");
        let name = CString::new(name).expect("Invalid CString");
        let value = CString::new(value).expect("Invalid CString");

        unsafe { (self.set_module_string)(module.as_ptr(), name.as_ptr(), value.as_ptr()) }
    }

    pub fn tolstring(&self, L: *mut lua_State, idx: i32) -> Option<&CStr> {
        let mut len: usize = 0;

        let c = unsafe { (self.tolstring)(L, idx, &mut len as *mut _) };

        if len == 0 {
            None
        } else {
            // Safety: As long as `len > 0`, Lua guarantees the constraints that `CStr::from_ptr`
            // requires.
            Some(unsafe { CStr::from_ptr(c) })
        }
    }

    pub fn pushstring(&self, L: *mut lua_State, s: impl Into<Vec<u8>>) {
        let s = CString::new(s).expect("Invalid CString");
        unsafe { (self.pushstring)(L, s.as_ptr()) }
    }

    pub fn pushboolean(&self, L: *mut lua_State, b: bool) {
        unsafe { (self.pushboolean)(L, b as i32) }
    }

    pub fn pushvalue(&self, L: *mut lua_State, idx: i32) {
        unsafe { (self.pushvalue)(L, idx) }
    }

    pub fn lib_ref(&self, L: *mut lua_State, idx: i32) -> i32 {
        unsafe { (self.lib_ref)(L, idx) }
    }

    pub fn rawgeti(&self, L: *mut lua_State, idx: i32, n: i32) {
        unsafe { (self.rawgeti)(L, idx, n) }
    }

    pub fn call(&self, L: *mut lua_State, n_args: i32, n_results: i32) {
        unsafe { (self.call)(L, n_args, n_results) }
    }

    pub fn get_script_environment_state(&self) -> *mut lua_State {
        unsafe { (self.getscriptenvironmentstate)() }
    }

    pub fn lua_type(&self, L: *mut lua_State, idx: i32) -> LuaType {
        LuaType::from(unsafe { (self.lua_type)(L, idx) })
    }

    pub fn lua_typename(&self, L: *mut lua_State, idx: i32) -> Option<String> {
        let c = unsafe { (self.lua_typename)(L, idx) };

        if c.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(c) }.to_string_lossy().into_owned())
        }
    }
}
