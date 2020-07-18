# Switchboard (WIP, not ready!)
Switchboard is a rust GStreamer based webrtc SFU/MCU

The general architecture is to create a GStreamer Pipeline for each published stream that terminates video / audio into a tagged interpipesink.  When a user subscribes to a stream we will take that tag and pipe it into webrtcbin as a new track.  Eventually composite streams can be created in their own pipelines that are subscribed to individual feeds.

With this configuration switchboard can be both an SFU and MCU simultaneously, and clients can choose how they want to consume video/audio/etc.  Eventually more complex topologies can be created/controlled via API.



