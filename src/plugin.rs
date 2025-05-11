use crate::stingray_sdk::{GetApiFunction, LoggingApi, LuaApi, LuaType, lua_State};
use crate::{MODULE_NAME, PLUGIN, PLUGIN_NAME};
use futures::{FutureExt, select};
use matchbox_socket::{PeerId, PeerState, WebRtcSocket};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use uuid::Uuid;

const LUA_REGISTRYINDEX: i32 = -10000;

pub(crate) struct Plugin {
    pub log: Arc<LoggingApi>,
    pub lua: LuaApi,
    pub tokio_runtime: tokio::runtime::Runtime,
    pub sockets: Arc<Mutex<HashMap<String, WebRtcSocket>>>,
    pub on_peer_connected_callbacks: Arc<Mutex<HashMap<String, i32>>>,
    pub on_message_callbacks: Arc<Mutex<HashMap<String, i32>>>,
    pub on_peer_disconnected_callbacks: Arc<Mutex<HashMap<String, i32>>>,
    pub send_queue: Arc<Mutex<HashMap<String, Vec<(String, String)>>>>,
    pub disconnect_queue: Arc<Mutex<Vec<String>>>,
}

extern "C" fn connect(l: *mut lua_State) -> i32 {
    // Safety: Plugin must have been initialized for this to be registered as module
    // function.
    let plugin = unsafe { PLUGIN.get().unwrap_unchecked() };

    let arg_1_type = plugin
        .lua
        .lua_typename(l, 1)
        .unwrap_or("unknown".to_string());
    let arg_2_type = plugin
        .lua
        .lua_typename(l, 2)
        .unwrap_or("unknown".to_string());
    let arg_3_type = plugin
        .lua
        .lua_typename(l, 3)
        .unwrap_or("unknown".to_string());
    let arg_4_type = plugin
        .lua
        .lua_typename(l, 4)
        .unwrap_or("unknown".to_string());

    if plugin.lua.lua_type(l, 2) != LuaType::Function {
        plugin.log.error(
            PLUGIN_NAME,
            format!("connect: second argument is not a function ({arg_2_type}), should be a on_peer_connected callback"),
        );
        plugin.lua.pushboolean(l, false); // error
        return 1;
    }
    if plugin.lua.lua_type(l, 3) != LuaType::Function {
        plugin.log.error(
            PLUGIN_NAME,
            format!("connect: third argument is not a function ({arg_3_type}), should be a on_message callback"),
        );
        plugin.lua.pushboolean(l, false); // error
        return 1;
    }
    if plugin.lua.lua_type(l, 4) != LuaType::Function {
        plugin.log.error(
            PLUGIN_NAME,
            format!("connect: fourth argument is not a function ({arg_4_type}), should be a on_peer_disconnected callback"),
        );
        plugin.lua.pushboolean(l, false); // error
        return 1;
    }

    if let Some(channel_c_str) = plugin.lua.tolstring(l, 1) {
        let channel = channel_c_str.to_string_lossy().to_string();

        plugin.lua.pushvalue(l, 2);
        let on_peer_connected_callback = plugin.lua.lib_ref(l, LUA_REGISTRYINDEX);
        {
            let mut callbacks = plugin.on_peer_connected_callbacks.blocking_lock();
            callbacks.insert(channel.clone(), on_peer_connected_callback);
        }

        plugin.lua.pushvalue(l, 3);
        let on_message_callback = plugin.lua.lib_ref(l, LUA_REGISTRYINDEX);
        {
            let mut callbacks = plugin.on_message_callbacks.blocking_lock();
            callbacks.insert(channel.clone(), on_message_callback);
        }

        plugin.lua.pushvalue(l, 4);
        let on_peer_disconnected_callback = plugin.lua.lib_ref(l, LUA_REGISTRYINDEX);
        {
            let mut callbacks = plugin.on_peer_disconnected_callbacks.blocking_lock();
            callbacks.insert(channel.clone(), on_peer_disconnected_callback);
        }

        let url = format!("wss://rtc-darkti-de.onrender.com/{}", channel);
        plugin.log.info(PLUGIN_NAME, format!("Connecting to {url}"));

        plugin.tokio_runtime.spawn(async move {
            let result = std::panic::AssertUnwindSafe(async move {
                let (socket, loop_fut) =
                    WebRtcSocket::new_unreliable(&url);

                {
                    let mut sockets = plugin.sockets.lock().await;
                    sockets.insert(channel, socket);
                }

                let loop_fut = loop_fut.fuse();
                futures::pin_mut!(loop_fut);

                let timeout = time::sleep(Duration::from_millis(100));
                tokio::pin!(timeout);


                loop {
                    select! {
                        // Restart this loop every 100ms
                        _ = (&mut timeout).fuse() => {
                            timeout.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(100));
                        }

                        // Or break if the message loop ends (disconnected, closed, etc.)
                        _ = &mut loop_fut => {
                            plugin.log.info(PLUGIN_NAME, "Connection closed");
                            break;
                        }
                    }
                }
            })
            .catch_unwind()
            .await;

            if let Err(panic) = result {
                plugin.log.info(PLUGIN_NAME, format!("Background task panicked: {:?}", panic));
            }
        });

        0
    } else {
        plugin.log.error(
            PLUGIN_NAME,
            format!("connect: first argument is not a string ({arg_1_type})"),
        );
        plugin.lua.pushboolean(l, false); // error
        return 1;
    }
}

