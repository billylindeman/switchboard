# switchboard
Switchboard is a scalable webrtc server built in rust!

The overall architecture is as follows:

- JSONRPC/Websocket signaling protocol
- webrtc.rs based routing engine
- pluggable coordination for multi-node operation
- Deep integration with gstreamer for multimedia processing (planned)

## Roadmap

This project is currently a work in progress, and very early stages. 

### Phase 0 (Current)
Single node operation that can host multiple call sessions & routing between peers

### Phase 1
Multi-node clustered operation with single node routing

This will likely start with redis/etcd coordination (comparable to how [ion-cluster](https://github.com/cryptagon/ion-cluster) works).  Cluster will support many call servers but any single session will only span a single node.

### Phase 2
Multi-node sessions that can span multiple call servers & support different distribution topoligies (ie fan out trees for broadcast).

This will involve replacing MediaTrackRouter/MediaTrackSubscriber's current tokio implementation.  I'd like this interface to be pluggable with different backends supported (UDP Unicast Relay, UDP Multicast groups, etc).
