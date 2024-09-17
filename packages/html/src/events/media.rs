use dioxus_core_types::Event;

pub type MediaEvent = Event<MediaData>;
pub struct MediaData {
    inner: Box<dyn HasMediaData>,
}

impl<E: HasMediaData> From<E> for MediaData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for MediaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaData").finish()
    }
}

impl PartialEq for MediaData {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl MediaData {
    /// Create a new MediaData
    pub fn new(inner: impl HasMediaData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of MediaData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedMediaData {}

#[cfg(feature = "serialize")]
impl From<&MediaData> for SerializedMediaData {
    fn from(_: &MediaData) -> Self {
        Self {}
    }
}

#[cfg(feature = "serialize")]
impl HasMediaData for SerializedMediaData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for MediaData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedMediaData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for MediaData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedMediaData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasMediaData: std::any::Any {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

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

    // todo: this conflicts with Media events
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
