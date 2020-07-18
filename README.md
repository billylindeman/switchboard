# Switchboard (WIP, not ready!)
Switchboard is a rust GStreamer based webrtc SFU/MCU

The general architecture is to create a GStreamer Pipeline for each published stream that terminates video / audio into a tagged interpipesink.  When a user subscribes to a stream we will take that tag and pipe it into webrtcbin as a new track.  Eventually composite streams can be created in their own pipelines that are subscribed to individual feeds.

With this configuration switchboard can be both an SFU and MCU simultaneously, and clients can choose how they want to consume video/audio/etc.  Eventually more complex topologies can be created/controlled via API.

Higher level controllers can be written that will consume streams and publish their own streams within a room (directly within gstreamer). A hypothetical use case could be a controller that consumes and decodes all video streams and composites them.  Or perhaps it runs a machine learning model against it (perhaps a nsfw detector) and publishes json results in realtime back to the websocket channels.  

Another idea is to consume all video streams, composite them into a single feed, and then relay that to an RTMP feed. By building on top of gstreamer, any gstreamer pipeline topology and any set of gstreamer plugins could be integrated.
