# About


Switchboard is a clustered, scalable webrtc server built in rust!


The overall architecture is as follows:

- JSONRPC/Websocket signaling protocol
- webrtc.rs based routing engine
- pluggable consensus / coordination for multi-node operation
- deep integration with gstreamer for multimedia processing



