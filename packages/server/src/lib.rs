use dioxus::prelude::*;
use serde::{de::DeserializeOwned, Deserializer, Serialize, Serializer};

// We use deref specialization to make it possible to pass either a value that implements
pub trait SerializeToRemoteWrapper {
    fn serialize_to_remote<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
}

impl<T: Serialize> SerializeToRemoteWrapper for &T {
    fn serialize_to_remote<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> {
        self.serialize(serializer)
    }
}

impl<S: SerializeToRemote> SerializeToRemoteWrapper for &mut &S {
    fn serialize_to_remote<S2: Serializer>(
        &self,
        serializer: S2,
    ) -> Result<<S2 as Serializer>::Ok, <S2 as Serializer>::Error> {
        (**self).serialize_to_remote(serializer)
    }
}

pub trait SerializeToRemote {
    fn serialize_to_remote<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
}

impl<S: Serialize> SerializeToRemote for UseState<S> {
    fn serialize_to_remote<S2: Serializer>(
        &self,
        serializer: S2,
    ) -> Result<<S2 as Serializer>::Ok, <S2 as Serializer>::Error> {
        self.current().serialize(serializer)
    }
}

// We use deref specialization to make it possible to pass either a value that implements
pub trait DeserializeOnRemoteWrapper {
    type Output;

    fn deserialize_on_remote<'a, D: Deserializer<'a>>(
        deserializer: D,
    ) -> Result<Self::Output, D::Error>;
}

impl<T: DeserializeOwned> DeserializeOnRemoteWrapper for &T {
    type Output = T;

    fn deserialize_on_remote<'a, D: Deserializer<'a>>(
        deserializer: D,
    ) -> Result<Self::Output, D::Error> {
        T::deserialize(deserializer)
    }
}

impl<D: DeserializeOnRemote> DeserializeOnRemoteWrapper for &mut &D {
    type Output = D::Output;

    fn deserialize_on_remote<'a, D2: Deserializer<'a>>(
        deserializer: D2,
    ) -> Result<Self::Output, D2::Error> {
        D::deserialize_on_remote(deserializer)
    }
}

pub trait DeserializeOnRemote {
    type Output;

    fn deserialize_on_remote<'a, D: Deserializer<'a>>(
        deserializer: D,
    ) -> Result<Self::Output, D::Error>;
}

impl<D: DeserializeOwned> DeserializeOnRemote for UseState<D> {
    type Output = D;

    fn deserialize_on_remote<'a, D2: Deserializer<'a>>(
        deserializer: D2,
    ) -> Result<Self::Output, D2::Error> {
        D::deserialize(deserializer)
    }
}
