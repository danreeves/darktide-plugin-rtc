# darktide-plugin-rtc

This plugin is a wrapper around the [matchbox](https://github.com/johanhelsing/matchbox) library, which is based on the WebRTC protocol. It enables udp-like, unordered, unreliable p2p connections. It uses a signaling server hosted at [rtc.darkti.de](https://github.com/danreeves/rtc.darkti.de) to establish the connections, which are then entirely peer-to-peer.

This plugin is exposed via a lua API:

## API

Note, a PeerId is a UUID string that uniquely identifies a peer in the network.

### RPC.connect(room: string, on_peer_connect: function, on_message: function, on_peer_disconnect: function)
This function connects to a room. The room name is a string, and the function takes three callbacks as arguments:
- `on_peer_connect`: This function is called when a peer connects to the room. It takes a single argument, which is the PeerId of the connected peer.
- `on_message`: This function is called when a message is received from a peer. It takes two arguments: the PeerId of the sender and the message itself.
- `on_peer_disconnect`: This function is called when a peer disconnects from the room. It takes a single argument, which is the PeerId of the disconnected peer.

### RPC.send(room: string, recipient: "all" | PeerId, message: string)
This function sends a message to a peer or all peers in the room. The `room` argument is a string, the `recipient` argument can be either "all" to send to all peers or a specific PeerId to send to a single peer, and the `message` argument is the message itself.

### RPC.disconnect(room: string)
This function disconnects from a room. The `room` argument is a string.

## Example Usage

```lua
function on_peer_connect(peer_id)
	mod:echo("Peer connected: " .. peer_id)
end

function on_message(message, peer_id)
	mod:echo("Message from " .. peer_id .. ": " .. message)
	if message == "ping" then
		RPC.send("my_room", peer_id, "pong")
	end
end

function on_peer_disconnect(peer_id)
	mod:echo("Peer disconnected: " .. peer_id)
end

-- Set up the connection
RPC.connect("my_room", on_peer_connect, on_message, on_peer_disconnect)

-- Send a message to all peers
RPC.send("my_room", "all", "Hello, everyone!")

-- Close the connection
RPC.disconnect("my_room")
```

## Installation
Once built, copy `darktide_plugin_rtc.dll` into `[Darktide install]/binaries/plugins/`.
