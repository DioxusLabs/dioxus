use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct MediaEvent {}

event! {
    MediaEvent: [
        ///abort
        onabort

        ///canplay
        oncanplay

        ///canplaythrough
        oncanplaythrough

        ///durationchange
        ondurationchange

        ///emptied
        onemptied

        ///encrypted
        onencrypted

        ///ended
        onended

        ///error
        onerror

        ///loadeddata
        onloadeddata

        ///loadedmetadata
        onloadedmetadata

        ///loadstart
        onloadstart

        ///pause
        onpause

        ///play
        onplay

        ///playing
        onplaying

        ///progress
        onprogress

        ///ratechange
        onratechange

        ///seeked
        onseeked

        ///seeking
        onseeking

        ///stalled
        onstalled

        ///suspend
        onsuspend

        ///timeupdate
        ontimeupdate

        ///volumechange
        onvolumechange

        ///waiting
        onwaiting
    ];
}
