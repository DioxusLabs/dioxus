use super::BuildRequest;
use super::Platform;
use crate::builder::progress::UpdateBuildProgress;
use crate::builder::progress::UpdateStage;
use crate::Result;
use crate::{
    assets::{copy_dir_to, AssetManifest},
    link::LINK_OUTPUT_ENV_VAR,
};
use crate::{builder::progress::Stage, link::InterceptedArgs};
use anyhow::Context;
use core::str;
use futures_channel::mpsc::UnboundedSender;
use manganis_core::ResourceAsset;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    env::current_exe,
    fs::{self, create_dir_all},
    io::Read,
    sync::{atomic::AtomicUsize, Arc},
};
use std::{
    io::{BufWriter, Write},
    path::Path,
};
use std::{path::PathBuf, process::Stdio};
use tokio::process::Command;
use tracing::Level;

impl BuildRequest {
    // pub fn copy_assets_dir(&self) -> anyhow::Result<()> {

    // }
}
