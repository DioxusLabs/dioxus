#[doc(hidden)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
#[derive(Debug, PartialEq, Clone)]
pub struct HotreloadedLiteral {
    pub name: String,
    pub value: HotReloadLiteral,
}

#[doc(hidden)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
#[derive(Debug, PartialEq, Clone)]
pub enum HotReloadLiteral {
    Fmted(FmtedSegments),
    Float(f64),
    Int(i64),
    Bool(bool),
}

#[doc(hidden)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FmtedSegments {
    pub segments: Vec<FmtSegment>,
}

impl FmtedSegments {
    pub fn new(segments: Vec<FmtSegment>) -> Self {
        Self { segments }
    }

    /// Render the formatted string by stitching together the segments
    pub fn render_with(&self, dynamic_nodes: Vec<String>) -> String {
        let mut out = String::new();

        for segment in &self.segments {
            match segment {
                FmtSegment::Literal { value } => out.push_str(value),
                FmtSegment::Dynamic { id } => out.push_str(&dynamic_nodes[*id]),
            }
        }

        out
    }

    /// Update the segments with new segments
    ///
    /// this will change how we render the formatted string
    pub fn update_segments(&mut self, new_segments: Vec<FmtSegment>) {
        self.segments = new_segments;
    }
}

#[cfg(feature = "serialize")]
use crate::nodes::deserialize_string_leaky;

#[doc(hidden)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FmtSegment {
    Literal {
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_string_leaky")
        )]
        value: &'static str,
    },
    Dynamic {
        id: usize,
    },
}