extern "C" fn send(l: *mut lua_State) -> i32 {
    // Safety: Plugin must have been initialized for this to be registered as module
    // function.
    let plugin = unsafe { PLUGIN.get().unwrap_unchecked() };

    if let Some(channel) = plugin.lua.tolstring(l, 1) {
        if let Some(recipient) = plugin.lua.tolstring(l, 2) {
            if let Some(message) = plugin.lua.tolstring(l, 3) {
                let channel = channel.to_string_lossy().to_string();
                let raw_recipient = recipient.to_string_lossy().to_string();
                let message = message.to_string_lossy().to_string();

                let recipient = if raw_recipient == "all" {
                    "all".to_string()
                } else {
                    match Uuid::parse_str(&raw_recipient) {
                        Ok(uuid) => uuid.to_string(),
                        Err(_) => {
                            plugin.log.error(
                                PLUGIN_NAME,
                                format!(
                                    "send: recipient {raw_recipient} is not \"all\" or a valid Uuid"
                                ),
                            );
                            plugin.lua.pushboolean(l, false); // error
                            return 1;
                        }
                    }
                };

                let mut send_queue = plugin.send_queue.blocking_lock();
                send_queue
                    .entry(channel)
                    .or_insert_with(Vec::new)
                    .push((recipient, message));

                plugin.lua.pushboolean(l, true);
                return 1;
            } else {
                plugin.log.error(
                    PLUGIN_NAME,
                    format!("send: third argument should be the message (string)"),
                );
                plugin.lua.pushboolean(l, false); // error
                return 1;
            }
        } else {
            plugin.log.error(
                PLUGIN_NAME,
                format!("send: second argument should be the recipient (string)"),
            );
            plugin.lua.pushboolean(l, false); // error
            return 1;
        }
    } else if plugin.lua.lua_type(l, 1) == LuaType::Nil {
        plugin
            .log
            .error(PLUGIN_NAME, format!("send: first argument is nil"));
        plugin.lua.pushboolean(l, false); // error
        return 1;
    } else {
        plugin.log.error(
            PLUGIN_NAME,
            format!("send: first argument should be the channel name (string)"),
        );
        plugin.lua.pushboolean(l, false); // error
        return 1;
    }
}

extern "C" fn disconnect(l: *mut lua_State) -> i32 {
    // Safety: Plugin must have been initialized for this to be registered as module
    // function.
    let plugin = unsafe { PLUGIN.get().unwrap_unchecked() };

    if let Some(channel) = plugin.lua.tolstring(l, 1) {
        let channel = channel.to_string_lossy().to_string();
        plugin.disconnect_queue.blocking_lock().push(channel);
        plugin.lua.pushboolean(l, true);
        return 1;
    } else {
        plugin.log.error(
            PLUGIN_NAME,
            format!("disconnect: first argument should be the channel name (string)"),
        );
        plugin.lua.pushboolean(l, false); // error
        return 1;
    }
}

