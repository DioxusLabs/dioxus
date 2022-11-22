use dioxus_core::UiEvent;

pub type MediaEvent = UiEvent<MediaData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct MediaData {}

impl_event! [
    MediaData;

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

    // todo: this conflicts with image events
    // neither have data, so it's okay
    // ///error
    // onerror

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