impl Plugin {
    pub fn new(get_engine_api: GetApiFunction) -> Self {
        let log = Arc::new(LoggingApi::get(get_engine_api));
        let lua = LuaApi::get(get_engine_api);
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();

        Self {
            log,
            lua,
            tokio_runtime,
            sockets: Arc::new(Mutex::new(HashMap::new())),
            on_peer_connected_callbacks: Arc::new(Mutex::new(HashMap::new())),
            on_message_callbacks: Arc::new(Mutex::new(HashMap::new())),
            on_peer_disconnected_callbacks: Arc::new(Mutex::new(HashMap::new())),
            send_queue: Arc::new(Mutex::new(HashMap::new())),
            disconnect_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn setup_game(&self) {
        self.log.info(PLUGIN_NAME, "Initializing");
        self.lua
            .add_module_function(MODULE_NAME, "connect", connect);
        self.lua.add_module_function(MODULE_NAME, "send", send);
        self.lua
            .add_module_function(MODULE_NAME, "disconnect", disconnect);
    }

    pub fn shutdown_game(&self) {
        for (channel, socket) in self.sockets.blocking_lock().iter_mut() {
            self.log
                .info(PLUGIN_NAME, format!("Closing connection to: {channel}"));
            socket.close();
        }
        self.log.info(PLUGIN_NAME, "Shutting down");
    }

    pub fn update_game(&self, _dt: f32) {
        for channel in self.disconnect_queue.blocking_lock().drain(..) {
            // Close the socket if it exists
            if let Some(socket) = self.sockets.blocking_lock().get_mut(&channel) {
                self.log
                    .info(PLUGIN_NAME, format!("Disconnecting from {channel}"));
                socket.close();
            }

            self.sockets.blocking_lock().remove(&channel);

            // Clear the message queue if it exists
            self.send_queue.blocking_lock().remove(&channel);

            // Remove the callbacks
            // TODO: MEMORY LEAK: The callbacks are not removed from the Lua registry
            self.on_peer_connected_callbacks
                .blocking_lock()
                .remove(&channel);
            self.on_message_callbacks.blocking_lock().remove(&channel);
            self.on_peer_disconnected_callbacks
                .blocking_lock()
                .remove(&channel);
        }

        let callbacks = self.on_message_callbacks.blocking_lock();
        for (channel, socket) in self.sockets.blocking_lock().iter_mut() {
            // Handle any new peers
            for (peer, state) in socket.update_peers() {
                match state {
                    PeerState::Connected => {
                        self.log.info(
                            PLUGIN_NAME,
                            format!("[Channel: {channel}] Peer joined: {peer}"),
                        );
                        let callbacks = self.on_peer_connected_callbacks.blocking_lock();
                        if let Some(callback) = callbacks.get(channel) {
                            let l = self.lua.get_script_environment_state();
                            self.lua.rawgeti(l, LUA_REGISTRYINDEX, *callback);
                            self.lua.pushstring(l, peer.to_string());
                            self.lua.call(l, 1, 0);
                        }
                    }
                    PeerState::Disconnected => {
                        self.log.info(
                            PLUGIN_NAME,
                            format!("[Channel: {channel}] Peer left: {peer}"),
                        );
                        let callbacks = self.on_peer_disconnected_callbacks.blocking_lock();
                        if let Some(callback) = callbacks.get(channel) {
                            let l = self.lua.get_script_environment_state();
                            self.lua.rawgeti(l, LUA_REGISTRYINDEX, *callback);
                            self.lua.pushstring(l, peer.to_string());
                            self.lua.call(l, 1, 0);
                        }
                    }
                }
            }

            // Accept any messages incoming
            if let Some(callback) = callbacks.get(channel) {
                for (peer, packet) in socket.channel_mut(0).receive() {
                    let message = String::from_utf8_lossy(&packet);
                    self.log.info(
                        PLUGIN_NAME,
                        format!("[Channel: {channel}] Message from {peer}: {message:?}"),
                    );
                    let l = self.lua.get_script_environment_state();
                    self.lua.rawgeti(l, LUA_REGISTRYINDEX, *callback);
                    self.lua.pushstring(l, message.to_string());
                    self.lua.pushstring(l, peer.to_string());
                    self.lua.call(l, 2, 0);
                }
            }

            // Send any queued outgoing messages
            if let Some(send_queue) = self.send_queue.blocking_lock().get_mut(channel) {
                // Use drain(..) to consume and remove all items as you iterate
                for (recipient, message) in send_queue.drain(..) {
                    self.log.info(
                        PLUGIN_NAME,
                        format!("[Channel {channel}]: Message to {recipient}: {message}"),
                    );

                    let packet = message.as_bytes().to_vec().into_boxed_slice();
                    if recipient == "all" {
                        for peer in socket.connected_peers().collect::<Vec<PeerId>>() {
                            socket.channel_mut(0).send(packet.clone(), peer);
                        }
                    } else {
                        if let Ok(uuid) = Uuid::parse_str(&recipient) {
                            let peer_id = PeerId::from(uuid);
                            socket.channel_mut(0).send(packet, peer_id);
                        } else {
                            self.log.error(
                                PLUGIN_NAME,
                                format!("send: recipient {recipient} is not a valid Uuid"),
                            );
                        }
                    }
                }
            }
        }
    }
}

impl std::fmt::Debug for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PluginApi")
    }
}
